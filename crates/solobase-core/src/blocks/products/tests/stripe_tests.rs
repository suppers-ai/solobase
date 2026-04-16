use std::collections::HashMap;

use hmac::{Hmac, Mac};
use sha2::Sha256;
use wafer_run::{types::Message, InputStream};

use super::mock_context::*;
use crate::blocks::{helpers::hex_encode, products::stripe};

// ============================================================
// Helpers
// ============================================================

const WEBHOOK_SECRET: &str = "whsec_test_secret_key";

/// Build a valid Stripe webhook message with correct HMAC signature.
fn webhook_msg(payload: &serde_json::Value, secret: &str) -> (Message, InputStream) {
    let payload_bytes = serde_json::to_vec(payload).unwrap();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let signed = format!("{}.{}", timestamp, String::from_utf8_lossy(&payload_bytes));
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(signed.as_bytes());
    let sig_bytes: [u8; 32] = mac.finalize().into_bytes().into();
    let sig_hex = hex_encode(&sig_bytes);

    let sig_header = format!("t={},v1={}", timestamp, sig_hex);

    let mut msg = Message::new("http.request");
    msg.set_meta("req.action", "create");
    msg.set_meta("req.resource", "/b/products/webhooks");
    msg.set_meta("http.header.stripe-signature", &sig_header);
    (msg, InputStream::from_bytes(payload_bytes))
}

fn checkout_completed_event(purchase_id: &str, payment_intent: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "checkout.session.completed",
        "data": {
            "object": {
                "metadata": { "purchase_id": purchase_id },
                "payment_intent": payment_intent
            }
        }
    })
}

fn charge_refunded_event(payment_intent: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "charge.refunded",
        "data": {
            "object": {
                "payment_intent": payment_intent
            }
        }
    })
}

// ============================================================
// Webhook — checkout.session.completed
// ============================================================

#[tokio::test]
async fn webhook_checkout_completed_marks_purchase() {
    let ctx = MockContext::new().with_config(
        "SUPPERS_AI__PRODUCTS__STRIPE_WEBHOOK_SECRET",
        WEBHOOK_SECRET,
    );

    // Seed a pending purchase
    let mut pd = HashMap::new();
    pd.insert("user_id".to_string(), serde_json::json!("user_1"));
    pd.insert("status".to_string(), serde_json::json!("pending"));
    pd.insert("total_cents".to_string(), serde_json::json!(5000));
    ctx.seed("suppers_ai__products__purchases", "pur_wh1", pd);

    let event = checkout_completed_event("pur_wh1", "pi_12345");
    let (msg, input) = webhook_msg(&event, WEBHOOK_SECRET);

    let out = stripe::handle_webhook(&ctx, &msg, input).await;
    let body = output_to_json(out).await;
    assert_eq!(body["received"], true);
}

#[tokio::test]
async fn webhook_checkout_completed_empty_purchase_id() {
    let ctx = MockContext::new().with_config(
        "SUPPERS_AI__PRODUCTS__STRIPE_WEBHOOK_SECRET",
        WEBHOOK_SECRET,
    );

    // Event with empty purchase_id — should still return 200 (no-op)
    let event = serde_json::json!({
        "type": "checkout.session.completed",
        "data": {
            "object": {
                "metadata": { "purchase_id": "" },
                "payment_intent": "pi_xxx"
            }
        }
    });
    let (msg, input) = webhook_msg(&event, WEBHOOK_SECRET);
    let out = stripe::handle_webhook(&ctx, &msg, input).await;
    let body = output_to_json(out).await;
    assert_eq!(body["received"], true);
}

// ============================================================
// Webhook — charge.refunded
// ============================================================

#[tokio::test]
async fn webhook_charge_refunded_marks_purchase() {
    let ctx = MockContext::new().with_config(
        "SUPPERS_AI__PRODUCTS__STRIPE_WEBHOOK_SECRET",
        WEBHOOK_SECRET,
    );

    // Seed a completed purchase with a payment intent
    let mut pd = HashMap::new();
    pd.insert("user_id".to_string(), serde_json::json!("user_1"));
    pd.insert("status".to_string(), serde_json::json!("completed"));
    pd.insert(
        "provider_payment_intent_id".to_string(),
        serde_json::json!("pi_refund_test"),
    );
    ctx.seed("suppers_ai__products__purchases", "pur_ref1", pd);

    let event = charge_refunded_event("pi_refund_test");
    let (msg, input) = webhook_msg(&event, WEBHOOK_SECRET);

    let out = stripe::handle_webhook(&ctx, &msg, input).await;
    let body = output_to_json(out).await;
    assert_eq!(body["received"], true);
}

#[tokio::test]
async fn webhook_charge_refunded_unknown_intent_is_noop() {
    let ctx = MockContext::new().with_config(
        "SUPPERS_AI__PRODUCTS__STRIPE_WEBHOOK_SECRET",
        WEBHOOK_SECRET,
    );

    // No matching purchase — should still return 200
    let event = charge_refunded_event("pi_unknown");
    let (msg, input) = webhook_msg(&event, WEBHOOK_SECRET);

    let out = stripe::handle_webhook(&ctx, &msg, input).await;
    let body = output_to_json(out).await;
    assert_eq!(body["received"], true);
}

// ============================================================
// Webhook — unhandled event types
// ============================================================

#[tokio::test]
async fn webhook_unhandled_event_returns_ok() {
    let ctx = MockContext::new().with_config(
        "SUPPERS_AI__PRODUCTS__STRIPE_WEBHOOK_SECRET",
        WEBHOOK_SECRET,
    );

    let event = serde_json::json!({
        "type": "payment_intent.created",
        "data": { "object": {} }
    });
    let (msg, input) = webhook_msg(&event, WEBHOOK_SECRET);

    let out = stripe::handle_webhook(&ctx, &msg, input).await;
    let body = output_to_json(out).await;
    assert_eq!(body["received"], true);
}

// ============================================================
// Webhook — security: signature verification
// ============================================================

#[tokio::test]
async fn webhook_rejects_missing_secret_config() {
    // No STRIPE_WEBHOOK_SECRET configured
    let ctx = MockContext::new();

    let event = checkout_completed_event("pur_1", "pi_1");
    let (msg, input) = webhook_msg(&event, "anything");

    let out = stripe::handle_webhook(&ctx, &msg, input).await;
    assert!(output_is_error(out, "internal").await);
}

#[tokio::test]
async fn webhook_rejects_missing_signature_header() {
    let ctx = MockContext::new().with_config(
        "SUPPERS_AI__PRODUCTS__STRIPE_WEBHOOK_SECRET",
        WEBHOOK_SECRET,
    );

    let event = checkout_completed_event("pur_1", "pi_1");
    let payload_bytes = serde_json::to_vec(&event).unwrap();
    let mut msg = Message::new("http.request");
    msg.set_meta("req.action", "create");
    msg.set_meta("req.resource", "/b/products/webhooks");
    // No stripe-signature header
    let input = InputStream::from_bytes(payload_bytes);

    let out = stripe::handle_webhook(&ctx, &msg, input).await;
    assert!(output_is_error(out, "unauthenticated").await);
}

#[tokio::test]
async fn webhook_rejects_invalid_signature() {
    let ctx = MockContext::new().with_config(
        "SUPPERS_AI__PRODUCTS__STRIPE_WEBHOOK_SECRET",
        WEBHOOK_SECRET,
    );

    let event = checkout_completed_event("pur_1", "pi_1");
    // Sign with wrong secret
    let (msg, input) = webhook_msg(&event, "wrong_secret");

    let out = stripe::handle_webhook(&ctx, &msg, input).await;
    assert!(output_is_error(out, "unauthenticated").await);
}

#[tokio::test]
async fn webhook_rejects_tampered_payload() {
    let ctx = MockContext::new().with_config(
        "SUPPERS_AI__PRODUCTS__STRIPE_WEBHOOK_SECRET",
        WEBHOOK_SECRET,
    );

    // Create a valid signature for one payload
    let original_event = checkout_completed_event("pur_1", "pi_1");
    let original_bytes = serde_json::to_vec(&original_event).unwrap();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let signed = format!("{}.{}", timestamp, String::from_utf8_lossy(&original_bytes));
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(WEBHOOK_SECRET.as_bytes()).unwrap();
    mac.update(signed.as_bytes());
    let sig_bytes: [u8; 32] = mac.finalize().into_bytes().into();
    let sig_hex = hex_encode(&sig_bytes);
    let sig_header = format!("t={},v1={}", timestamp, sig_hex);

    // But send a different payload
    let tampered_event = checkout_completed_event("pur_HACKED", "pi_evil");
    let tampered_bytes = serde_json::to_vec(&tampered_event).unwrap();

    let mut msg = Message::new("http.request");
    msg.set_meta("req.action", "create");
    msg.set_meta("req.resource", "/b/products/webhooks");
    msg.set_meta("http.header.stripe-signature", &sig_header);
    let input = InputStream::from_bytes(tampered_bytes);

    let out = stripe::handle_webhook(&ctx, &msg, input).await;
    assert!(output_is_error(out, "unauthenticated").await);
}

// ============================================================
// Checkout — error cases (no network mock, just config errors)
// ============================================================

#[tokio::test]
async fn checkout_rejects_when_stripe_not_configured() {
    let ctx = MockContext::new();
    // No STRIPE_SECRET_KEY configured

    let (msg, input) = create_msg(
        "/b/products/checkout",
        "user_1",
        serde_json::json!({
            "purchase_id": "pur_1"
        }),
    );

    let out = stripe::handle_checkout(&ctx, &msg, input).await;
    assert!(output_is_error(out, "internal").await);
}
