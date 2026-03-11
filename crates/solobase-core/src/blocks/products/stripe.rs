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

    let total = purchase.data.get("total_amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let currency = purchase.data.get("currency").and_then(|v| v.as_str()).unwrap_or("usd").to_lowercase();

    let base_url = config::get_default(ctx, "FRONTEND_URL", "http://localhost:5173").await;
    let success_url = body.success_url.unwrap_or_else(|| format!("{}/checkout/success?session_id={{CHECKOUT_SESSION_ID}}", base_url));
    let cancel_url = body.cancel_url.unwrap_or_else(|| format!("{}/checkout/cancel", base_url));

    // Create Stripe checkout session
    let stripe_body = format!(
        "payment_method_types[]=card&line_items[0][price_data][currency]={}&line_items[0][price_data][unit_amount]={}&line_items[0][price_data][product_data][name]=Order {}&line_items[0][quantity]=1&mode=payment&success_url={}&cancel_url={}&metadata[purchase_id]={}",
        currency,
        (total * 100.0) as i64, // Stripe uses cents
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
    upd.insert("payment_provider".to_string(), serde_json::Value::String("stripe".to_string()));
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

    match event_type {
        "checkout.session.completed" => {
            let data = event.get("data").and_then(|d| d.get("object"));
            if let Some(session) = data {
                let purchase_id = session.get("metadata")
                    .and_then(|m| m.get("purchase_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if !purchase_id.is_empty() {
                    let payment_intent = session.get("payment_intent")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let mut data = HashMap::new();
                    data.insert("status".to_string(), serde_json::Value::String("completed".to_string()));
                    data.insert("provider_payment_intent_id".to_string(), serde_json::Value::String(payment_intent));
                    data.insert("approved_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                    data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                    if let Err(e) = db::update(ctx, PURCHASES_COLLECTION, purchase_id, data).await {
                        tracing::warn!("Failed to mark purchase as completed: {e}");
                    }
                }
            }
        }
        "charge.refunded" => {
            let data = event.get("data").and_then(|d| d.get("object"));
            if let Some(charge) = data {
                let payment_intent = charge.get("payment_intent")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if !payment_intent.is_empty() {
                    // Find purchase by provider_payment_intent_id
                    if let Ok(purchase) = db::get_by_field(
                        ctx, PURCHASES_COLLECTION,
                        "provider_payment_intent_id", serde_json::Value::String(payment_intent),
                    ).await {
                        let mut data = HashMap::new();
                        data.insert("status".to_string(), serde_json::Value::String("refunded".to_string()));
                        data.insert("refunded_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                        data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                        if let Err(e) = db::update(ctx, PURCHASES_COLLECTION, &purchase.id, data).await {
                            tracing::warn!("Failed to mark purchase as refunded: {e}");
                        }
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

fn urlencoding(s: &str) -> String {
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
