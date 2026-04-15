use super::{PURCHASES_COLLECTION, SUBSCRIPTIONS};
use crate::blocks::helpers::hex_encode;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use wafer_core::clients::{config, database as db, network};
use wafer_core::interfaces::database::service::{Filter, FilterOp, ListOptions};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::{InputStream, OutputStream};
use wafer_sql_utils::value::sea_values_to_json;
use wafer_sql_utils::Backend;

use crate::blocks::helpers::{
    err_bad_request, err_forbidden, err_internal, err_not_found, err_unauthorized, ok_json,
};

pub async fn handle_checkout(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    let stripe_key = match config::get(ctx, "SUPPERS_AI__PRODUCTS__STRIPE_SECRET_KEY").await {
        Ok(k) => k,
        Err(_) => return err_internal("Stripe is not configured"),
    };

    #[derive(serde::Deserialize)]
    struct CheckoutReq {
        purchase_id: String,
        success_url: Option<String>,
        cancel_url: Option<String>,
    }
    let raw = input.collect_to_bytes().await;
    let body: CheckoutReq = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    // Get purchase and verify ownership
    let purchase = match db::get(ctx, PURCHASES_COLLECTION, &body.purchase_id).await {
        Ok(p) => p,
        Err(_) => return err_not_found("Purchase not found"),
    };
    let purchase_user = purchase
        .data
        .get("user_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if purchase_user != msg.user_id() {
        return err_forbidden("Cannot checkout another user's purchase");
    }

    let total_cents = purchase
        .data
        .get("total_cents")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if total_cents <= 0 {
        return err_bad_request("Purchase total must be positive");
    }

    // Check product dependency (requires field)
    let line_items = db::query_raw(
        ctx,
        &format!(
            "SELECT product_id FROM {} WHERE purchase_id = ?1 LIMIT 1",
            super::LINE_ITEMS_COLLECTION
        ),
        &[serde_json::Value::String(body.purchase_id.clone())],
    )
    .await;

    if let Ok(items) = &line_items {
        for item in items {
            let product_id = item.data.get("product_id").and_then(|v| v.as_str()).unwrap_or("");
            if product_id.is_empty() {
                continue;
            }
            if let Ok(product) = db::get(ctx, super::PRODUCTS_COLLECTION, product_id).await {
                let requires = product.data.get("requires").and_then(|v| v.as_str()).unwrap_or("");
                if !requires.is_empty() {
                    let has_required = user_owns_product(ctx, purchase_user, requires).await;
                    if !has_required {
                        return err_bad_request(
                            "You must own the required product before purchasing this item.",
                        );
                    }
                }
            }
        }
    }

    // Atomic status transition: pending -> checkout_started (prevents double-checkout race)
    let (sql, vals) = wafer_sql_utils::query::build_update_where(
        PURCHASES_COLLECTION,
        &[
            ("status".to_string(), serde_json::json!("checkout_started")),
            ("updated_at".to_string(), serde_json::json!(chrono::Utc::now().to_rfc3339())),
        ],
        &[
            Filter { field: "id".into(), operator: FilterOp::Equal, value: serde_json::json!(body.purchase_id) },
            Filter { field: "status".into(), operator: FilterOp::Equal, value: serde_json::json!("pending") },
        ],
        Backend::Sqlite,
    );
    let args = sea_values_to_json(vals);
    let rows = db::exec_raw(ctx, &sql, &args).await.unwrap_or(0);
    if rows == 0 {
        return err_bad_request(
            "Purchase is not in pending state or is already being processed",
        );
    }

    let currency = purchase
        .data
        .get("currency")
        .and_then(|v| v.as_str())
        .unwrap_or("usd")
        .to_lowercase();

    let base_url = config::get_default(
        ctx,
        "SOLOBASE_SHARED__FRONTEND_URL",
        "http://localhost:5173",
    )
    .await;
    let success_url = body.success_url.unwrap_or_else(|| {
        format!(
            "{}/checkout/success?session_id={{CHECKOUT_SESSION_ID}}",
            base_url
        )
    });
    let cancel_url = body
        .cancel_url
        .unwrap_or_else(|| format!("{}/checkout/cancel", base_url));

    // Create Stripe checkout session
    let stripe_body = format!(
        "payment_method_types[]=card&line_items[0][price_data][currency]={}&line_items[0][price_data][unit_amount]={}&line_items[0][price_data][product_data][name]=Order {}&line_items[0][quantity]=1&mode=payment&success_url={}&cancel_url={}&metadata[purchase_id]={}",
        currency,
        total_cents,
        body.purchase_id,
        urlencoding(&success_url),
        urlencoding(&cancel_url),
        body.purchase_id
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

    let stripe_api_url = config::get_default(
        ctx,
        "SUPPERS_AI__PRODUCTS__STRIPE_API_URL",
        "https://api.stripe.com",
    )
    .await;
    let checkout_url_endpoint = format!("{}/v1/checkout/sessions", stripe_api_url);

    let resp = match network::do_request(
        ctx,
        "POST",
        &checkout_url_endpoint,
        &headers,
        Some(&stripe_body.into_bytes()),
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return err_internal(&format!("Stripe API error: {e}")),
    };

    if resp.status_code >= 400 {
        // Revert status back to pending so user can retry
        let (revert_sql, revert_vals) = wafer_sql_utils::query::build_update_where(
            PURCHASES_COLLECTION,
            &[
                ("status".to_string(), serde_json::json!("pending")),
                ("updated_at".to_string(), serde_json::json!(chrono::Utc::now().to_rfc3339())),
            ],
            &[
                Filter { field: "id".into(), operator: FilterOp::Equal, value: serde_json::json!(body.purchase_id) },
                Filter { field: "status".into(), operator: FilterOp::Equal, value: serde_json::json!("checkout_started") },
            ],
            Backend::Sqlite,
        );
        let revert_args = sea_values_to_json(revert_vals);
        let _ = db::exec_raw(ctx, &revert_sql, &revert_args).await;
        let err_body = String::from_utf8_lossy(&resp.body);
        return err_internal(&format!("Stripe error ({}): {}", resp.status_code, err_body));
    }

    let session: serde_json::Value = match serde_json::from_slice(&resp.body) {
        Ok(d) => d,
        Err(_) => return err_internal("Failed to parse Stripe response"),
    };

    let session_id = session.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let checkout_url = session.get("url").and_then(|v| v.as_str()).unwrap_or("");

    // Update purchase with Stripe session ID
    let mut upd = HashMap::new();
    upd.insert(
        "provider".to_string(),
        serde_json::Value::String("stripe".to_string()),
    );
    upd.insert(
        "provider_session_id".to_string(),
        serde_json::Value::String(session_id.to_string()),
    );
    upd.insert(
        "updated_at".to_string(),
        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
    );
    if let Err(e) = db::update(ctx, PURCHASES_COLLECTION, &body.purchase_id, upd).await {
        tracing::warn!("Failed to update purchase with Stripe session ID: {e}");
    }

    ok_json(&serde_json::json!({
        "session_id": session_id,
        "checkout_url": checkout_url
    }))
}

pub async fn handle_webhook(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    // Verify Stripe webhook signature - REQUIRED
    let webhook_secret =
        config::get_default(ctx, "SUPPERS_AI__PRODUCTS__STRIPE_WEBHOOK_SECRET", "").await;
    if webhook_secret.is_empty() {
        return err_internal(
            "STRIPE_WEBHOOK_SECRET not configured — webhook processing disabled for security",
        );
    }
    let sig_header = msg.header("stripe-signature").to_string();
    if sig_header.is_empty() {
        return err_unauthorized("Missing Stripe-Signature header");
    }
    let raw_body = input.collect_to_bytes().await;
    if !verify_stripe_signature(&raw_body, &sig_header, &webhook_secret) {
        return err_unauthorized("Invalid webhook signature");
    }

    // Parse webhook event
    let event: serde_json::Value = match serde_json::from_slice(&raw_body) {
        Ok(e) => e,
        Err(e) => return err_bad_request(&format!("Invalid webhook body: {e}")),
    };

    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");

    let data_object = event
        .get("data")
        .and_then(|d| d.get("object"))
        .cloned()
        .unwrap_or_default();

    match event_type {
        "checkout.session.completed" => {
            // Handle product purchase completion
            let purchase_id = data_object
                .pointer("/metadata/purchase_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if !purchase_id.is_empty() {
                let payment_intent = data_object
                    .get("payment_intent")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // Atomic: only complete if still in checkout_started (or pending for backwards compat)
                let now = chrono::Utc::now().to_rfc3339();
                let (sql, vals) = wafer_sql_utils::query::build_update_where(
                    PURCHASES_COLLECTION,
                    &[
                        ("status".to_string(), serde_json::json!("completed")),
                        ("provider_payment_intent_id".to_string(), serde_json::json!(payment_intent)),
                        ("approved_at".to_string(), serde_json::json!(&now)),
                        ("updated_at".to_string(), serde_json::json!(&now)),
                    ],
                    &[
                        Filter { field: "id".into(), operator: FilterOp::Equal, value: serde_json::json!(purchase_id) },
                        Filter { field: "status".into(), operator: FilterOp::In, value: serde_json::json!(["checkout_started", "pending"]) },
                    ],
                    Backend::Sqlite,
                );
                let args = sea_values_to_json(vals);
                let rows = db::exec_raw(ctx, &sql, &args).await.unwrap_or(0);
                if rows == 0 {
                    tracing::warn!(
                        "Purchase {} not updated — already completed or refunded",
                        purchase_id
                    );
                }
            }

            // Handle subscription creation (platform billing)
            let user_id = data_object
                .pointer("/metadata/user_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let plan = data_object
                .pointer("/metadata/plan")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let stripe_customer_id = data_object
                .get("customer")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let stripe_sub_id = data_object
                .get("subscription")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if !user_id.is_empty() && !plan.is_empty() {
                let now = chrono::Utc::now().to_rfc3339();
                let sub_id = format!("sub_{}_{}", user_id, chrono::Utc::now().timestamp_millis());

                let (sql, vals) = wafer_sql_utils::upsert::build_upsert(
                    SUBSCRIPTIONS,
                    &[
                        ("id".to_string(), serde_json::json!(sub_id)),
                        ("user_id".to_string(), serde_json::json!(user_id)),
                        ("stripe_customer_id".to_string(), serde_json::json!(stripe_customer_id)),
                        ("stripe_subscription_id".to_string(), serde_json::json!(stripe_sub_id)),
                        ("plan".to_string(), serde_json::json!(plan)),
                        ("status".to_string(), serde_json::json!("active")),
                        ("created_at".to_string(), serde_json::json!(&now)),
                        ("updated_at".to_string(), serde_json::json!(&now)),
                    ],
                    &["user_id"],
                    &["stripe_customer_id", "stripe_subscription_id", "plan", "status", "updated_at"],
                    Backend::Sqlite,
                );
                let args = sea_values_to_json(vals);
                let _ = db::exec_raw(ctx, &sql, &args).await;

                fire_products_webhook(
                    ctx,
                    "products.checkout.completed",
                    &serde_json::json!({
                        "user_id": user_id, "plan": plan
                    }),
                )
                .await;
            }
        }

        "customer.subscription.updated" => {
            let stripe_sub_id = data_object.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let status = data_object
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let plan = data_object
                .pointer("/items/data/0/price/lookup_key")
                .or_else(|| data_object.pointer("/items/data/0/price/metadata/plan"))
                .and_then(|v| v.as_str());
            let now = chrono::Utc::now().to_rfc3339();

            {
                let sub_filter = vec![
                    Filter { field: "stripe_subscription_id".into(), operator: FilterOp::Equal, value: serde_json::json!(stripe_sub_id) },
                ];
                let mut data: Vec<(String, serde_json::Value)> = vec![
                    ("status".to_string(), serde_json::json!(status)),
                    ("updated_at".to_string(), serde_json::json!(&now)),
                ];
                if let Some(plan) = plan {
                    data.push(("plan".to_string(), serde_json::json!(plan)));
                }
                let (sql, vals) = wafer_sql_utils::query::build_update_where(
                    SUBSCRIPTIONS,
                    &data,
                    &sub_filter,
                    Backend::Sqlite,
                );
                let args = sea_values_to_json(vals);
                let _ = db::exec_raw(ctx, &sql, &args).await;
            }

            // Sync addon totals from Stripe subscription items metadata.
            // Each addon subscription item has metadata fields: extra_projects,
            // extra_requests, extra_r2_bytes, extra_d1_bytes (set when creating
            // the subscription item via Stripe API).
            let user_id = get_user_for_stripe_sub(ctx, stripe_sub_id).await;
            if let Some(ref uid) = user_id {
                if let Some(items) = data_object.get("items") {
                    sync_addon_totals_from_items(ctx, uid, items).await;
                }
            }

            // Notify control plane
            if let Some(uid) = user_id {
                fire_products_webhook(
                    ctx,
                    "products.subscription.updated",
                    &serde_json::json!({
                        "user_id": uid, "plan": plan.unwrap_or("free")
                    }),
                )
                .await;
            }
        }

        "invoice.payment_failed" => {
            let stripe_sub_id = data_object
                .get("subscription")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !stripe_sub_id.is_empty() {
                let now = chrono::Utc::now().to_rfc3339();
                let grace_end = (chrono::Utc::now() + chrono::Duration::days(7)).to_rfc3339();
                let (sql, vals) = wafer_sql_utils::query::build_update_where(
                    SUBSCRIPTIONS,
                    &[
                        ("status".to_string(), serde_json::json!("past_due")),
                        ("grace_period_end".to_string(), serde_json::json!(&grace_end)),
                        ("updated_at".to_string(), serde_json::json!(&now)),
                    ],
                    &[
                        Filter { field: "stripe_subscription_id".into(), operator: FilterOp::Equal, value: serde_json::json!(stripe_sub_id) },
                    ],
                    Backend::Sqlite,
                );
                let args = sea_values_to_json(vals);
                let _ = db::exec_raw(ctx, &sql, &args).await;
            }
        }

        "customer.subscription.deleted" => {
            let stripe_sub_id = data_object.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let now = chrono::Utc::now().to_rfc3339();

            let user_id = get_user_for_stripe_sub(ctx, stripe_sub_id).await;

            // Cancel subscription and reset all addon columns to 0
            let (sql, vals) = wafer_sql_utils::query::build_update_where(
                SUBSCRIPTIONS,
                &[
                    ("status".to_string(), serde_json::json!("cancelled")),
                    ("addon_projects".to_string(), serde_json::json!(0)),
                    ("addon_requests".to_string(), serde_json::json!(0)),
                    ("addon_r2_bytes".to_string(), serde_json::json!(0)),
                    ("addon_d1_bytes".to_string(), serde_json::json!(0)),
                    ("updated_at".to_string(), serde_json::json!(&now)),
                ],
                &[
                    Filter { field: "stripe_subscription_id".into(), operator: FilterOp::Equal, value: serde_json::json!(stripe_sub_id) },
                ],
                Backend::Sqlite,
            );
            let args = sea_values_to_json(vals);
            let _ = db::exec_raw(ctx, &sql, &args).await;

            if let Some(uid) = user_id {
                fire_products_webhook(
                    ctx,
                    "products.subscription.deleted",
                    &serde_json::json!({
                        "user_id": uid
                    }),
                )
                .await;
            }
        }

        "charge.refunded" => {
            let payment_intent = data_object
                .get("payment_intent")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if !payment_intent.is_empty() {
                if let Ok(purchase) = db::get_by_field(
                    ctx,
                    PURCHASES_COLLECTION,
                    "provider_payment_intent_id",
                    serde_json::Value::String(payment_intent),
                )
                .await
                {
                    let mut data = HashMap::new();
                    data.insert(
                        "status".to_string(),
                        serde_json::Value::String("refunded".to_string()),
                    );
                    data.insert(
                        "refunded_at".to_string(),
                        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                    );
                    data.insert(
                        "updated_at".to_string(),
                        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                    );
                    if let Err(e) = db::update(ctx, PURCHASES_COLLECTION, &purchase.id, data).await
                    {
                        tracing::error!("Failed to mark purchase as refunded: {e}");
                        return err_internal(&format!("Failed to update purchase: {e}"));
                    }
                }
            }
        }

        _ => {
            // Ignore unhandled event types
        }
    }

    ok_json(&serde_json::json!({"received": true}))
}

async fn get_user_for_stripe_sub(ctx: &dyn Context, stripe_sub_id: &str) -> Option<String> {
    let opts = ListOptions {
        filters: vec![
            Filter { field: "stripe_subscription_id".into(), operator: FilterOp::Equal, value: serde_json::json!(stripe_sub_id) },
        ],
        limit: 1,
        ..Default::default()
    };
    let (sql, vals) = wafer_sql_utils::query::build_select_columns(
        SUBSCRIPTIONS,
        &["user_id"],
        &opts,
        None,
        Backend::Sqlite,
    );
    let args = sea_values_to_json(vals);
    let rows = db::query_raw(ctx, &sql, &args).await.ok()?;
    rows.first()?
        .data
        .get("user_id")
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Fire a webhook for product/billing events.
/// Best-effort — if PRODUCTS_WEBHOOK_URL is not configured, this is a no-op.
/// The webhook is signed with HMAC-SHA256 using PRODUCTS_WEBHOOK_SECRET.
async fn fire_products_webhook(ctx: &dyn Context, event: &str, data: &serde_json::Value) {
    let url = config::get_default(ctx, "SUPPERS_AI__PRODUCTS__WEBHOOK_URL", "").await;
    let secret = config::get_default(ctx, "SUPPERS_AI__PRODUCTS__WEBHOOK_SECRET", "").await;
    if url.is_empty() {
        return;
    }

    let body = serde_json::json!({
        "event": event,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "data": data
    });
    let payload = serde_json::to_vec(&body).unwrap_or_default();

    // Sign with HMAC-SHA256 (same pattern as Stripe webhook verification)
    let signature = if !secret.is_empty() {
        let sig = hmac_sha256(secret.as_bytes(), &payload);
        format!("sha256={}", hex_encode(&sig))
    } else {
        String::new()
    };

    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    if !signature.is_empty() {
        headers.insert("X-Webhook-Signature".to_string(), signature);
    }

    match network::do_request(ctx, "POST", &url, &headers, Some(&payload)).await {
        Ok(resp) if resp.status_code < 400 => {
            tracing::info!(event = event, "products webhook delivered");
        }
        Ok(resp) => {
            tracing::warn!(
                event = event,
                status = resp.status_code,
                "products webhook failed"
            );
        }
        Err(e) => {
            tracing::warn!(event = event, error = %e, "products webhook delivery error");
        }
    }
}

pub(super) fn urlencoding(s: &str) -> String {
    // Iterate over bytes (not chars) to correctly handle multi-byte UTF-8
    s.as_bytes()
        .iter()
        .map(|&b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                String::from(b as char)
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

/// Verify Stripe webhook signature using HMAC-SHA256.
/// Stripe sends `t=timestamp,v1=signature` in the Stripe-Signature header.
fn verify_stripe_signature(payload: &[u8], sig_header: &str, secret: &str) -> bool {
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut timestamp = "";
    let mut expected_sig = "";

    for part in sig_header.split(',') {
        let part = part.trim();
        if let Some(t) = part.strip_prefix("t=") {
            timestamp = t;
        } else if let Some(v) = part.strip_prefix("v1=") {
            expected_sig = v;
        }
    }

    if timestamp.is_empty() || expected_sig.is_empty() {
        return false;
    }

    // Reject events with timestamps older than 5 minutes (replay protection)
    if let Ok(ts) = timestamp.parse::<u64>() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if now.abs_diff(ts) > 300 {
            return false;
        }
    } else {
        return false;
    }

    // Compute expected signature: HMAC-SHA256(secret, "timestamp.payload")
    let signed_payload = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));

    // Use ring or hmac crate if available; fallback to manual HMAC-SHA256
    // For now, use the crypto service pattern — but since we don't have it here,
    // implement a constant-time comparison with the sha2/hmac approach
    let computed = hmac_sha256(secret.as_bytes(), signed_payload.as_bytes());
    let computed_hex = hex_encode(&computed);

    // Constant-time comparison
    constant_time_eq(computed_hex.as_bytes(), expected_sig.as_bytes())
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key");
    mac.update(data);
    mac.finalize().into_bytes().into()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Check if a user owns a product — either via an active subscription that
/// references it, or a completed purchase containing it as a line item.
async fn user_owns_product(ctx: &dyn Context, user_id: &str, product_id: &str) -> bool {
    // Check subscriptions: the plan field may reference the product
    let sub_rows = db::query_raw(
        ctx,
        &format!(
            "SELECT 1 FROM {} WHERE user_id = ?1 AND status = 'active' AND plan = ?2 LIMIT 1",
            SUBSCRIPTIONS
        ),
        &[
            serde_json::Value::String(user_id.to_string()),
            serde_json::Value::String(product_id.to_string()),
        ],
    )
    .await;

    if matches!(&sub_rows, Ok(rows) if !rows.is_empty()) {
        return true;
    }

    // Check completed purchases containing this product as a line item
    let purchase_rows = db::query_raw(
        ctx,
        &format!(
            "SELECT 1 FROM {} p JOIN {} li ON li.purchase_id = p.id \
             WHERE p.user_id = ?1 AND p.status = 'completed' AND li.product_id = ?2 LIMIT 1",
            super::PURCHASES_COLLECTION,
            super::LINE_ITEMS_COLLECTION,
        ),
        &[
            serde_json::Value::String(user_id.to_string()),
            serde_json::Value::String(product_id.to_string()),
        ],
    )
    .await;

    matches!(&purchase_rows, Ok(rows) if !rows.is_empty())
}

/// Sync addon column totals from Stripe subscription items.
///
/// Reads addon values from item metadata (set by the platform when creating
/// subscription items). This keeps the products block plan-agnostic — it
/// doesn't need to know what addon packs exist, just what Stripe reports.
async fn sync_addon_totals_from_items(ctx: &dyn Context, user_id: &str, items: &serde_json::Value) {
    let mut total_projects: i64 = 0;
    let mut total_requests: i64 = 0;
    let mut total_r2: i64 = 0;
    let mut total_d1: i64 = 0;

    if let Some(data) = items.get("data").and_then(|v| v.as_array()) {
        for item in data {
            let meta = item.get("metadata")
                .or_else(|| item.pointer("/price/metadata"));
            let meta = match meta {
                Some(m) => m,
                None => continue,
            };

            // Skip non-addon items (the base plan item won't have addon_id)
            if meta.get("addon_id").is_none() {
                continue;
            }

            let qty = item.get("quantity").and_then(|v| v.as_i64()).unwrap_or(1);
            let parse = |key: &str| -> i64 {
                meta.get(key)
                    .and_then(|v| v.as_str().and_then(|s| s.parse().ok()).or_else(|| v.as_i64()))
                    .unwrap_or(0)
            };
            total_projects += parse("extra_projects") * qty;
            total_requests += parse("extra_requests") * qty;
            total_r2 += parse("extra_r2_bytes") * qty;
            total_d1 += parse("extra_d1_bytes") * qty;
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    let sql = format!(
        "UPDATE {SUBSCRIPTIONS} SET \
           addon_projects = ?1, addon_requests = ?2, \
           addon_r2_bytes = ?3, addon_d1_bytes = ?4, \
           updated_at = ?5 \
         WHERE user_id = ?6 AND status = 'active'"
    );
    let _ = db::exec_raw(
        ctx,
        &sql,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_eq_equal() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn test_constant_time_eq_not_equal() {
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hell"));
        assert!(!constant_time_eq(b"a", b"b"));
    }

    #[test]
    fn test_constant_time_eq_different_lengths() {
        assert!(!constant_time_eq(b"short", b"longer"));
        assert!(!constant_time_eq(b"", b"x"));
    }

    #[test]
    fn test_hmac_sha256_deterministic() {
        let hash1 = hmac_sha256(b"secret", b"payload");
        let hash2 = hmac_sha256(b"secret", b"payload");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hmac_sha256_different_keys() {
        let hash1 = hmac_sha256(b"key1", b"data");
        let hash2 = hmac_sha256(b"key2", b"data");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hmac_sha256_different_data() {
        let hash1 = hmac_sha256(b"key", b"data1");
        let hash2 = hmac_sha256(b"key", b"data2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_stripe_signature_valid() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let secret = "whsec_test_secret";
        let payload = b"{\"type\":\"checkout.session.completed\"}";
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let signed_payload = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));
        let computed = hmac_sha256(secret.as_bytes(), signed_payload.as_bytes());
        let computed_hex = hex_encode(&computed);

        let sig_header = format!("t={},v1={}", timestamp, computed_hex);

        assert!(verify_stripe_signature(payload, &sig_header, secret));
    }

    #[test]
    fn test_verify_stripe_signature_invalid_sig() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let sig_header = format!(
            "t={},v1=0000000000000000000000000000000000000000000000000000000000000000",
            timestamp
        );

        assert!(!verify_stripe_signature(b"payload", &sig_header, "secret"));
    }

    #[test]
    fn test_verify_stripe_signature_expired() {
        let secret = "whsec_test";
        let payload = b"data";
        let old_timestamp = 1000000; // way in the past

        let signed_payload = format!("{}.{}", old_timestamp, String::from_utf8_lossy(payload));
        let computed = hmac_sha256(secret.as_bytes(), signed_payload.as_bytes());
        let computed_hex = hex_encode(&computed);

        let sig_header = format!("t={},v1={}", old_timestamp, computed_hex);

        assert!(!verify_stripe_signature(payload, &sig_header, secret));
    }

    #[test]
    fn test_verify_stripe_signature_missing_parts() {
        assert!(!verify_stripe_signature(b"data", "", "secret"));
        assert!(!verify_stripe_signature(b"data", "t=123", "secret"));
        assert!(!verify_stripe_signature(b"data", "v1=abc", "secret"));
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding("hello"), "hello");
        assert_eq!(urlencoding("hello world"), "hello%20world");
        assert_eq!(urlencoding("a+b=c&d"), "a%2Bb%3Dc%26d");
        assert_eq!(
            urlencoding("https://example.com"),
            "https%3A%2F%2Fexample.com"
        );
    }
}
