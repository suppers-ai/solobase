//! Solobase User Worker — per-project block execution.
//!
//! Deployed to a Workers for Platforms dispatch namespace. Each instance
//! has its own D1 database binding and runs the full solobase block pipeline.
//!
//! Receives requests forwarded from the dispatch worker. Does NOT handle
//! CORS (dispatch adds those headers), usage tracking, or static files.
//!
//! Config is loaded from the D1 `variables` table — the single source of
//! truth for project configuration. Feature flags use `FEATURE_*` variables.

mod convert;
mod database;
mod helpers;
mod schema;
mod service_blocks;
mod storage;

use std::collections::HashMap;
use std::sync::Arc;

use worker::*;

use database::D1DatabaseService;
use storage::R2StorageService;

use solobase::app_config::FeatureSnapshot;
use solobase::blocks;
use solobase_core::routing::BlockId;

/// Keys read from worker env bindings, overriding any D1 variables table values.
/// Only platform-specific vars are protected — regular features (auth, admin,
/// files, etc.) are user-configurable via the dashboard.
const WORKER_BINDING_KEYS: &[&str] = &[
    "FEATURE_PROJECTS",
    "CONTROL_PLANE_URL", "CONTROL_PLANE_SECRET",
];

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let pathname = req.url()?.path().to_string();

    // Internal migration endpoint — called by dispatch worker during provisioning
    if pathname == "/_internal/migrate" && req.method() == Method::Post {
        return handle_migrate(&env).await;
    }

    // All other requests: run through the block pipeline
    handle_request(&req, &env).await
}

// ---------------------------------------------------------------------------
// Schema migration handler
// ---------------------------------------------------------------------------

async fn handle_migrate(env: &Env) -> Result<Response> {
    let db = env.d1("DB")
        .map_err(|e| Error::RustError(format!("D1: {e}")))?;

    schema::run_migrations(&db).await?;

    // Ensure required secrets exist in variables table — generate if missing
    seed_secrets(&db).await?;

    helpers::json_response(&serde_json::json!({"ok": true}), 200)
}

/// Generate and store required secrets in the variables table if they don't exist.
async fn seed_secrets(db: &D1Database) -> Result<()> {
    let secrets = [
        ("JWT_SECRET", "JWT Secret", "Secret key used to sign authentication tokens",
         "Changing this will invalidate all existing user sessions and tokens"),
        ("PRODUCTS_WEBHOOK_SECRET", "Products Webhook Secret", "Secret key used to sign outgoing product/billing webhooks",
         "Changing this will require updating the webhook receiver"),
    ];

    for (key, name, description, warning) in &secrets {
        let existing = db
            .prepare("SELECT value FROM variables WHERE key = ?1")
            .bind(&[(*key).into()])?.first::<serde_json::Value>(None).await?;

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

// ---------------------------------------------------------------------------
// Block execution — sets up WAFER runtime and runs the request through it
// ---------------------------------------------------------------------------

async fn handle_request(req: &Request, env: &Env) -> Result<Response> {
    // Get bindings — each project has its own D1 and R2 bucket (no prefixing needed)
    let db = env.d1("DB")
        .map_err(|e| Error::RustError(format!("D1: {e}")))?;
    let bucket = env.bucket("STORAGE")
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
    // FEATURE_PROJECTS is platform-only (prevents tenants from enabling provisioning).
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

    // Feature flags — read FEATURE_* variables, default to all enabled
    let features = FeatureSnapshot::from_vars(&env_vars);

    // Create WAFER runtime
    let mut wafer = wafer_run::runtime::Wafer::new();

    // Register CF service blocks
    let db_service = D1DatabaseService::new(db);
    let storage_service = R2StorageService::new(bucket);
    wafer.register_block("wafer-run/d1", Arc::new(service_blocks::D1Block::new(db_service)));
    wafer.register_block("wafer-run/r2", Arc::new(service_blocks::R2Block::new(storage_service)));
    wafer.register_block("wafer-run/config", Arc::new(service_blocks::ConfigBlock::new(env_vars)));
    wafer.register_block("wafer-run/crypto", Arc::new(service_blocks::CryptoBlock::new(jwt_secret.clone())));
    wafer.register_block("wafer-run/network", Arc::new(service_blocks::NetworkBlock));
    wafer.register_block("wafer-run/logger", Arc::new(service_blocks::LoggerBlock));

    // Register dispatcher block for internal RPC via service binding
    if let Some(fetcher) = has_dispatcher {
        wafer.register_block("solobase/dispatcher", Arc::new(service_blocks::DispatcherBlock::new(fetcher)));
    }

    // Aliases
    wafer.add_alias("wafer-run/database", "wafer-run/d1");
    wafer.add_alias("db", "wafer-run/d1");
    wafer.add_alias("wafer-run/storage", "wafer-run/r2");
    wafer.add_alias("storage", "wafer-run/r2");

    // Pass-through flow blocks (handled at dispatch/platform level)
    for name in &[
        "wafer-run/security-headers",
        "wafer-run/cors",
        "wafer-run/readonly-guard",
        "wafer-run/ip-rate-limit",
        "wafer-run/monitoring",
    ] {
        wafer.register_block_func(*name, |_ctx, msg| {
            wafer_run::Result_::continue_with(msg.clone())
        });
    }

    // The site-main flow references wafer-run/router — alias to suppers-ai/router
    wafer.add_alias("wafer-run/router", "suppers-ai/router");

    // Register wafer-run/web for SPA frontend
    wafer_block_web::register(&mut wafer);
    wafer.add_block_config("wafer-run/web", serde_json::json!({
        "web_root": "site",
        "web_spa": "true",
        "web_index": "index.html"
    }));

    // Register solobase feature blocks — only enabled features get instantiated.
    // Most features are user-configurable via the dashboard. FEATURE_PROJECTS
    // is protected via worker env binding (only the cloud platform gets it).
    let auth_header = req.headers().get("authorization")?;

    let mut shared_blocks = HashMap::new();
    // System and profile are always enabled
    shared_blocks.insert(BlockId::System, Arc::new(blocks::system::SystemBlock) as Arc<dyn wafer_run::block::Block>);
    shared_blocks.insert(BlockId::Profile, Arc::new(blocks::profile::ProfileBlock) as Arc<dyn wafer_run::block::Block>);
    // Feature-gated blocks
    if features.is_enabled("auth") {
        shared_blocks.insert(BlockId::Auth, Arc::new(blocks::auth::AuthBlock::new()) as Arc<dyn wafer_run::block::Block>);
    }
    if features.is_enabled("admin") {
        shared_blocks.insert(BlockId::Admin, Arc::new(blocks::admin::AdminBlock) as Arc<dyn wafer_run::block::Block>);
    }
    if features.is_enabled("files") {
        shared_blocks.insert(BlockId::Files, Arc::new(blocks::files::FilesBlock::new()) as Arc<dyn wafer_run::block::Block>);
    }
    if features.is_enabled("products") {
        shared_blocks.insert(BlockId::Products, Arc::new(blocks::products::ProductsBlock::new()) as Arc<dyn wafer_run::block::Block>);
    }
    if features.is_enabled("projects") {
        shared_blocks.insert(BlockId::Projects, Arc::new(blocks::projects::ProjectsBlock::new()) as Arc<dyn wafer_run::block::Block>);
    }
    if features.is_enabled("legalpages") {
        shared_blocks.insert(BlockId::LegalPages, Arc::new(blocks::legalpages::LegalPagesBlock) as Arc<dyn wafer_run::block::Block>);
    }
    if features.is_enabled("userportal") {
        shared_blocks.insert(BlockId::UserPortal, Arc::new(blocks::userportal::UserPortalBlock) as Arc<dyn wafer_run::block::Block>);
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
    wafer.add_flow_json(solobase::flows::site_main::JSON)
        .expect("invalid site-main flow JSON");

    // Start runtime
    wafer.start_without_bind().await.map_err(|e| Error::RustError(e))?;

    // Convert HTTP request to WAFER Message
    let mut msg = convert::worker_request_to_message(req).await?;

    // Set auth header in meta for the router block
    if let Some(ref auth) = auth_header {
        msg.set_meta("http.header.authorization", auth);
    }

    // Execute flow
    let result = wafer.run("site-main", &mut msg).await;

    // If the flow returned a 404 for a non-API path, try serving a static file from R2.
    // The native wafer-run/web block uses std::fs which doesn't work on CF Workers,
    // so we handle static file serving directly from the project's R2 bucket.
    let response = convert::wafer_result_to_worker_response(result)?;
    if response.status_code() == 404 {
        let path = req.url()?.path().to_string();
        let r2 = env.bucket("STORAGE")
            .map_err(|e| Error::RustError(format!("R2: {e}")))?;
        if let Some(static_resp) = serve_from_r2(&r2, &path).await {
            return Ok(static_resp);
        }
    }
    Ok(response)
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
                resp.headers_mut().set("Cache-Control", "public, max-age=31536000, immutable").ok()?;
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
                resp.headers_mut().set("Content-Type", "text/html; charset=utf-8").ok()?;
                resp.headers_mut().set("Cache-Control", "no-cache").ok()?;
                return Some(resp);
            }
        }
    }

    None
}

fn guess_content_type(key: &str) -> &'static str {
    if key.ends_with(".html") { "text/html; charset=utf-8" }
    else if key.ends_with(".js") { "application/javascript" }
    else if key.ends_with(".css") { "text/css" }
    else if key.ends_with(".json") { "application/json" }
    else if key.ends_with(".png") { "image/png" }
    else if key.ends_with(".jpg") || key.ends_with(".jpeg") { "image/jpeg" }
    else if key.ends_with(".svg") { "image/svg+xml" }
    else if key.ends_with(".ico") { "image/x-icon" }
    else if key.ends_with(".woff2") { "font/woff2" }
    else if key.ends_with(".woff") { "font/woff" }
    else if key.ends_with(".wasm") { "application/wasm" }
    else { "application/octet-stream" }
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
