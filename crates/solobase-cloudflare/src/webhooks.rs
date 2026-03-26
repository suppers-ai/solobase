//! Webhook receivers — handle incoming webhooks from solobase blocks.
//!
//! `/_webhooks/products` receives billing events from the products block
//! (checkout completed, subscription changes). Verified via HMAC-SHA256
//! signature using the PRODUCTS_WEBHOOK_SECRET.

use worker::*;

use crate::helpers::json_response;
use crate::provision;

/// Handle an incoming webhook request.
pub async fn handle(req: &Request, env: &Env, path: &str, body: &[u8]) -> Result<Response> {
    match path {
        "products" => handle_products_webhook(req, env, body).await,
        _ => json_response(
            &serde_json::json!({"error": "not_found", "message": "webhook endpoint not found"}),
            404,
        ),
    }
}

async fn handle_products_webhook(req: &Request, env: &Env, body: &[u8]) -> Result<Response> {
    // Verify HMAC-SHA256 signature
    let secret = env.secret("PRODUCTS_WEBHOOK_SECRET")
        .map(|s| s.to_string())
        .or_else(|_| env.var("PRODUCTS_WEBHOOK_SECRET").map(|v| v.to_string()))
        .unwrap_or_default();

    if !secret.is_empty() {
        let sig_header = req.headers()
            .get("x-webhook-signature")
            .ok()
            .flatten()
            .unwrap_or_default();

        if !verify_webhook_signature(body, &sig_header, &secret) {
            return json_response(
                &serde_json::json!({"error": "unauthorized", "message": "invalid webhook signature"}),
                401,
            );
        }
    }

    // Parse webhook payload
    #[derive(serde::Deserialize)]
    struct WebhookPayload {
        event: String,
        #[serde(default)]
        data: serde_json::Value,
    }

    let payload: WebhookPayload = serde_json::from_slice(body)
        .map_err(|e| Error::RustError(format!("invalid webhook body: {e}")))?;

    let kv = env.kv("PROJECTS")
        .map_err(|e| Error::RustError(format!("KV binding: {e}")))?;

    match payload.event.as_str() {
        "products.checkout.completed" | "products.subscription.updated" => {
            let user_id = payload.data.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
            let plan = payload.data.get("plan").and_then(|v| v.as_str()).unwrap_or("free");

            if user_id.is_empty() {
                return json_response(
                    &serde_json::json!({"error": "bad_request", "message": "missing user_id"}),
                    400,
                );
            }

            // Update plan on all projects owned by this user
            let updated = sync_user_projects(&kv, user_id, plan, "active").await?;
            console_log!("Webhook {}: updated {} projects for user {}", payload.event, updated, user_id);

            json_response(&serde_json::json!({"ok": true, "updated": updated}), 200)
        }

        "products.subscription.deleted" => {
            let user_id = payload.data.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

            if user_id.is_empty() {
                return json_response(
                    &serde_json::json!({"error": "bad_request", "message": "missing user_id"}),
                    400,
                );
            }

            // Suspend all projects owned by this user
            let updated = sync_user_projects(&kv, user_id, "free", "suspended").await?;
            console_log!("Webhook {}: suspended {} projects for user {}", payload.event, updated, user_id);

            json_response(&serde_json::json!({"ok": true, "suspended": updated}), 200)
        }

        _ => {
            console_log!("Unknown webhook event: {}", payload.event);
            json_response(&serde_json::json!({"ok": true, "ignored": true}), 200)
        }
    }
}

/// Update plan/status on all projects owned by a user.
async fn sync_user_projects(
    kv: &kv::KvStore,
    user_id: &str,
    plan: &str,
    status: &str,
) -> Result<u32> {
    let subdomains = provision::list_projects(kv).await?;
    let mut updated = 0;

    for subdomain in &subdomains {
        if let Some(mut config) = provision::get_project(kv, subdomain).await? {
            if config.owner_user_id.as_deref() == Some(user_id) {
                config.plan = plan.to_string();
                config.status = status.to_string();
                provision::update_project(kv, subdomain, &config).await?;
                updated += 1;
            }
        }
    }

    Ok(updated)
}

/// Verify HMAC-SHA256 webhook signature.
/// Expected format: `sha256={hex_encoded_hmac}`
fn verify_webhook_signature(payload: &[u8], sig_header: &str, secret: &str) -> bool {
    let expected_hex = match sig_header.strip_prefix("sha256=") {
        Some(hex) => hex,
        None => return false,
    };

    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC key");
    mac.update(payload);
    let computed: [u8; 32] = mac.finalize().into_bytes().into();
    let computed_hex: String = computed.iter().map(|b| format!("{:02x}", b)).collect();

    // Constant-time comparison
    if computed_hex.len() != expected_hex.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in computed_hex.bytes().zip(expected_hex.bytes()) {
        diff |= a ^ b;
    }
    diff == 0
}
