//! Control plane API — platform-level tenant management.
//!
//! These endpoints are protected by a platform admin secret and handle
//! tenant provisioning, configuration, and monitoring.
//!
//! All control plane routes are under `/_control/` and require the
//! `X-Admin-Secret` header to match the `ADMIN_SECRET` environment variable.

use std::collections::HashMap;

use worker::*;

use crate::provision;

/// JSON response helper.
fn json_response<T: serde::Serialize>(data: &T, status: u16) -> Result<Response> {
    let body = serde_json::to_string(data)
        .map_err(|e| Error::RustError(format!("JSON serialize: {e}")))?;
    let resp = Response::ok(body)?;
    let mut resp = resp.with_status(status);
    resp.headers_mut().set("Content-Type", "application/json")?;
    Ok(resp)
}

/// Handle a control plane request.
///
/// Called when the path starts with `/_control/`.
pub async fn handle(req: &Request, env: &Env, path: &str, body: &[u8]) -> Result<Response> {
    // Verify admin secret
    let provided = req
        .headers()
        .get("x-admin-secret")
        .ok()
        .flatten()
        .unwrap_or_default();

    let expected = env
        .secret("ADMIN_SECRET")
        .map(|s| s.to_string())
        .or_else(|_| env.var("ADMIN_SECRET").map(|v| v.to_string()))
        .unwrap_or_default();

    if expected.is_empty() || provided != expected {
        return json_response(
            &serde_json::json!({"error": "unauthorized", "message": "invalid admin secret"}),
            401,
        );
    }

    let kv = env
        .kv("TENANTS")
        .map_err(|e| Error::RustError(format!("KV binding: {e}")))?;

    let db = env
        .d1("DB")
        .map_err(|e| Error::RustError(format!("D1 binding: {e}")))?;

    let method = req.method().to_string();

    match (method.as_str(), path) {
        // List all tenants
        ("GET", "tenants") => {
            let tenants = provision::list_tenants(&kv).await?;
            json_response(&serde_json::json!({"tenants": tenants}), 200)
        }

        // Get a specific tenant
        ("GET", _) if path.starts_with("tenants/") => {
            let subdomain = path.strip_prefix("tenants/").unwrap_or("");
            match provision::get_tenant(&kv, subdomain).await? {
                Some(config) => json_response(&config, 200),
                None => json_response(
                    &serde_json::json!({"error": "not_found", "message": "tenant not found"}),
                    404,
                ),
            }
        }

        // Create a new tenant
        ("POST", "tenants") => {
            #[derive(serde::Deserialize)]
            struct Req {
                subdomain: String,
                #[serde(default = "default_plan")]
                plan: String,
            }
            fn default_plan() -> String {
                "hobby".into()
            }

            let req: Req = serde_json::from_slice(body)
                .map_err(|e| Error::RustError(format!("invalid body: {e}")))?;

            let config = provision::create_tenant(&kv, &db, &req.subdomain, &req.plan).await?;
            json_response(&config, 201)
        }

        // Update a tenant
        ("PUT" | "PATCH", _) if path.starts_with("tenants/") => {
            let subdomain = path.strip_prefix("tenants/").unwrap_or("");
            let current = provision::get_tenant(&kv, subdomain)
                .await?
                .ok_or_else(|| Error::RustError("tenant not found".into()))?;

            let updates: HashMap<String, serde_json::Value> =
                serde_json::from_slice(body)
                    .map_err(|e| Error::RustError(format!("invalid body: {e}")))?;

            let mut config = current;
            if let Some(plan) = updates.get("plan").and_then(|v| v.as_str()) {
                config.plan = plan.to_string();
            }
            if let Some(features) = updates.get("features") {
                if let Ok(f) = serde_json::from_value(features.clone()) {
                    config.features = f;
                }
            }

            provision::update_tenant(&kv, subdomain, &config).await?;
            json_response(&config, 200)
        }

        // Delete a tenant
        ("DELETE", _) if path.starts_with("tenants/") => {
            let subdomain = path.strip_prefix("tenants/").unwrap_or("");
            provision::delete_tenant(&kv, subdomain).await?;
            json_response(&serde_json::json!({"deleted": true}), 200)
        }

        // Run schema migrations
        ("POST", "migrate") => {
            crate::schema::run_migrations(&db).await?;
            json_response(
                &serde_json::json!({"status": "ok", "message": "migrations applied"}),
                200,
            )
        }

        // Platform health
        ("GET", "health") => {
            let tenants = provision::list_tenants(&kv).await?;
            json_response(
                &serde_json::json!({
                    "status": "ok",
                    "tenant_count": tenants.len(),
                    "version": env!("CARGO_PKG_VERSION"),
                }),
                200,
            )
        }

        _ => json_response(
            &serde_json::json!({"error": "not_found", "message": "control endpoint not found"}),
            404,
        ),
    }
}
