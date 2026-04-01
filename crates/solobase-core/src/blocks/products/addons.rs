//! Recurring add-on management via Stripe subscription items.
//!
//! Add-ons are recurring line items on the user's existing Stripe subscription.
//! They persist until explicitly cancelled. The `subscriptions.addon_*` columns
//! reflect the currently active add-on totals.

use crate::plans;
use std::collections::HashMap;
use wafer_core::clients::{config, database as db, network};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

/// GET /b/products/addons — list available add-on packs with prices.
pub async fn handle_list(_ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let packs: Vec<serde_json::Value> = plans::ADDON_PACKS
        .iter()
        .map(|a| {
            serde_json::json!({
                "id": a.id,
                "name": a.name,
                "price_cents": a.price_cents,
                "extra_requests": a.extra_requests,
                "extra_r2_bytes": a.extra_r2_bytes,
                "extra_d1_bytes": a.extra_d1_bytes,
                "extra_projects": a.extra_projects,
            })
        })
        .collect();
    json_respond(msg, &packs)
}

/// POST /b/products/addons/subscribe — add a recurring add-on to the user's subscription.
///
/// Adds a new subscription item to their existing Stripe subscription.
pub async fn handle_subscribe(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return err_unauthorized(msg, "Not authenticated");
    }

    #[derive(serde::Deserialize)]
    struct Req {
        addon_id: String,
    }

    let body: Req = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };

    let addon = match plans::get_addon(&body.addon_id) {
        Some(a) => a,
        None => return err_bad_request(msg, "Unknown add-on pack"),
    };

    // Get user's Stripe subscription ID
    let sub = get_subscription(ctx, &user_id).await;
    let stripe_sub_id = match &sub {
        Some(s) => s
            .get("stripe_subscription_id")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        None => "",
    };

    if stripe_sub_id.is_empty() {
        return err_bad_request(msg, "No active subscription. Subscribe to a plan first.");
    }

    // Get the Stripe Price ID from config
    let stripe_price_id = config::get_default(ctx, addon.stripe_price_env, "").await;
    if stripe_price_id.is_empty() {
        return err_internal(
            msg,
            &format!(
                "Add-on not configured: {} env var is not set",
                addon.stripe_price_env
            ),
        );
    }

    let stripe_key = match config::get(ctx, "STRIPE_SECRET_KEY").await {
        Ok(k) => k,
        Err(_) => return err_internal(msg, "Stripe is not configured"),
    };

    // Add subscription item via Stripe API
    let stripe_body = format!(
        "subscription={}&price={}&metadata[addon_id]={}",
        super::stripe::urlencoding(stripe_sub_id),
        super::stripe::urlencoding(&stripe_price_id),
        addon.id,
    );

    let mut headers = HashMap::new();
    headers.insert(
        "Authorization".to_string(),
        format!("Bearer {}", stripe_key),
    );
    headers.insert(
        "Content-Type".to_string(),
        "application/x-www-form-urlencoded".to_string(),
    );

    let stripe_api_url = config::get_default(ctx, "STRIPE_API_URL", "https://api.stripe.com").await;
    let url = format!("{}/v1/subscription_items", stripe_api_url);

    let resp =
        match network::do_request(ctx, "POST", &url, &headers, Some(&stripe_body.into_bytes()))
            .await
        {
            Ok(r) => r,
            Err(e) => return err_internal(msg, &format!("Stripe API error: {e}")),
        };

    if resp.status_code >= 400 {
        let err_body = String::from_utf8_lossy(&resp.body);
        return err_internal(
            msg,
            &format!("Stripe error ({}): {}", resp.status_code, err_body),
        );
    }

    let item: serde_json::Value = match serde_json::from_slice(&resp.body) {
        Ok(d) => d,
        Err(_) => return err_internal(msg, "Failed to parse Stripe response"),
    };

    // Update addon columns on subscriptions table
    apply_addon_change(ctx, &user_id, addon, true).await;

    json_respond(
        msg,
        &serde_json::json!({
            "subscription_item_id": item.get("id").and_then(|v| v.as_str()).unwrap_or(""),
            "addon_id": addon.id,
            "status": "active"
        }),
    )
}

/// POST /b/products/addons/cancel — remove a recurring add-on.
///
/// Deletes the subscription item from Stripe and decrements addon columns.
pub async fn handle_cancel(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return err_unauthorized(msg, "Not authenticated");
    }

    #[derive(serde::Deserialize)]
    struct Req {
        addon_id: String,
        /// The Stripe subscription_item ID to cancel.
        subscription_item_id: String,
    }

    let body: Req = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };

    let addon = match plans::get_addon(&body.addon_id) {
        Some(a) => a,
        None => return err_bad_request(msg, "Unknown add-on pack"),
    };

    let stripe_key = match config::get(ctx, "STRIPE_SECRET_KEY").await {
        Ok(k) => k,
        Err(_) => return err_internal(msg, "Stripe is not configured"),
    };

    // Validate subscription_item_id format (must be si_* — prevent path traversal/SSRF)
    if !body.subscription_item_id.starts_with("si_")
        || body.subscription_item_id.contains('/')
        || body.subscription_item_id.contains("..")
    {
        return err_bad_request(msg, "Invalid subscription item ID format");
    }

    // Verify the subscription item belongs to this user's subscription
    let sub = get_subscription(ctx, &user_id).await;
    let stripe_sub_id = match &sub {
        Some(s) => s
            .get("stripe_subscription_id")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        None => "",
    };
    if stripe_sub_id.is_empty() {
        return err_bad_request(msg, "No active subscription found");
    }

    // Delete subscription item via Stripe API
    let mut headers = HashMap::new();
    headers.insert(
        "Authorization".to_string(),
        format!("Bearer {}", stripe_key),
    );

    let stripe_api_url = config::get_default(ctx, "STRIPE_API_URL", "https://api.stripe.com").await;
    let url = format!(
        "{}/v1/subscription_items/{}",
        stripe_api_url, body.subscription_item_id
    );

    let resp = match network::do_request(ctx, "DELETE", &url, &headers, None).await {
        Ok(r) => r,
        Err(e) => return err_internal(msg, &format!("Stripe API error: {e}")),
    };

    if resp.status_code >= 400 {
        let err_body = String::from_utf8_lossy(&resp.body);
        return err_internal(
            msg,
            &format!("Stripe error ({}): {}", resp.status_code, err_body),
        );
    }

    // Decrement addon columns
    apply_addon_change(ctx, &user_id, addon, false).await;

    json_respond(
        msg,
        &serde_json::json!({
            "addon_id": addon.id,
            "status": "cancelled"
        }),
    )
}

/// Update addon columns on the subscriptions table.
/// `add = true` increments, `add = false` decrements (clamped to 0).
async fn apply_addon_change(ctx: &dyn Context, user_id: &str, addon: &plans::AddonPack, add: bool) {
    let (op, clamp) = if add { ("+", "") } else { ("-", ", 0)") };
    let open = if add { "" } else { "MAX(" };

    let sql = format!(
        "UPDATE subscriptions SET \
           addon_projects = {open}COALESCE(addon_projects, 0) {op} ?1{clamp}, \
           addon_requests = {open}COALESCE(addon_requests, 0) {op} ?2{clamp}, \
           addon_r2_bytes = {open}COALESCE(addon_r2_bytes, 0) {op} ?3{clamp}, \
           addon_d1_bytes = {open}COALESCE(addon_d1_bytes, 0) {op} ?4{clamp}, \
           updated_at = ?5 \
         WHERE user_id = ?6 AND status = 'active'"
    );

    let now = chrono::Utc::now().to_rfc3339();
    let result = db::exec_raw(
        ctx,
        &sql,
        &[
            serde_json::json!(addon.extra_projects),
            serde_json::json!(addon.extra_requests),
            serde_json::json!(addon.extra_r2_bytes),
            serde_json::json!(addon.extra_d1_bytes),
            serde_json::Value::String(now),
            serde_json::Value::String(user_id.to_string()),
        ],
    )
    .await;

    match result {
        Ok(_) => tracing::info!(
            user_id = user_id,
            addon_id = addon.id,
            add = add,
            "Addon change applied"
        ),
        Err(e) => tracing::warn!(
            user_id = user_id,
            addon_id = addon.id,
            "Addon change failed: {e}"
        ),
    }
}

/// Sync addon columns from Stripe subscription items.
///
/// Called from the webhook handler on `customer.subscription.updated`.
/// Recalculates addon totals from the subscription's active items.
pub async fn sync_addons_from_stripe(ctx: &dyn Context, user_id: &str, items: &serde_json::Value) {
    let mut total_projects: usize = 0;
    let mut total_requests: u64 = 0;
    let mut total_r2: u64 = 0;
    let mut total_d1: u64 = 0;

    // items.data is an array of subscription items
    if let Some(data) = items.get("data").and_then(|v| v.as_array()) {
        for item in data {
            let addon_id = item
                .pointer("/metadata/addon_id")
                .or_else(|| item.pointer("/price/metadata/addon_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if let Some(addon) = plans::get_addon(addon_id) {
                let qty = item.get("quantity").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                total_projects += addon.extra_projects * qty;
                total_requests += addon.extra_requests * qty as u64;
                total_r2 += addon.extra_r2_bytes * qty as u64;
                total_d1 += addon.extra_d1_bytes * qty as u64;
            }
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    let result = db::exec_raw(
        ctx,
        "UPDATE subscriptions SET \
           addon_projects = ?1, addon_requests = ?2, \
           addon_r2_bytes = ?3, addon_d1_bytes = ?4, \
           updated_at = ?5 \
         WHERE user_id = ?6 AND status = 'active'",
        &[
            serde_json::json!(total_projects),
            serde_json::json!(total_requests),
            serde_json::json!(total_r2),
            serde_json::json!(total_d1),
            serde_json::Value::String(now),
            serde_json::Value::String(user_id.to_string()),
        ],
    )
    .await;

    match result {
        Ok(_) => tracing::info!(
            user_id = user_id,
            projects = total_projects,
            requests = total_requests,
            "Addon sync complete"
        ),
        Err(e) => tracing::warn!(user_id = user_id, "Addon sync failed: {e}"),
    }
}

/// Helper: get user's subscription record.
async fn get_subscription(
    ctx: &dyn Context,
    user_id: &str,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    let rows = db::query_raw(
        ctx,
        "SELECT stripe_subscription_id, plan, status FROM subscriptions WHERE user_id = ?1 AND status = 'active'",
        &[serde_json::Value::String(user_id.to_string())],
    ).await.ok()?;

    if rows.is_empty() {
        return None;
    }

    let data = &rows[0].data;
    let map: serde_json::Map<String, serde_json::Value> =
        data.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    Some(map)
}
