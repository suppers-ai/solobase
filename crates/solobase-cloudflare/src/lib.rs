//! Solobase on Cloudflare Workers — multi-tenant WAFER runtime.
//!
//! Two modes based on hostname:
//!
//! 1. Platform (cloud.solobase.dev / localhost):
//!    - Root redirects to dashboard SPA
//!    - Platform API: auth, projects, admin, billing
//!    - Control plane at /_control/*
//!    - Uses the shared DB directly (no project lookup)
//!
//! 2. Project instance ({project}.solobase.dev):
//!    - Project SPA from R2 `{projectId}/site/`
//!    - Project API: all block endpoints
//!    - Usage tracking and plan limit enforcement
//!    - Inactive projects return 403

mod billing;
mod control;
mod convert;
mod database;
mod helpers;
mod project;
mod provision;
mod schema;
mod service_blocks;
mod storage;
mod usage;

use std::collections::HashMap;
use std::sync::Arc;

use worker::*;

use database::D1DatabaseService;
use storage::R2StorageService;
use project::{ProjectConfig, ProjectAppConfig, is_platform_host};

use solobase::blocks;
use solobase_core::routing::BlockId;

/// The main Cloudflare Worker fetch handler.
#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_log!("{} {}", req.method().to_string(), req.path());

    // 1. Handle CORS preflight
    if req.method() == Method::Options {
        return cors_preflight(&req);
    }

    let url = req.url()?;
    let pathname = url.path().to_string();
    let host = req.headers().get("host")?.unwrap_or_default();
    let is_dev = is_dev_env(&env);

    // 2. Control plane routes (/_control/*) — platform admin
    if pathname.starts_with("/_control/") {
        let sub_path = pathname.strip_prefix("/_control/").unwrap_or("");
        let mut req_clone = req.clone()?;
        let body = req_clone.bytes().await.unwrap_or_default();
        let resp = control::handle(&req, &env, sub_path, &body).await?;
        return add_cors_headers(resp, &req);
    }

    let is_platform = is_platform_host(&host);

    // 3. API routes → dispatch
    if pathname.starts_with("/api/") || pathname == "/api" || is_api_route(&pathname) {
        if is_platform {
            return handle_platform_api(&req, &env, &pathname, is_dev).await;
        } else {
            return handle_project_api(&req, &env, &host, is_dev).await;
        }
    }

    // 4. Static file serving
    if is_platform {
        // Redirect cloud.solobase.dev root to dashboard
        if pathname == "/" || pathname.is_empty() {
            let dash_url = if is_dev {
                "/blocks/dashboard/frontend/"
            } else {
                "https://cloud.solobase.dev/blocks/dashboard/frontend/"
            };
            return redirect(dash_url);
        }

        // Platform: serve SPA from R2 _site/
        if let Some(resp) = serve_static(&env, "_site/", &pathname).await {
            return add_security_headers(resp);
        }
    } else {
        // Project: resolve and serve from {projectId}/site/
        let kv = env.kv("PROJECTS").map_err(|e| Error::RustError(format!("KV: {e}")))?;
        match project::resolve_project(&host, &kv, is_dev).await {
            Ok(proj) => {
                if proj.status == "inactive" {
                    return add_security_headers(Response::ok(
                        "<html><body style=\"font-family:sans-serif;display:flex;align-items:center;justify-content:center;height:100vh;margin:0\"><div style=\"text-align:center\"><h1>Project Inactive</h1><p>This project is inactive. The owner needs to upgrade their plan.</p></div></body></html>",
                    )?.with_status(403));
                }
                let prefix = format!("{}/site/", proj.id);
                if let Some(resp) = serve_static(&env, &prefix, &pathname).await {
                    return add_security_headers(resp);
                }
            }
            Err(_) => {}
        }
    }

    add_security_headers(Response::ok("Not Found")?.with_status(404))
}

// ---------------------------------------------------------------------------
// Platform API (cloud.solobase.dev) — uses shared DB, no project
// ---------------------------------------------------------------------------

async fn handle_platform_api(
    req: &Request,
    env: &Env,
    pathname: &str,
    is_dev: bool,
) -> Result<Response> {
    // Billing routes — handled directly, not through block dispatch
    let api_path = strip_api_prefix(pathname);
    if api_path.starts_with("/billing/") || api_path == "/billing" {
        let billing_path = api_path.strip_prefix("/billing/").unwrap_or("").to_string();
        let mut req_clone = req.clone()?;
        let body = req_clone.bytes().await.unwrap_or_default();
        let resp = billing::handle_billing_route(req, env, &billing_path, &body).await?;
        return add_cors_headers(resp, req);
    }

    // Platform uses the shared DB directly
    let platform_config = ProjectConfig {
        id: "platform".to_string(),
        subdomain: "cloud".to_string(),
        name: "Solobase Platform".to_string(),
        plan: "platform".to_string(),
        status: "active".to_string(),
        owner_user_id: None,
        db_id: None,
        db_binding: Some("DB".to_string()),
        config: ProjectAppConfig::all_enabled(),
        blocks: Vec::new(),
    };

    let resp = dispatch_to_blocks(req, env, &platform_config, is_dev).await?;
    add_cors_headers(resp, req)
}

// ---------------------------------------------------------------------------
// Project API ({project}.solobase.dev) — resolves project from KV
// ---------------------------------------------------------------------------

async fn handle_project_api(
    req: &Request,
    env: &Env,
    host: &str,
    is_dev: bool,
) -> Result<Response> {
    let kv = env.kv("PROJECTS").map_err(|e| Error::RustError(format!("KV: {e}")))?;

    let project = match project::resolve_project(host, &kv, is_dev).await {
        Ok(p) => p,
        Err(e) => {
            let resp = helpers::error_json("not_found", &format!("project not found: {e}"), 404)?;
            return add_cors_headers(resp, req);
        }
    };

    // Check if project is active
    if project.status == "inactive" {
        let resp = Response::ok(
            serde_json::json!({"error":"project-inactive","message":"This project is inactive. Upgrade your plan to activate it."}).to_string()
        )?.with_status(403);
        return add_cors_headers(resp, req);
    }

    // Usage tracking
    let db = env.d1(project.db_binding.as_deref().unwrap_or("DB"))
        .map_err(|e| Error::RustError(format!("D1: {e}")))?;
    let usage_result = usage::check_and_increment_usage(&db, &project).await;

    if let Some(ref err) = usage_result.error {
        let resp = helpers::error_json("resource_exhausted", err, 429)?;
        return add_cors_headers(resp, req);
    }

    let mut resp = dispatch_to_blocks(req, env, &project, is_dev).await?;

    // Add warning header if usage is high
    if let Some(ref warning) = usage_result.warning {
        resp.headers_mut().set("X-Solobase-Warning", warning)?;
    }

    add_cors_headers(resp, req)
}

// ---------------------------------------------------------------------------
// Block dispatch — creates WAFER runtime and runs the request through it
// ---------------------------------------------------------------------------

async fn dispatch_to_blocks(
    req: &Request,
    env: &Env,
    project: &ProjectConfig,
    _is_dev: bool,
) -> Result<Response> {
    // Get bindings
    let db_binding = project.db_binding.as_deref().unwrap_or("DB");
    let db = env.d1(db_binding)
        .map_err(|e| Error::RustError(format!("D1 binding '{}' error: {e}", db_binding)))?;
    let bucket = env.bucket("STORAGE")
        .map_err(|e| Error::RustError(format!("R2 binding error: {e}")))?;
    let jwt_secret = get_env_str(env, "JWT_SECRET");

    // Build env vars map
    let mut env_vars = HashMap::new();
    env_vars.insert("JWT_SECRET".to_string(), jwt_secret.clone());
    for key in &[
        "STRIPE_SECRET_KEY", "STRIPE_WEBHOOK_SECRET",
        "STRIPE_PRICE_STARTER", "STRIPE_PRICE_PRO",
        "MAILGUN_API_KEY", "MAILGUN_DOMAIN", "MAILGUN_FROM",
        "SITE_NAME", "SITE_URL", "ADMIN_EMAIL",
        "STORAGE_MAX_FILE_SIZE", "STORAGE_QUOTA_MB",
        "CONTROL_PLANE_URL", "CONTROL_PLANE_SECRET",
    ] {
        if let Ok(val) = env.var(key).map(|v| v.to_string()) {
            env_vars.insert(key.to_string(), val);
        } else if let Ok(val) = env.secret(key).map(|s| s.to_string()) {
            env_vars.insert(key.to_string(), val);
        }
    }

    // Create Wafer runtime
    let mut wafer = wafer_run::runtime::Wafer::new();

    // Register CF service blocks
    let db_service = D1DatabaseService::new(db);
    let storage_service = R2StorageService::new(bucket, project.id.clone());
    wafer.register_block("wafer-run/d1", Arc::new(service_blocks::D1Block::new(db_service)));
    wafer.register_block("wafer-run/r2", Arc::new(service_blocks::R2Block::new(storage_service)));
    wafer.register_block("wafer-run/config", Arc::new(service_blocks::ConfigBlock::new(env_vars)));
    wafer.register_block("wafer-run/crypto", Arc::new(service_blocks::CryptoBlock::new(jwt_secret.clone())));
    wafer.register_block("wafer-run/network", Arc::new(service_blocks::NetworkBlock));
    wafer.register_block("wafer-run/logger", Arc::new(service_blocks::LoggerBlock));

    // Aliases
    wafer.add_alias("wafer-run/database", "wafer-run/d1");
    wafer.add_alias("db", "wafer-run/d1");
    wafer.add_alias("wafer-run/storage", "wafer-run/r2");
    wafer.add_alias("storage", "wafer-run/r2");

    // Register pass-through flow blocks (handled at CF platform level)
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

    // Register solobase feature blocks via the router
    let auth_header = req.headers().get("authorization")?;

    let mut shared_blocks = HashMap::new();
    shared_blocks.insert(BlockId::System, Arc::new(blocks::system::SystemBlock) as Arc<dyn wafer_run::block::Block>);
    shared_blocks.insert(BlockId::Auth, Arc::new(blocks::auth::AuthBlock::new()) as Arc<dyn wafer_run::block::Block>);
    shared_blocks.insert(BlockId::Admin, Arc::new(blocks::admin::AdminBlock) as Arc<dyn wafer_run::block::Block>);
    shared_blocks.insert(BlockId::Files, Arc::new(blocks::files::FilesBlock::new()) as Arc<dyn wafer_run::block::Block>);
    shared_blocks.insert(BlockId::LegalPages, Arc::new(blocks::legalpages::LegalPagesBlock) as Arc<dyn wafer_run::block::Block>);
    shared_blocks.insert(BlockId::Products, Arc::new(blocks::products::ProductsBlock::new()) as Arc<dyn wafer_run::block::Block>);
    shared_blocks.insert(BlockId::Deployments, Arc::new(blocks::deployments::DeploymentsBlock::new()) as Arc<dyn wafer_run::block::Block>);
    shared_blocks.insert(BlockId::UserPortal, Arc::new(blocks::userportal::UserPortalBlock) as Arc<dyn wafer_run::block::Block>);
    shared_blocks.insert(BlockId::Profile, Arc::new(blocks::profile::ProfileBlock) as Arc<dyn wafer_run::block::Block>);

    // Register email block
    wafer.register_block("suppers-ai/email", Arc::new(blocks::email::EmailBlock));

    // Register the solobase router block
    let features: Arc<dyn solobase_core::FeatureConfig> = Arc::new(project.config.clone());
    use solobase::blocks::router::{NativeBlockFactory, SolobaseRouterBlock};
    let factory = NativeBlockFactory::new(shared_blocks);
    let router = SolobaseRouterBlock::new(jwt_secret, features, factory);
    wafer.register_block("suppers-ai/router", Arc::new(router));

    // Register the site-main flow
    wafer.add_flow_json(solobase::flows::site_main::JSON)
        .expect("invalid site-main flow JSON");

    // Resolve
    wafer.start_without_bind().await.map_err(|e| Error::RustError(e))?;

    // Convert HTTP request to WAFER Message
    let mut msg = convert::worker_request_to_message(req).await?;

    // Set auth header in meta for the router block
    if let Some(ref auth) = auth_header {
        msg.set_meta("http.header.authorization", auth);
    }

    // Execute flow
    let result = wafer.run("site-main", &mut msg).await;

    // Convert result to HTTP response
    convert::wafer_result_to_worker_response(result)
}

// ---------------------------------------------------------------------------
// Static file serving from R2
// ---------------------------------------------------------------------------

async fn serve_static(env: &Env, prefix: &str, path: &str) -> Option<Response> {
    let bucket = env.bucket("STORAGE").ok()?;

    // Build the R2 key
    let clean_path = path.trim_start_matches('/');
    let key = if clean_path.is_empty() || clean_path.ends_with('/') {
        format!("{}{}index.html", prefix, clean_path)
    } else {
        format!("{}{}", prefix, clean_path)
    };

    // Try the exact key
    if let Ok(Some(obj)) = bucket.get(&key).execute().await {
        if let Some(body) = obj.body() {
            let bytes = body.bytes().await.ok()?;
            let content_type = guess_content_type(&key);
            let resp = Response::from_bytes(bytes).ok()?;
            let mut resp = resp.with_status(200);
            resp.headers_mut().set("Content-Type", content_type).ok()?;
            resp.headers_mut().set("Cache-Control", "public, max-age=3600").ok()?;
            return Some(resp);
        }
    }

    // Try as directory (append /index.html)
    if !clean_path.contains('.') {
        let index_key = format!("{}{}/index.html", prefix, clean_path);
        if let Ok(Some(obj)) = bucket.get(&index_key).execute().await {
            if let Some(body) = obj.body() {
                let bytes = body.bytes().await.ok()?;
                let resp = Response::from_bytes(bytes).ok()?;
                let mut resp = resp.with_status(200);
                resp.headers_mut().set("Content-Type", "text/html; charset=utf-8").ok()?;
                return Some(resp);
            }
        }
    }

    // SPA fallback — only for /blocks/ paths (not for arbitrary routes)
    if path.starts_with("/blocks/") {
        let spa_key = format!("{}index.html", prefix);
        if let Ok(Some(obj)) = bucket.get(&spa_key).execute().await {
            if let Some(body) = obj.body() {
                let bytes = body.bytes().await.ok()?;
                let resp = Response::from_bytes(bytes).ok()?;
                let mut resp = resp.with_status(200);
                resp.headers_mut().set("Content-Type", "text/html; charset=utf-8").ok()?;
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

fn is_api_route(pathname: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "/health", "/nav", "/debug/",
        "/auth/", "/admin/", "/storage/",
        "/b/", "/ext/", "/profile/", "/settings/",
        "/internal/", "/billing/",
    ];
    PREFIXES.iter().any(|p| {
        pathname == p.trim_end_matches('/') || pathname.starts_with(p)
    })
}

fn strip_api_prefix(pathname: &str) -> String {
    if pathname.starts_with("/api") {
        let rest = &pathname[4..];
        if rest.is_empty() { "/".to_string() } else { rest.to_string() }
    } else {
        pathname.to_string()
    }
}

fn is_dev_env(env: &Env) -> bool {
    env.var("ENVIRONMENT")
        .map(|v| v.to_string() == "development")
        .unwrap_or(false)
}

fn get_env_str(env: &Env, key: &str) -> String {
    env.secret(key)
        .map(|s| s.to_string())
        .or_else(|_| env.var(key).map(|v| v.to_string()))
        .unwrap_or_default()
}

fn redirect(url: &str) -> Result<Response> {
    let resp = Response::empty()?;
    let mut resp = resp.with_status(302);
    resp.headers_mut().set("Location", url)?;
    Ok(resp)
}

// ---------------------------------------------------------------------------
// CORS
// ---------------------------------------------------------------------------

fn cors_preflight(req: &Request) -> Result<Response> {
    let origin = get_allowed_origin(req);
    let resp = Response::empty()?;
    let mut resp = resp.with_status(204);
    let headers = resp.headers_mut();
    headers.set("Access-Control-Allow-Methods", "GET, POST, PUT, PATCH, DELETE, OPTIONS")?;
    headers.set("Access-Control-Allow-Headers", "Content-Type, Authorization")?;
    headers.set("Access-Control-Max-Age", "86400")?;
    headers.set("Vary", "Origin")?;
    if !origin.is_empty() {
        headers.set("Access-Control-Allow-Origin", &origin)?;
    }
    Ok(resp)
}

fn get_allowed_origin(req: &Request) -> String {
    let origin = req.headers().get("origin").ok().flatten().unwrap_or_default();
    if origin.is_empty() {
        return String::new();
    }
    if let Ok(u) = Url::parse(&origin) {
        let host = u.host_str().unwrap_or("");
        // Localhost always allowed
        if host == "localhost" || host == "127.0.0.1" {
            return origin;
        }
        // Platform hosts
        if host == "cloud.solobase.dev" || host == "solobase.dev" {
            return origin;
        }
        // Same-origin: request's own host
        let req_host = req.headers().get("host").ok().flatten().unwrap_or_default();
        let req_host_only = req_host.split(':').next().unwrap_or("");
        if !req_host_only.is_empty() && host == req_host_only {
            return origin;
        }
    }
    String::new()
}

fn add_cors_headers(mut resp: Response, req: &Request) -> Result<Response> {
    let origin = get_allowed_origin(req);
    let headers = resp.headers_mut();
    if !origin.is_empty() {
        headers.set("Access-Control-Allow-Origin", &origin)?;
        headers.set("Vary", "Origin")?;
    }
    headers.set("X-Content-Type-Options", "nosniff")?;
    headers.set("X-Frame-Options", "DENY")?;
    Ok(resp)
}

fn add_security_headers(mut resp: Response) -> Result<Response> {
    let headers = resp.headers_mut();
    headers.set("X-Content-Type-Options", "nosniff")?;
    headers.set("X-Frame-Options", "DENY")?;
    headers.set("Strict-Transport-Security", "max-age=63072000; includeSubDomains; preload")?;
    headers.set("Referrer-Policy", "strict-origin-when-cross-origin")?;
    headers.set("Permissions-Policy", "camera=(), microphone=(), geolocation=()")?;
    Ok(resp)
}
