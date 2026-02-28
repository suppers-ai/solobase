use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_run::services::database::{self};
use super::get_db;

const PURCHASES_COLLECTION: &str = "ext_products_purchases";

pub fn handle_checkout(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };
    let config = match ctx.services().and_then(|s| s.config.as_ref()) {
        Some(c) => c,
        None => return err_internal(msg.clone(), "Config service unavailable"),
    };
    let network = match ctx.services().and_then(|s| s.network.as_ref()) {
        Some(n) => n,
        None => return err_internal(msg.clone(), "Network service unavailable"),
    };

    let stripe_key = match config.get("STRIPE_SECRET_KEY") {
        Some(k) => k,
        None => return err_internal(msg.clone(), "Stripe is not configured"),
    };

    #[derive(serde::Deserialize)]
    struct CheckoutReq {
        purchase_id: String,
        success_url: Option<String>,
        cancel_url: Option<String>,
    }
    let body: CheckoutReq = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };

    // Get purchase
    let purchase = match db.get(PURCHASES_COLLECTION, &body.purchase_id) {
        Ok(p) => p,
        Err(_) => return err_not_found(msg.clone(), "Purchase not found"),
    };

    let total = purchase.data.get("total_amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let currency = purchase.data.get("currency").and_then(|v| v.as_str()).unwrap_or("usd").to_lowercase();

    let base_url = config.get_default("FRONTEND_URL", "http://localhost:5173");
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

    let resp = match network.do_request(&wafer_run::services::network::Request {
        method: "POST".to_string(),
        url: "https://api.stripe.com/v1/checkout/sessions".to_string(),
        headers,
        body: Some(stripe_body.into_bytes()),
    }) {
        Ok(r) => r,
        Err(e) => return err_internal(msg.clone(), &format!("Stripe API error: {e}")),
    };

    if resp.status_code >= 400 {
        let err_body = String::from_utf8_lossy(&resp.body);
        return err_internal(msg.clone(), &format!("Stripe error ({}): {}", resp.status_code, err_body));
    }

    let session: serde_json::Value = match serde_json::from_slice(&resp.body) {
        Ok(d) => d,
        Err(_) => return err_internal(msg.clone(), "Failed to parse Stripe response"),
    };

    let session_id = session.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let checkout_url = session.get("url").and_then(|v| v.as_str()).unwrap_or("");

    // Update purchase with Stripe session ID
    let mut upd = HashMap::new();
    upd.insert("payment_provider".to_string(), serde_json::Value::String("stripe".to_string()));
    upd.insert("payment_id".to_string(), serde_json::Value::String(session_id.to_string()));
    upd.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    let _ = db.update(PURCHASES_COLLECTION, &body.purchase_id, upd);

    json_respond(msg.clone(), 200, &serde_json::json!({
        "session_id": session_id,
        "checkout_url": checkout_url
    }))
}

pub fn handle_webhook(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };

    // Parse webhook event
    let event: serde_json::Value = match msg.decode() {
        Ok(e) => e,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid webhook body: {e}")),
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
                    data.insert("payment_intent".to_string(), serde_json::Value::String(payment_intent));
                    data.insert("completed_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                    data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                    let _ = db.update(PURCHASES_COLLECTION, purchase_id, data);
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
                    // Find purchase by payment_intent
                    if let Ok(purchase) = database::get_by_field(
                        db.as_ref(), PURCHASES_COLLECTION,
                        "payment_intent", serde_json::Value::String(payment_intent),
                    ) {
                        let mut data = HashMap::new();
                        data.insert("status".to_string(), serde_json::Value::String("refunded".to_string()));
                        data.insert("refunded_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                        data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                        let _ = db.update(PURCHASES_COLLECTION, &purchase.id, data);
                    }
                }
            }
        }
        _ => {
            // Ignore unhandled event types
        }
    }

    json_respond(msg.clone(), 200, &serde_json::json!({"received": true}))
}

fn urlencoding(s: &str) -> String {
    s.chars().map(|c| match c {
        'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
        _ => format!("%{:02X}", c as u8),
    }).collect()
}
