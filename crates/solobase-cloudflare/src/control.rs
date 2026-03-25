//! Control plane API — platform-level project management.
//!
//! All control plane routes are under `/_control/` and require the
//! `X-Admin-Secret` header to match the `ADMIN_SECRET` environment variable.

use std::collections::HashMap;

use worker::*;

use crate::cf_api::{self, CfCredentials};
use crate::helpers::json_response;
use crate::project::is_reserved_subdomain;
use crate::provision;

/// Handle a control plane request.
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

        // Create a new project (provisions D1 + user worker)
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
                #[serde(default)]
                config: Option<serde_json::Value>,
            }
            fn default_plan() -> String { "free".into() }

            let req: Req = serde_json::from_slice(body)
                .map_err(|e| Error::RustError(format!("invalid body: {e}")))?;

            if is_reserved_subdomain(&req.subdomain) {
                return json_response(
                    &serde_json::json!({"error": "invalid_argument", "message": "subdomain is reserved"}),
                    400,
                );
            }

            let project = provision::create_project(
                env, &kv, &req.subdomain, &req.name, &req.plan,
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
                config.config = app_config.clone();
            }

            provision::update_project(&kv, subdomain, &config).await?;
            json_response(&config, 200)
        }

        // Delete a project (cleans up D1 + user worker)
        ("DELETE", _) if path.starts_with("projects/") => {
            let subdomain = path.strip_prefix("projects/").unwrap_or("");
            provision::delete_project(env, &kv, subdomain).await?;
            json_response(&serde_json::json!({"deleted": true}), 200)
        }

        // Run platform schema migrations (subscriptions, project_usage)
        ("POST", "migrate") => {
            crate::schema::run_migrations(&db).await?;
            json_response(
                &serde_json::json!({"status": "ok", "message": "platform migrations applied"}),
                200,
            )
        }

        // Deploy updated code to all user workers (reads artifacts from R2)
        ("POST", "deploy") => {
            let creds = CfCredentials::from_env(env)?;
            let bucket = env.bucket("STORAGE")
                .map_err(|e| Error::RustError(format!("R2: {e}")))?;

            let js_module = read_r2_bytes(&bucket, "_system/worker/index.js").await?;
            let wasm_bytes = read_r2_bytes(&bucket, "_system/worker/index_bg.wasm").await?;

            let updated = cf_api::update_all_workers(&creds, &js_module, &wasm_bytes).await?;

            json_response(
                &serde_json::json!({"status": "ok", "updated": updated.len(), "workers": updated}),
                200,
            )
        }

        // Trigger migrations on all user workers
        ("POST", "migrate-workers") => {
            let projects = provision::list_projects(&kv).await?;
            let dispatcher = env.dynamic_dispatcher("DISPATCHER")
                .map_err(|e| Error::RustError(format!("dispatcher: {e}")))?;

            let mut results = Vec::new();
            for subdomain in &projects {
                let migrate_req = Request::new(
                    "https://internal/_internal/migrate",
                    Method::Post,
                )?;

                match dispatcher.get(subdomain) {
                    Ok(fetcher) => {
                        match fetcher.fetch_request(migrate_req).await {
                            Ok(resp) => results.push(serde_json::json!({
                                "project": subdomain,
                                "status": resp.status_code(),
                            })),
                            Err(e) => results.push(serde_json::json!({
                                "project": subdomain,
                                "error": e.to_string(),
                            })),
                        }
                    }
                    Err(e) => results.push(serde_json::json!({
                        "project": subdomain,
                        "error": e.to_string(),
                    })),
                }
            }

            json_response(&serde_json::json!({"status": "ok", "results": results}), 200)
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Constant-time byte comparison using SHA-256 to avoid timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    use sha2::{Sha256, Digest};

    let hash_a = Sha256::digest(a);
    let hash_b = Sha256::digest(b);

    let mut diff = 0u8;
    for (x, y) in hash_a.iter().zip(hash_b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

async fn read_r2_bytes(bucket: &Bucket, key: &str) -> Result<Vec<u8>> {
    let obj = bucket.get(key).execute().await?
        .ok_or_else(|| Error::RustError(format!("R2 object '{}' not found", key)))?;
    let body = obj.body()
        .ok_or_else(|| Error::RustError(format!("R2 object '{}' has no body", key)))?;
    body.bytes().await
}
