use std::collections::HashMap;
use crate::wafer::block_world::types::*;
use wafer_core::clients::{config, database as db, network};
use crate::helpers::*;
use crate::PURCHASES_COLLECTION;

pub fn handle_checkout(msg: &Message) -> BlockResult {
    let stripe_key = match config::get("STRIPE_SECRET_KEY") {
        Ok(k) => k,
        Err(_) => return err_internal(msg, "Stripe is not configured"),
    };

    #[derive(serde::Deserialize)]
    struct CheckoutReq {
        purchase_id: String,
        success_url: Option<String>,
        cancel_url: Option<String>,
    }
    let body: CheckoutReq = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    // Get purchase
    let purchase = match db::get(PURCHASES_COLLECTION, &body.purchase_id) {
        Ok(p) => p,
        Err(_) => return err_not_found(msg, "Purchase not found"),
    };

    let total_cents = purchase.data.get("total_cents").and_then(|v| v.as_i64()).unwrap_or(0);
    let currency = purchase.data.get("currency").and_then(|v| v.as_str()).unwrap_or("usd").to_lowercase();

    let base_url = config::get_default("FRONTEND_URL", "http://localhost:5173");
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

    let stripe_api_url = config::get_default("STRIPE_API_URL", "https://api.stripe.com");
    let checkout_url_endpoint = format!("{}/v1/checkout/sessions", stripe_api_url);

    let resp = match network::do_request(
        "POST",
        &checkout_url_endpoint,
        &headers,
        Some(&stripe_body.into_bytes()),
    ) {
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
    let now = now_rfc3339();
    let mut upd = HashMap::new();
    upd.insert("provider".to_string(), serde_json::Value::String("stripe".to_string()));
    upd.insert("provider_session_id".to_string(), serde_json::Value::String(session_id.to_string()));
    upd.insert("updated_at".to_string(), serde_json::Value::String(now));
    let _ = db::update(PURCHASES_COLLECTION, &body.purchase_id, upd);

    json_respond(msg, &serde_json::json!({
        "session_id": session_id,
        "checkout_url": checkout_url
    }))
}

pub fn handle_webhook(msg: &Message) -> BlockResult {
    // Verify Stripe webhook signature - REQUIRED
    let webhook_secret = config::get_default("STRIPE_WEBHOOK_SECRET", "");
    if webhook_secret.is_empty() {
        return err_internal(msg, "STRIPE_WEBHOOK_SECRET not configured — webhook processing disabled for security");
    }
    let sig_header = msg_header(msg, "stripe-signature");
    if sig_header.is_empty() {
        return err_unauthorized(msg, "Missing Stripe-Signature header");
    }
    if !verify_stripe_signature(&msg.data, sig_header, &webhook_secret) {
        return err_unauthorized(msg, "Invalid webhook signature");
    }

    // Parse webhook event
    let event: serde_json::Value = match decode_body(msg) {
        Ok(e) => e,
        Err(r) => return r,
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

                    let now = now_rfc3339();
                    let mut data = HashMap::new();
                    data.insert("status".to_string(), serde_json::Value::String("completed".to_string()));
                    data.insert("provider_payment_intent_id".to_string(), serde_json::Value::String(payment_intent));
                    data.insert("approved_at".to_string(), serde_json::Value::String(now.clone()));
                    data.insert("updated_at".to_string(), serde_json::Value::String(now));
                    if let Err(e) = db::update(PURCHASES_COLLECTION, purchase_id, data) {
                        return err_internal(msg, &format!("Failed to update purchase: {e}"));
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
                        PURCHASES_COLLECTION,
                        "provider_payment_intent_id", serde_json::Value::String(payment_intent),
                    ) {
                        let now = now_rfc3339();
                        let mut data = HashMap::new();
                        data.insert("status".to_string(), serde_json::Value::String("refunded".to_string()));
                        data.insert("refunded_at".to_string(), serde_json::Value::String(now.clone()));
                        data.insert("updated_at".to_string(), serde_json::Value::String(now));
                        if let Err(e) = db::update(PURCHASES_COLLECTION, &purchase.id, data) {
                            return err_internal(msg, &format!("Failed to update purchase: {e}"));
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

    // Validate timestamp is a valid number (we skip replay protection in WASM
    // since there is no system clock — the host/platform handles this)
    if timestamp.parse::<u64>().is_err() {
        return false;
    }

    // Compute expected signature: HMAC-SHA256(secret, "timestamp.payload")
    let signed_payload = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));

    let computed = hmac_sha256(secret.as_bytes(), signed_payload.as_bytes());
    let computed_hex = hex_encode(&computed);

    // Constant-time comparison
    constant_time_eq(computed_hex.as_bytes(), expected_sig.as_bytes())
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    // Manual HMAC-SHA256 implementation (no external crate dependencies for WASM)
    // HMAC(K, m) = H((K' ^ opad) || H((K' ^ ipad) || m))
    use sha2::{Sha256, Digest};

    let block_size = 64;

    // If key is longer than block size, hash it
    let key_prime: Vec<u8> = if key.len() > block_size {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.finalize().to_vec()
    } else {
        key.to_vec()
    };

    // Pad key to block size
    let mut key_padded = key_prime.clone();
    key_padded.resize(block_size, 0);

    // Inner and outer pads
    let ipad: Vec<u8> = key_padded.iter().map(|b| b ^ 0x36).collect();
    let opad: Vec<u8> = key_padded.iter().map(|b| b ^ 0x5c).collect();

    // Inner hash: H(ipad || data)
    let mut inner_hasher = Sha256::new();
    inner_hasher.update(&ipad);
    inner_hasher.update(data);
    let inner_hash = inner_hasher.finalize();

    // Outer hash: H(opad || inner_hash)
    let mut outer_hasher = Sha256::new();
    outer_hasher.update(&opad);
    outer_hasher.update(inner_hash);
    let result = outer_hasher.finalize();

    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
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
