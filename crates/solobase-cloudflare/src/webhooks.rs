//! Webhook receivers — handle incoming webhooks from solobase blocks.
//!
//! `/_webhooks/products` receives billing events from the products block
//! (checkout completed, subscription changes). Verified via HMAC-SHA256
//! signature using the PRODUCTS_WEBHOOK_SECRET from the cloud project's D1.

use worker::*;

use crate::cf_api::{self, CfCredentials};
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
    // Read PRODUCTS_WEBHOOK_SECRET from the cloud project's D1 database
    let secret = get_cloud_variable(env, "PRODUCTS_WEBHOOK_SECRET").await.unwrap_or_default();

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

    let db = env.d1("DB")
        .map_err(|e| Error::RustError(format!("D1 binding: {e}")))?;

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

            let updated = sync_user_projects(&db, user_id, plan, "active").await?;
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

            let updated = sync_user_projects(&db, user_id, "free", "suspended").await?;
            console_log!("Webhook {}: suspended {} projects for user {}", payload.event, updated, user_id);

            json_response(&serde_json::json!({"ok": true, "suspended": updated}), 200)
        }

        _ => {
            console_log!("Unknown webhook event: {}", payload.event);
            json_response(&serde_json::json!({"ok": true, "ignored": true}), 200)
        }
    }
}

/// Read a variable from the cloud project's D1 database via CF API.
/// Looks up the cloud project's db_id from D1, then queries its project D1.
async fn get_cloud_variable(env: &Env, key: &str) -> Option<String> {
    let db = env.d1("DB").ok()?;
    let config = provision::get_project(&db, "cloud").await.ok()??;
    let db_id = config.db_id.as_ref()?;

    let creds = CfCredentials::from_env(env).ok()?;
    let rows = cf_api::query_d1(
        &creds, db_id,
        "SELECT value FROM variables WHERE key = ?1",
        &[key],
    ).await.ok()?;

    rows.first()
        .and_then(|row| row.get("value"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Update plan/status on all projects owned by a user.
async fn sync_user_projects(
    db: &D1Database,
    user_id: &str,
    plan: &str,
    status: &str,
) -> Result<u32> {
    // Update all projects owned by this user in a single query
    let result = db
        .prepare(
            "UPDATE projects SET plan = ?1, status = ?2, updated_at = datetime('now') \
             WHERE owner_user_id = ?3"
        )
        .bind(&[plan.into(), status.into(), user_id.into()])?
        .run()
        .await?;

    // D1 run() returns metadata; extract rows_written for the count
    let meta = result.meta()?;
    let updated = meta
        .and_then(|m| m.rows_written)
        .unwrap_or(0) as u32;

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
