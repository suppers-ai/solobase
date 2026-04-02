//! Solobase User Worker — per-project block execution.
//!
//! Deployed to a Workers for Platforms dispatch namespace. Each instance
//! has its own D1 database binding and runs the full solobase block pipeline.
//!
//! Receives requests forwarded from the dispatch worker. Runs project-level
//! middleware (CORS, security headers, readonly guard) configured per-project.
//! The dispatch worker handles platform-level concerns (plan quotas, platform CORS).
//!
//! Config is loaded from the D1 `variables` table — the single source of
//! truth for project configuration. Block enablement is in `block_settings` table.

mod config_service;
mod convert;
mod crypto_service;
mod database;
mod dispatcher;
mod helpers;
mod logger_service;
mod network_service;
mod schema;
mod storage;

use std::collections::HashMap;
use std::sync::Arc;

use worker::*;

use database::D1DatabaseService;
use storage::R2StorageService;

use solobase::blocks;
use solobase_core::features::BlockSettings;
use solobase_core::routing::BlockId;

/// Keys read from worker env bindings, overriding any D1 variables table values.
/// These are platform-level overrides that tenants cannot change.
const WORKER_BINDING_KEYS: &[&str] = &["CONTROL_PLANE_URL", "CONTROL_PLANE_SECRET"];

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let pathname = req.url()?.path().to_string();

    // Internal migration endpoint — called by dispatch worker during provisioning
    if pathname == "/_internal/migrate" && req.method() == Method::Post {
        return handle_migrate(&req, &env).await;
    }

    // All other requests: run through the block pipeline
    handle_request(&req, &env).await
}

// ---------------------------------------------------------------------------
// Schema migration handler
// ---------------------------------------------------------------------------

async fn handle_migrate(req: &Request, env: &Env) -> Result<Response> {
    let db = env
        .d1("DB")
        .map_err(|e| Error::RustError(format!("D1: {e}")))?;

    schema::run_migrations(&db).await?;

    // Ensure required secrets exist in variables table — generate if missing
    seed_secrets(&db).await?;

    // Seed default config variables (INSERT OR IGNORE — existing values preserved)
    seed_defaults(&db).await?;

    // Parse migration request body for optional config
    let body = req.clone()?.json::<serde_json::Value>().await.unwrap_or_default();

    // Enable specific blocks if requested by the dispatch worker.
    // e.g., {"enable_blocks": ["suppers-ai/projects"]} for the cloud project.
    if let Some(blocks) = body.get("enable_blocks").and_then(|v| v.as_array()) {
        for block in blocks {
            if let Some(name) = block.as_str() {
                let _ = db
                    .prepare(
                        "INSERT INTO block_settings (block_name, enabled) VALUES (?1, 1) \
                         ON CONFLICT (block_name) DO UPDATE SET enabled = 1",
                    )
                    .bind(&[name.into()])?
                    .run()
                    .await;
            }
        }
    }

    // Ensure admin_email user has admin role (create user if none exist, or just assign role)
    if let Some(admin_email) = body.get("admin_email").and_then(|v| v.as_str()) {
        if !admin_email.is_empty() {
            ensure_admin_role(&db, admin_email).await?;
        }
    }

    helpers::json_response(&serde_json::json!({"ok": true}), 200)
}

/// Generate and store required secrets in the variables table if they don't exist.
async fn seed_secrets(db: &D1Database) -> Result<()> {
    let secrets = [
        (
            "JWT_SECRET",
            "JWT Secret",
            "Secret key used to sign authentication tokens",
            "Changing this will invalidate all existing user sessions and tokens",
        ),
        (
            "PRODUCTS_WEBHOOK_SECRET",
            "Products Webhook Secret",
            "Secret key used to sign outgoing product/billing webhooks",
            "Changing this will require updating the webhook receiver",
        ),
    ];

    for (key, name, description, warning) in &secrets {
        let existing = db
            .prepare("SELECT value FROM variables WHERE key = ?1")
            .bind(&[(*key).into()])?
            .first::<serde_json::Value>(None)
            .await?;

        if existing.is_some() {
            continue;
        }

        let secret = format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
        let id = format!("var_{}", uuid::Uuid::new_v4());

        db.prepare(
            "INSERT INTO variables (id, key, name, description, value, warning, sensitive, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, datetime('now'), datetime('now'))"
        )
        .bind(&[id.into(), (*key).into(), (*name).into(), (*description).into(), secret.into(), (*warning).into()])?
        .run().await?;

        console_log!("Generated {} for project", key);
    }

    Ok(())
}

/// Seed default UI/config variables if they don't exist yet.
/// Uses INSERT OR IGNORE so existing dashboard-edited values are preserved.
async fn seed_defaults(db: &D1Database) -> Result<()> {
    // (key, name, description, default_value, warning, sensitive)
    let defaults: &[(&str, &str, &str, &str, &str, i32)] = &[
        ("APP_NAME", "App Name", "Display name shown in the UI and emails", "Solobase", "", 0),
        ("ALLOW_SIGNUP", "Allow Signup", "Allow new users to register", "true", "", 0),
        ("ENABLE_OAUTH", "Enable OAuth", "Enable third-party OAuth login", "false", "", 0),
        ("PRIMARY_COLOR", "Primary Color", "Brand color used in the UI", "#6366f1", "", 0),
        ("POST_LOGIN_REDIRECT", "Post-Login Redirect", "URL to redirect to after login", "/", "", 0),
        ("LOGO_URL", "Logo URL", "Logo shown in the header and emails", "https://solobase.dev/images/logo_long.png", "", 0),
        ("LOGO_ICON_URL", "Logo Icon URL", "Small icon logo", "https://solobase.dev/images/logo.png", "", 0),
        ("AUTH_LOGO_URL", "Auth Logo URL", "Logo shown on login/signup pages (falls back to LOGO_URL)", "", "", 0),
    ];

    for &(key, name, description, value, warning, sensitive) in defaults {
        let id = format!("var_{}", uuid::Uuid::new_v4());
        let _ = db.prepare(
            "INSERT OR IGNORE INTO variables (id, key, name, description, value, warning, sensitive, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), datetime('now'))"
        )
        .bind(&[id.into(), key.into(), name.into(), description.into(), value.into(), warning.into(), sensitive.into()])?
        .run().await;
    }

    Ok(())
}

/// Ensure the given email has the admin role. If the user exists, assign admin.
/// Also sets ADMIN_EMAIL variable so future signups with this email get admin.
async fn ensure_admin_role(db: &D1Database, admin_email: &str) -> Result<()> {
    // Set ADMIN_EMAIL variable
    let var_id = format!("var_{}", uuid::Uuid::new_v4());
    let _ = db.prepare(
        "INSERT OR IGNORE INTO variables (id, key, name, description, value, sensitive, created_at, updated_at) \
         VALUES (?1, 'ADMIN_EMAIL', 'Admin Email', 'Email that gets admin role on signup', ?2, 0, datetime('now'), datetime('now'))"
    ).bind(&[var_id.into(), admin_email.into()])?.run().await;

    // Find user by email
    let user = db
        .prepare("SELECT id FROM auth_users WHERE email = ?1")
        .bind(&[admin_email.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    if let Some(row) = user {
        let user_id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if !user_id.is_empty() {
            // Assign admin role (ignore if already assigned)
            let role_id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            let _ = db.prepare(
                "INSERT OR IGNORE INTO iam_user_roles (id, user_id, role, assigned_at) \
                 SELECT ?1, ?2, 'admin', ?3 \
                 WHERE NOT EXISTS (SELECT 1 FROM iam_user_roles WHERE user_id = ?2 AND role = 'admin')"
            ).bind(&[role_id.into(), user_id.into(), now.into()])?.run().await;
            console_log!("Admin role assigned to {}", admin_email);
        }
    } else {
        console_log!("User {} not found — admin role will be assigned on signup", admin_email);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Block execution — sets up WAFER runtime and runs the request through it
// ---------------------------------------------------------------------------

async fn handle_request(req: &Request, env: &Env) -> Result<Response> {
    // Get bindings — each project has its own D1 and R2 bucket (no prefixing needed)
    let db = env
        .d1("DB")
        .map_err(|e| Error::RustError(format!("D1: {e}")))?;
    let bucket = env
        .bucket("STORAGE")
        .map_err(|e| Error::RustError(format!("R2: {e}")))?;

    // Load all config from the D1 variables table — the single source of
    // truth for project configuration. Set via the admin dashboard.
    let mut env_vars = HashMap::new();
    if let Ok(stmt) = db.prepare("SELECT key, value FROM variables").bind(&[]) {
        if let Ok(result) = stmt.all().await {
            for row in result.results::<serde_json::Value>().unwrap_or_default() {
                if let (Some(key), Some(value)) = (
                    row.get("key").and_then(|v| v.as_str()),
                    row.get("value").and_then(|v| v.as_str()),
                ) {
                    if !key.is_empty() {
                        env_vars.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }
    }

    // Merge protected worker env bindings — these override D1 variables table values.
    // CONTROL_PLANE_* are infrastructure secrets for the cloud worker.
    for key in WORKER_BINDING_KEYS {
        let val = get_env_str(env, key);
        if !val.is_empty() {
            env_vars.insert(key.to_string(), val);
        }
    }

    // Register dispatcher service binding if available
    let has_dispatcher = if let Ok(fetcher) = env.service("DISPATCHER") {
        env_vars.insert("HAS_DISPATCHER_BINDING".to_string(), "true".to_string());
        Some(fetcher)
    } else {
        None
    };

    let jwt_secret = env_vars.get("JWT_SECRET").cloned().unwrap_or_default();

    // Block settings — read from block_settings table in D1
    let features = {
        let mut map = std::collections::HashMap::new();
        if let Ok(stmt) = db
            .prepare("SELECT block_name, enabled FROM block_settings")
            .bind(&[])
        {
            if let Ok(result) = stmt.all().await {
                for row in result.results::<serde_json::Value>().unwrap_or_default() {
                    if let (Some(name), Some(enabled)) = (
                        row.get("block_name").and_then(|v| v.as_str()),
                        row.get("enabled").and_then(|v| v.as_i64()),
                    ) {
                        map.insert(name.to_string(), enabled != 0);
                    }
                }
            }
        }
        BlockSettings::from_map(map)
    };

    // Create WAFER runtime
    let mut wafer = wafer_run::runtime::Wafer::new();

    // Register unified service blocks with CF-specific service implementations
    wafer_core::service_blocks::database::register_with(
        &mut wafer,
        Arc::new(D1DatabaseService::new(db)),
    );
    wafer.add_alias("db", "wafer-run/database");

    wafer_core::service_blocks::storage::register_with(
        &mut wafer,
        Arc::new(R2StorageService::new(bucket)),
    );
    wafer.add_alias("storage", "wafer-run/storage");

    wafer_core::service_blocks::config::register_with(
        &mut wafer,
        Arc::new(config_service::HashMapConfigService::new(env_vars)),
    );
    wafer_core::service_blocks::crypto::register_with(
        &mut wafer,
        Arc::new(crypto_service::SolobaseCryptoService::new(
            jwt_secret.clone(),
        )),
    );
    wafer_core::service_blocks::network::register_with(
        &mut wafer,
        Arc::new(network_service::WorkerFetchService),
    );
    wafer_core::service_blocks::logger::register_with(
        &mut wafer,
        Arc::new(logger_service::ConsoleLoggerService),
    );

    // Register dispatcher block for internal RPC via service binding
    if let Some(fetcher) = has_dispatcher {
        wafer.register_block(
            "solobase/dispatcher",
            Arc::new(dispatcher::DispatcherBlock::new(fetcher)),
        );
    }

    // Project-level middleware blocks — users can configure these per-project
    // (e.g. custom CORS origins, readonly mode, security headers).
    // The dispatch worker handles platform-level concerns (plan quotas, platform CORS);
    // these blocks handle project-level configuration.
    wafer_block_security_headers::register(&mut wafer);
    wafer_block_cors::register(&mut wafer);
    wafer_block_readonly_guard::register(&mut wafer);

    // The site-main flow references wafer-run/router — alias to suppers-ai/router
    wafer.add_alias("wafer-run/router", "suppers-ai/router");

    // Register wafer-run/web for SPA frontend
    wafer_block_web::register(&mut wafer);
    wafer.add_block_config(
        "wafer-run/web",
        serde_json::json!({
            "web_root": "site",
            "web_spa": "true",
            "web_index": "index.html"
        }),
    );

    // Register solobase feature blocks — only enabled blocks get instantiated.
    // Block enablement is read from the block_settings table in D1.
    let auth_header = req.headers().get("authorization")?;

    let mut shared_blocks = HashMap::new();
    // System and profile are always enabled
    shared_blocks.insert(
        BlockId::System,
        Arc::new(blocks::system::SystemBlock) as Arc<dyn wafer_run::block::Block>,
    );
    shared_blocks.insert(
        BlockId::Profile,
        Arc::new(blocks::profile::ProfileBlock) as Arc<dyn wafer_run::block::Block>,
    );
    // Feature-gated blocks
    if features.is_enabled("auth") {
        shared_blocks.insert(
            BlockId::Auth,
            Arc::new(blocks::auth::AuthBlock::new()) as Arc<dyn wafer_run::block::Block>,
        );
    }
    if features.is_enabled("admin") {
        shared_blocks.insert(
            BlockId::Admin,
            Arc::new(blocks::admin::AdminBlock) as Arc<dyn wafer_run::block::Block>,
        );
    }
    if features.is_enabled("files") {
        shared_blocks.insert(
            BlockId::Files,
            Arc::new(blocks::files::FilesBlock::new()) as Arc<dyn wafer_run::block::Block>,
        );
    }
    if features.is_enabled("products") {
        shared_blocks.insert(
            BlockId::Products,
            Arc::new(blocks::products::ProductsBlock::new()) as Arc<dyn wafer_run::block::Block>,
        );
    }
    if features.is_enabled("projects") {
        shared_blocks.insert(
            BlockId::Projects,
            Arc::new(blocks::projects::ProjectsBlock::new()) as Arc<dyn wafer_run::block::Block>,
        );
    }
    if features.is_enabled("legalpages") {
        shared_blocks.insert(
            BlockId::LegalPages,
            Arc::new(blocks::legalpages::LegalPagesBlock) as Arc<dyn wafer_run::block::Block>,
        );
    }
    if features.is_enabled("userportal") {
        shared_blocks.insert(
            BlockId::UserPortal,
            Arc::new(blocks::userportal::UserPortalBlock) as Arc<dyn wafer_run::block::Block>,
        );
    }

    // Register email block
    wafer.register_block("suppers-ai/email", Arc::new(blocks::email::EmailBlock));

    // Register the solobase router block
    let feature_config: Arc<dyn solobase_core::FeatureConfig> = Arc::new(features);
    use solobase::blocks::router::{NativeBlockFactory, SolobaseRouterBlock};
    let factory = NativeBlockFactory::new(shared_blocks);
    let router = SolobaseRouterBlock::new(jwt_secret, feature_config, factory);
    wafer.register_block("suppers-ai/router", Arc::new(router));

    // Register the site-main flow
    wafer
        .add_flow_json(solobase::flows::site_main::JSON)
        .map_err(|e| Error::RustError(format!("invalid site-main flow JSON: {e}")))?;

    // Start runtime
    wafer
        .start_without_bind()
        .await
        .map_err(|e| Error::RustError(e))?;

    // Try serving static files from R2 for non-API paths first.
    // The native wafer-run/web block uses std::fs which doesn't work on CF Workers,
    // so we handle static file serving directly from the project's R2 bucket.
    let pathname = req.url()?.path().to_string();
    if !is_api_path(&pathname) {
        let r2 = env
            .bucket("STORAGE")
            .map_err(|e| Error::RustError(format!("R2: {e}")))?;
        if let Some(static_resp) = serve_from_r2(&r2, &pathname).await {
            return Ok(static_resp);
        }
    }

    // Convert HTTP request to WAFER Message
    let mut msg = convert::worker_request_to_message(req).await?;

    // Set auth header in meta for the router block
    if let Some(ref auth) = auth_header {
        msg.set_meta("http.header.authorization", auth);
    }

    // Execute flow
    let result = wafer.run("site-main", &mut msg).await;
    convert::wafer_result_to_worker_response(result)
}

/// Serve a static file from the project's R2 bucket (site/ folder).
/// Returns None if the file doesn't exist.
async fn serve_from_r2(bucket: &worker::Bucket, path: &str) -> Option<worker::Response> {
    let clean = path.trim_start_matches('/');
    let key = if clean.is_empty() || clean.ends_with('/') {
        format!("site/{}index.html", clean)
    } else {
        format!("site/{}", clean)
    };

    // Try exact key
    if let Ok(Some(obj)) = bucket.get(&key).execute().await {
        if let Some(body) = obj.body() {
            let bytes = body.bytes().await.ok()?;
            let ct = guess_content_type(&key);
            let mut resp = worker::Response::from_bytes(bytes).ok()?.with_status(200);
            resp.headers_mut().set("Content-Type", ct).ok()?;
            if key.ends_with(".html") {
                resp.headers_mut().set("Cache-Control", "no-cache").ok()?;
            } else {
                resp.headers_mut()
                    .set("Cache-Control", "public, max-age=31536000, immutable")
                    .ok()?;
            }
            return Some(resp);
        }
    }

    // SPA fallback — serve index.html for paths without extensions
    if !clean.contains('.') {
        if let Ok(Some(obj)) = bucket.get("site/index.html").execute().await {
            if let Some(body) = obj.body() {
                let bytes = body.bytes().await.ok()?;
                let mut resp = worker::Response::from_bytes(bytes).ok()?.with_status(200);
                resp.headers_mut()
                    .set("Content-Type", "text/html; charset=utf-8")
                    .ok()?;
                resp.headers_mut().set("Cache-Control", "no-cache").ok()?;
                return Some(resp);
            }
        }
    }

    None
}

fn is_api_path(path: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "/health",
        "/nav",
        "/debug/",
        "/auth/",
        "/admin/",
        "/storage/",
        "/b/",
        "/ext/",
        "/profile/",
        "/settings/",
        "/internal/",
        "/_internal/",
    ];
    PREFIXES
        .iter()
        .any(|p| path == p.trim_end_matches('/') || path.starts_with(p))
}

fn guess_content_type(key: &str) -> &'static str {
    if key.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if key.ends_with(".js") {
        "application/javascript"
    } else if key.ends_with(".css") {
        "text/css"
    } else if key.ends_with(".json") {
        "application/json"
    } else if key.ends_with(".png") {
        "image/png"
    } else if key.ends_with(".jpg") || key.ends_with(".jpeg") {
        "image/jpeg"
    } else if key.ends_with(".svg") {
        "image/svg+xml"
    } else if key.ends_with(".ico") {
        "image/x-icon"
    } else if key.ends_with(".woff2") {
        "font/woff2"
    } else if key.ends_with(".woff") {
        "font/woff"
    } else if key.ends_with(".wasm") {
        "application/wasm"
    } else {
        "application/octet-stream"
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn get_env_str(env: &Env, key: &str) -> String {
    env.secret(key)
        .map(|s| s.to_string())
        .or_else(|_| env.var(key).map(|v| v.to_string()))
        .unwrap_or_default()
}
