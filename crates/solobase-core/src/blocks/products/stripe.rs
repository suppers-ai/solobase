use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::{config, database as db, network};
use super::PURCHASES_COLLECTION;
use crate::blocks::helpers::hex_encode;
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub async fn handle_checkout(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let stripe_key = match config::get(ctx, "STRIPE_SECRET_KEY").await {
        Ok(k) => k,
        Err(_) => return err_internal(msg, "Stripe is not configured"),
    };

    #[derive(serde::Deserialize)]
    struct CheckoutReq {
        purchase_id: String,
        success_url: Option<String>,
        cancel_url: Option<String>,
    }
    let body: CheckoutReq = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };

    // Get purchase
    let purchase = match db::get(ctx, PURCHASES_COLLECTION, &body.purchase_id).await {
        Ok(p) => p,
        Err(_) => return err_not_found(msg, "Purchase not found"),
    };

    let total_cents = purchase.data.get("total_cents").and_then(|v| v.as_i64()).unwrap_or(0);
    let currency = purchase.data.get("currency").and_then(|v| v.as_str()).unwrap_or("usd").to_lowercase();

    let base_url = config::get_default(ctx, "FRONTEND_URL", "http://localhost:5173").await;
    let success_url = body.success_url.unwrap_or_else(|| format!("{}/checkout/success?session_id={{CHECKOUT_SESSION_ID}}", base_url));
    let cancel_url = body.cancel_url.unwrap_or_else(|| format!("{}/checkout/cancel", base_url));

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
    headers.insert("Authorization".to_string(), format!("Bearer {}", stripe_key));
    headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());

    let stripe_api_url = config::get_default(ctx, "STRIPE_API_URL", "https://api.stripe.com").await;
    let checkout_url_endpoint = format!("{}/v1/checkout/sessions", stripe_api_url);

    let resp = match network::do_request(
        ctx,
        "POST",
        &checkout_url_endpoint,
        &headers,
        Some(&stripe_body.into_bytes()),
    ).await {
        Ok(r) => r,
        Err(e) => return err_internal(msg, &format!("Stripe API error: {e}")),
    };

    if resp.status_code >= 400 {
        let err_body = String::from_utf8_lossy(&resp.body);
        return err_internal(msg, &format!("Stripe error ({}): {}", resp.status_code, err_body));
    }

    let session: serde_json::Value = match serde_json::from_slice(&resp.body) {
        Ok(d) => d,
        Err(_) => return err_internal(msg, "Failed to parse Stripe response"),
    };

    let session_id = session.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let checkout_url = session.get("url").and_then(|v| v.as_str()).unwrap_or("");

    // Update purchase with Stripe session ID
    let mut upd = HashMap::new();
    upd.insert("provider".to_string(), serde_json::Value::String("stripe".to_string()));
    upd.insert("provider_session_id".to_string(), serde_json::Value::String(session_id.to_string()));
    upd.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    if let Err(e) = db::update(ctx, PURCHASES_COLLECTION, &body.purchase_id, upd).await {
        tracing::warn!("Failed to update purchase with Stripe session ID: {e}");
    }

    json_respond(msg, &serde_json::json!({
        "session_id": session_id,
        "checkout_url": checkout_url
    }))
}

pub async fn handle_webhook(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    // Verify Stripe webhook signature - REQUIRED
    let webhook_secret = config::get_default(ctx, "STRIPE_WEBHOOK_SECRET", "").await;
    if webhook_secret.is_empty() {
        return err_internal(msg, "STRIPE_WEBHOOK_SECRET not configured — webhook processing disabled for security");
    }
    let sig_header = msg.header("stripe-signature");
    if sig_header.is_empty() {
        return err_unauthorized(msg, "Missing Stripe-Signature header");
    }
    if !verify_stripe_signature(&msg.data, sig_header, &webhook_secret) {
        return err_unauthorized(msg, "Invalid webhook signature");
    }

    // Parse webhook event
    let event: serde_json::Value = match msg.decode() {
        Ok(e) => e,
        Err(e) => return err_bad_request(msg, &format!("Invalid webhook body: {e}")),
    };

    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");

    let data_object = event.get("data").and_then(|d| d.get("object")).cloned().unwrap_or_default();

    match event_type {
        "checkout.session.completed" => {
            // Handle product purchase completion
            let purchase_id = data_object.pointer("/metadata/purchase_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if !purchase_id.is_empty() {
                let payment_intent = data_object.get("payment_intent")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let mut data = HashMap::new();
                data.insert("status".to_string(), serde_json::Value::String("completed".to_string()));
                data.insert("provider_payment_intent_id".to_string(), serde_json::Value::String(payment_intent));
                data.insert("approved_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                if let Err(e) = db::update(ctx, PURCHASES_COLLECTION, purchase_id, data).await {
                    tracing::error!("Failed to mark purchase as completed: {e}");
                    return err_internal(msg, &format!("Failed to update purchase: {e}"));
                }
            }

            // Handle subscription creation (platform billing)
            let user_id = data_object.pointer("/metadata/user_id").and_then(|v| v.as_str()).unwrap_or("");
            let plan = data_object.pointer("/metadata/plan").and_then(|v| v.as_str()).unwrap_or("");
            let stripe_customer_id = data_object.get("customer").and_then(|v| v.as_str()).unwrap_or("");
            let stripe_sub_id = data_object.get("subscription").and_then(|v| v.as_str()).unwrap_or("");

            if !user_id.is_empty() && !plan.is_empty() {
                let now = chrono::Utc::now().to_rfc3339();
                let sub_id = format!("sub_{}_{}", user_id, chrono::Utc::now().timestamp_millis());

                let _ = db::exec_raw(
                    ctx,
                    "INSERT INTO subscriptions (id, user_id, stripe_customer_id, stripe_subscription_id, plan, status, created_at, updated_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?6) \
                     ON CONFLICT (user_id) DO UPDATE SET \
                       stripe_customer_id = excluded.stripe_customer_id, \
                       stripe_subscription_id = excluded.stripe_subscription_id, \
                       plan = excluded.plan, \
                       status = 'active', \
                       updated_at = excluded.updated_at",
                    &[sub_id.into(), user_id.into(), stripe_customer_id.into(),
                      stripe_sub_id.into(), plan.into(), now.into()],
                ).await;

                fire_products_webhook(ctx, "products.checkout.completed", &serde_json::json!({
                    "user_id": user_id, "plan": plan
                })).await;
            }
        }

        "customer.subscription.updated" => {
            let stripe_sub_id = data_object.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let status = data_object.get("status").and_then(|v| v.as_str()).unwrap_or("");
            let plan = data_object.pointer("/items/data/0/price/lookup_key")
                .or_else(|| data_object.pointer("/items/data/0/price/metadata/plan"))
                .and_then(|v| v.as_str());
            let now = chrono::Utc::now().to_rfc3339();

            if let Some(plan) = plan {
                let _ = db::exec_raw(
                    ctx,
                    "UPDATE subscriptions SET status = ?1, plan = ?2, updated_at = ?3 WHERE stripe_subscription_id = ?4",
                    &[status.into(), plan.into(), now.clone().into(), stripe_sub_id.into()],
                ).await;
            } else {
                let _ = db::exec_raw(
                    ctx,
                    "UPDATE subscriptions SET status = ?1, updated_at = ?2 WHERE stripe_subscription_id = ?3",
                    &[status.into(), now.clone().into(), stripe_sub_id.into()],
                ).await;
            }

            // Sync addon totals from the subscription's items
            let user_id = get_user_for_stripe_sub(ctx, stripe_sub_id).await;
            if let Some(ref uid) = user_id {
                if let Some(items) = data_object.get("items") {
                    super::addons::sync_addons_from_stripe(ctx, uid, items).await;
                }
            }

            // Notify control plane
            if let Some(uid) = user_id {
                fire_products_webhook(ctx, "products.subscription.updated", &serde_json::json!({
                    "user_id": uid, "plan": plan.unwrap_or("free")
                })).await;
            }
        }

        "invoice.payment_failed" => {
            let stripe_sub_id = data_object.get("subscription").and_then(|v| v.as_str()).unwrap_or("");
            if !stripe_sub_id.is_empty() {
                let now = chrono::Utc::now().to_rfc3339();
                let grace_end = (chrono::Utc::now() + chrono::Duration::days(7)).to_rfc3339();
                let _ = db::exec_raw(
                    ctx,
                    "UPDATE subscriptions SET status = 'past_due', grace_period_end = ?1, updated_at = ?2 WHERE stripe_subscription_id = ?3",
                    &[grace_end.into(), now.into(), stripe_sub_id.into()],
                ).await;
            }
        }

        "customer.subscription.deleted" => {
            let stripe_sub_id = data_object.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let now = chrono::Utc::now().to_rfc3339();

            let user_id = get_user_for_stripe_sub(ctx, stripe_sub_id).await;

            // Cancel subscription and reset all addon columns to 0
            let _ = db::exec_raw(
                ctx,
                "UPDATE subscriptions SET status = 'cancelled', \
                   addon_projects = 0, addon_requests = 0, \
                   addon_r2_bytes = 0, addon_d1_bytes = 0, \
                   updated_at = ?1 \
                 WHERE stripe_subscription_id = ?2",
                &[now.into(), stripe_sub_id.into()],
            ).await;

            if let Some(uid) = user_id {
                fire_products_webhook(ctx, "products.subscription.deleted", &serde_json::json!({
                    "user_id": uid
                })).await;
            }
        }

        "charge.refunded" => {
            let payment_intent = data_object.get("payment_intent")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if !payment_intent.is_empty() {
                if let Ok(purchase) = db::get_by_field(
                    ctx, PURCHASES_COLLECTION,
                    "provider_payment_intent_id", serde_json::Value::String(payment_intent),
                ).await {
                    let mut data = HashMap::new();
                    data.insert("status".to_string(), serde_json::Value::String("refunded".to_string()));
                    data.insert("refunded_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                    data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                    if let Err(e) = db::update(ctx, PURCHASES_COLLECTION, &purchase.id, data).await {
                        tracing::error!("Failed to mark purchase as refunded: {e}");
                        return err_internal(msg, &format!("Failed to update purchase: {e}"));
                    }
                }
            }
        }

        _ => {
            // Ignore unhandled event types
        }
    }

    json_respond(msg, &serde_json::json!({"received": true}))
}

async fn get_user_for_stripe_sub(ctx: &dyn Context, stripe_sub_id: &str) -> Option<String> {
    let rows = db::query_raw(
        ctx,
        "SELECT user_id FROM subscriptions WHERE stripe_subscription_id = ?1",
        &[serde_json::Value::String(stripe_sub_id.to_string())],
    ).await.ok()?;
    rows.first()?.data.get("user_id").and_then(|v| v.as_str()).map(String::from)
}

/// Fire a webhook for product/billing events.
/// Best-effort — if PRODUCTS_WEBHOOK_URL is not configured, this is a no-op.
/// The webhook is signed with HMAC-SHA256 using PRODUCTS_WEBHOOK_SECRET.
async fn fire_products_webhook(ctx: &dyn Context, event: &str, data: &serde_json::Value) {
    let url = config::get_default(ctx, "PRODUCTS_WEBHOOK_URL", "").await;
    let secret = config::get_default(ctx, "PRODUCTS_WEBHOOK_SECRET", "").await;
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
            tracing::warn!(event = event, status = resp.status_code, "products webhook failed");
        }
        Err(e) => {
            tracing::warn!(event = event, error = %e, "products webhook delivery error");
        }
    }
}

pub(super) fn urlencoding(s: &str) -> String {
    // Iterate over bytes (not chars) to correctly handle multi-byte UTF-8
    s.as_bytes().iter().map(|&b| match b {
        b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
            String::from(b as char)
        }
        _ => format!("%{:02X}", b),
    }).collect()
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

        let sig_header = format!("t={},v1=0000000000000000000000000000000000000000000000000000000000000000", timestamp);

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
        assert_eq!(urlencoding("https://example.com"), "https%3A%2F%2Fexample.com");
    }
}
