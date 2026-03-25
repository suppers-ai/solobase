//! Control plane API — platform-level project management.
//!
//! All control plane routes are under `/_control/` and require the
//! `X-Admin-Secret` header to match the `ADMIN_SECRET` environment variable.
//! Uses constant-time comparison to prevent timing attacks.

use std::collections::HashMap;

use worker::*;

use crate::helpers::json_response;
use crate::project::is_reserved_subdomain;
use crate::provision;

/// Handle a control plane request.
pub async fn handle(req: &Request, env: &Env, path: &str, body: &[u8]) -> Result<Response> {
    // Verify admin secret with constant-time comparison
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

    if expected.is_empty() || !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        return json_response(
            &serde_json::json!({"error": "unauthorized", "message": "invalid admin secret"}),
            401,
        );
    }

    let kv = env
        .kv("PROJECTS")
        .map_err(|e| Error::RustError(format!("KV binding: {e}")))?;

    let db = env
        .d1("DB")
        .map_err(|e| Error::RustError(format!("D1 binding: {e}")))?;

    let method = req.method().to_string();

    match (method.as_str(), path) {
        // List all projects
        ("GET", "projects") => {
            let projects = provision::list_projects(&kv).await?;
            json_response(&serde_json::json!({"projects": projects}), 200)
        }

        // Get a specific project
        ("GET", _) if path.starts_with("projects/") => {
            let subdomain = path.strip_prefix("projects/").unwrap_or("");
            match provision::get_project(&kv, subdomain).await? {
                Some(config) => json_response(&config, 200),
                None => json_response(
                    &serde_json::json!({"error": "not_found", "message": "project not found"}),
                    404,
                ),
            }
        }

        // Create a new project
        ("POST", "projects") => {
            #[derive(serde::Deserialize)]
            struct Req {
                subdomain: String,
                #[serde(default)]
                name: String,
                #[serde(default = "default_plan")]
                plan: String,
                #[serde(default)]
                owner_user_id: Option<String>,
                config: Option<crate::project::ProjectAppConfig>,
            }
            fn default_plan() -> String { "free".into() }

            let req: Req = serde_json::from_slice(body)
                .map_err(|e| Error::RustError(format!("invalid body: {e}")))?;

            // Validate subdomain
            if is_reserved_subdomain(&req.subdomain) {
                return json_response(
                    &serde_json::json!({"error": "invalid_argument", "message": "subdomain is reserved"}),
                    400,
                );
            }

            let project = provision::create_project(
                &kv, &req.subdomain, &req.name, &req.plan,
                req.owner_user_id.as_deref(), req.config,
            ).await?;
            json_response(&project, 201)
        }

        // Update a project
        ("PUT" | "PATCH", _) if path.starts_with("projects/") => {
            let subdomain = path.strip_prefix("projects/").unwrap_or("");
            let current = provision::get_project(&kv, subdomain)
                .await?
                .ok_or_else(|| Error::RustError("project not found".into()))?;

            let updates: HashMap<String, serde_json::Value> =
                serde_json::from_slice(body)
                    .map_err(|e| Error::RustError(format!("invalid body: {e}")))?;

            let mut config = current;
            if let Some(plan) = updates.get("plan").and_then(|v| v.as_str()) {
                config.plan = plan.to_string();
            }
            if let Some(status) = updates.get("status").and_then(|v| v.as_str()) {
                config.status = status.to_string();
            }
            if let Some(name) = updates.get("name").and_then(|v| v.as_str()) {
                config.name = name.to_string();
            }
            if let Some(app_config) = updates.get("config") {
                if let Ok(c) = serde_json::from_value(app_config.clone()) {
                    config.config = c;
                }
            }

            provision::update_project(&kv, subdomain, &config).await?;
            json_response(&config, 200)
        }

        // Delete a project
        ("DELETE", _) if path.starts_with("projects/") => {
            let subdomain = path.strip_prefix("projects/").unwrap_or("");
            provision::delete_project(&kv, subdomain).await?;
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
            let projects = provision::list_projects(&kv).await?;
            json_response(
                &serde_json::json!({
                    "status": "ok",
                    "project_count": projects.len(),
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

/// Constant-time byte comparison using HMAC to avoid leaking input length.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    // HMAC both inputs with a fixed key so the comparison is always
    // over two 32-byte digests, regardless of the original lengths.
    let key = b"solobase-constant-time-eq";
    let mut mac_a = Hmac::<Sha256>::new_from_slice(key).unwrap();
    mac_a.update(a);
    let hash_a = mac_a.finalize().into_bytes();

    let mut mac_b = Hmac::<Sha256>::new_from_slice(key).unwrap();
    mac_b.update(b);
    let hash_b = mac_b.finalize().into_bytes();

    let mut diff = 0u8;
    for (x, y) in hash_a.iter().zip(hash_b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
