use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::{config, database as db, network};
use super::PURCHASES_COLLECTION;
use crate::blocks::helpers::hex_encode;

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
    if !verify_stripe_signature(&msg.data, &sig_header, &webhook_secret) {
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
    // HMAC-SHA256 per RFC 2104
    let block_size = 64;
    let mut padded_key = [0u8; 64];

    if key.len() > block_size {
        // Hash key if longer than block size (using simple sha256)
        let hashed = sha256(key);
        padded_key[..32].copy_from_slice(&hashed);
    } else {
        padded_key[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= padded_key[i];
        opad[i] ^= padded_key[i];
    }

    // inner hash = SHA256(ipad || data)
    let mut inner_input = Vec::with_capacity(64 + data.len());
    inner_input.extend_from_slice(&ipad);
    inner_input.extend_from_slice(data);
    let inner_hash = sha256(&inner_input);

    // outer hash = SHA256(opad || inner_hash)
    let mut outer_input = Vec::with_capacity(64 + 32);
    outer_input.extend_from_slice(&opad);
    outer_input.extend_from_slice(&inner_hash);
    sha256(&outer_input)
}

fn sha256(data: &[u8]) -> [u8; 32] {
    // SHA-256 implementation using the constants and algorithm from FIPS 180-4
    let k: [u32; 64] = [
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    // Pre-processing: pad message
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    // Process each 512-bit block
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[4*i], chunk[4*i+1], chunk[4*i+2], chunk[4*i+3]]);
        }
        for i in 16..64 {
            let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
            let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(k[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g; g = f; f = e; e = d.wrapping_add(temp1);
            d = c; c = b; b = a; a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a); h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c); h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e); h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g); h[7] = h[7].wrapping_add(hh);
    }

    let mut result = [0u8; 32];
    for (i, &val) in h.iter().enumerate() {
        result[4*i..4*i+4].copy_from_slice(&val.to_be_bytes());
    }
    result
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
