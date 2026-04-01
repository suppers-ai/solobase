//! Control plane API — platform-level project management.
//!
//! All control plane routes are under `/_control/` and require the
//! `X-Control-Api-Key` header to match the `CONTROL_API_KEY` secret.

use std::collections::HashMap;

use worker::*;

use crate::cf_api::{self, CfCredentials};
use crate::helpers::json_response;
use crate::project::is_reserved_subdomain;
use crate::provision;

/// Handle a control plane request.
pub async fn handle(req: &Request, env: &Env, path: &str, body: &[u8]) -> Result<Response> {
    // Verify control plane API key
    let provided = req
        .headers()
        .get("x-control-api-key")
        .ok()
        .flatten()
        .unwrap_or_default();

    let expected = env
        .secret("CONTROL_API_KEY")
        .map(|s| s.to_string())
        .or_else(|_| env.var("CONTROL_API_KEY").map(|v| v.to_string()))
        .unwrap_or_default();

    if expected.is_empty() || !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        return json_response(
            &serde_json::json!({"error": "unauthorized", "message": "invalid control API key"}),
            401,
        );
    }

    let db = env
        .d1("DB")
        .map_err(|e| Error::RustError(format!("D1 binding: {e}")))?;

    let method = req.method().to_string();

    match (method.as_str(), path) {
        // List all projects
        ("GET", "projects") => {
            let projects = provision::list_projects(&db).await?;
            json_response(&serde_json::json!({"projects": projects}), 200)
        }

        // Get a specific project
        ("GET", _) if path.starts_with("projects/") => {
            let subdomain = path.strip_prefix("projects/").unwrap_or("");
            match provision::get_project(&db, subdomain).await? {
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
                platform: bool,
            }
            fn default_plan() -> String { "free".into() }

            let req: Req = serde_json::from_slice(body)
                .map_err(|e| Error::RustError(format!("invalid body: {e}")))?;

            // "cloud" is allowed (it's the platform project). Other reserved names are blocked.
            if req.subdomain != "cloud" && is_reserved_subdomain(&req.subdomain) {
                return json_response(
                    &serde_json::json!({"error": "invalid_argument", "message": "subdomain is reserved"}),
                    400,
                );
            }

            match provision::create_project(
                env, &db, &req.subdomain, &req.name, &req.plan,
                req.owner_user_id.as_deref(), req.platform,
            ).await {
                Ok(project) => json_response(&project, 201),
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("already exists") {
                        json_response(
                            &serde_json::json!({"error": "conflict", "message": format!("Subdomain '{}' is already taken", req.subdomain)}),
                            409,
                        )
                    } else {
                        json_response(
                            &serde_json::json!({"error": "internal", "message": msg}),
                            500,
                        )
                    }
                }
            }
        }

        // Update a project
        ("PUT" | "PATCH", _) if path.starts_with("projects/") => {
            let subdomain = path.strip_prefix("projects/").unwrap_or("");
            let current = provision::get_project(&db, subdomain)
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
            if let Some(platform) = updates.get("platform").and_then(|v| v.as_bool()) {
                config.platform = platform;
            }

            provision::update_project(&db, subdomain, &config).await?;

            // Update grace_period_end if provided
            if let Some(gpe) = updates.get("grace_period_end") {
                let gpe_val = if gpe.is_null() {
                    "".to_string()
                } else {
                    gpe.as_str().unwrap_or("").to_string()
                };
                let _ = db
                    .prepare("UPDATE projects SET grace_period_end = ?1 WHERE subdomain = ?2")
                    .bind(&[gpe_val.into(), subdomain.into()])?
                    .run()
                    .await;
            }

            json_response(&config, 200)
        }

        // Delete a project (cleans up D1 + user worker)
        ("DELETE", _) if path.starts_with("projects/") => {
            let subdomain = path.strip_prefix("projects/").unwrap_or("");
            provision::delete_project(env, &db, subdomain).await?;
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
            let projects = provision::list_projects(&db).await?;
            let dispatcher = env.dynamic_dispatcher("DISPATCHER")
                .map_err(|e| Error::RustError(format!("dispatcher: {e}")))?;

            let mut results = Vec::new();
            for subdomain in &projects {
                // Platform project (cloud) gets extra blocks enabled
                let is_platform = subdomain == "cloud";
                let body = if is_platform {
                    serde_json::json!({"enable_blocks": ["suppers-ai/projects"]}).to_string()
                } else {
                    "{}".to_string()
                };
                let mut migrate_req = Request::new_with_init(
                    "https://internal/_internal/migrate",
                    RequestInit::new()
                        .with_method(Method::Post)
                        .with_body(Some(wasm_bindgen::JsValue::from_str(&body))),
                )?;
                migrate_req.headers_mut()?.set("Content-Type", "application/json")?;

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

        // Cleanup expired inactive projects (called by external cron)
        ("POST", "cleanup-expired") => {
            // Find all inactive projects whose grace period has expired
            let now = chrono::Utc::now().to_rfc3339();
            let rows = db
                .prepare(
                    "SELECT subdomain FROM projects WHERE status = 'inactive' AND grace_period_end IS NOT NULL AND grace_period_end < ?1",
                )
                .bind(&[now.clone().into()])?
                .all()
                .await?;

            let expired: Vec<String> = rows
                .results::<serde_json::Value>()?
                .iter()
                .filter_map(|row| row.get("subdomain").and_then(|v| v.as_str().map(String::from)))
                .collect();

            let mut cleaned = Vec::new();
            for subdomain in &expired {
                // Delete CF resources (D1, R2, worker)
                match provision::delete_project(env, &db, subdomain).await {
                    Ok(()) => cleaned.push(serde_json::json!({
                        "subdomain": subdomain,
                        "status": "deleted",
                    })),
                    Err(e) => cleaned.push(serde_json::json!({
                        "subdomain": subdomain,
                        "error": e.to_string(),
                    })),
                }
            }

            json_response(
                &serde_json::json!({
                    "status": "ok",
                    "cleaned": cleaned.len(),
                    "projects": cleaned,
                }),
                200,
            )
        }

        // Platform health
        ("GET", "health") => {
            let projects = provision::list_projects(&db).await?;
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
