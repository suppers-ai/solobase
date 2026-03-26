//! Solobase Dispatch Worker — thin routing layer for Workers for Platforms.
//!
//! Routes requests based on hostname:
//!
//! 1. `/_control/*` → handled directly (project CRUD, deploy, migrations)
//! 2. Platform (cloud.solobase.dev):
//!    - Static files from R2 `_site/`
//!    - API routes dispatched to "cloud" user worker
//! 3. Project ({project}.solobase.dev):
//!    - Static files from R2 `{projectId}/site/`
//!    - API routes: usage tracking → dispatched to project's user worker
//!
//! This worker has NO WAFER/block dependencies — all block execution
//! happens in user workers deployed to the dispatch namespace.

pub mod cf_api;
mod control;
mod helpers;
mod project;
mod provision;
mod schema;
mod usage;

use worker::*;

use project::is_platform_host;

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

    // 2. Control plane routes (/_control/*) — handled directly
    if pathname.starts_with("/_control/") {
        let sub_path = pathname.strip_prefix("/_control/").unwrap_or("");
        let mut req_clone = req.clone()?;
        let body = req_clone.bytes().await.unwrap_or_default();
        let resp = control::handle(&req, &env, sub_path, &body).await?;
        return add_cors_headers(resp, &req).await;
    }

    let is_platform = is_platform_host(&host);
    let is_dev = is_dev_env(&env);

    // 3. API routes → dispatch to user worker
    if pathname.starts_with("/api/") || pathname == "/api" || is_api_route(&pathname) {
        if is_platform {
            return handle_platform_api(&req, &env).await;
        } else {
            return handle_project_api(&req, &env, &host, is_dev).await;
        }
    }

    // 4. Static file serving from R2
    if is_platform {
        let serve_path = if pathname == "/" || pathname.is_empty() {
            "/blocks/dashboard/frontend/"
        } else {
            &pathname
        };
        if let Some(resp) = serve_static(&env, "_site/", serve_path).await {
            return add_security_headers(resp);
        }
    } else {
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
// Platform API — dispatch to "cloud" user worker
// ---------------------------------------------------------------------------

async fn handle_platform_api(req: &Request, env: &Env) -> Result<Response> {
    let dispatcher = env.dynamic_dispatcher("DISPATCHER")
        .map_err(|e| Error::RustError(format!("dispatcher: {e}")))?;

    let resp = dispatcher.get("cloud")
        .map_err(|e| Error::RustError(format!("dispatch get 'cloud': {e}")))?
        .fetch_request(req.clone()?)
        .await?;

    add_cors_headers(resp, req).await
}

// ---------------------------------------------------------------------------
// Project API — usage tracking + dispatch to project's user worker
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
            return add_cors_headers(resp, req).await;
        }
    };

    // Check if project is active
    if project.status == "inactive" {
        let resp = Response::ok(
            serde_json::json!({"error":"project-inactive","message":"This project is inactive. Upgrade your plan to activate it."}).to_string()
        )?.with_status(403);
        return add_cors_headers(resp, req).await;
    }

    // Usage tracking against platform DB
    let platform_db = env.d1("DB")
        .map_err(|e| Error::RustError(format!("D1: {e}")))?;
    let usage_result = usage::check_and_increment_usage(&platform_db, &project).await;

    if let Some(ref err) = usage_result.error {
        let resp = helpers::error_json("resource_exhausted", err, 429)?;
        return add_cors_headers(resp, req).await;
    }

    // Dispatch to project's user worker
    let dispatcher = env.dynamic_dispatcher("DISPATCHER")
        .map_err(|e| Error::RustError(format!("dispatcher: {e}")))?;

    let mut resp = make_mutable(
        dispatcher.get(&project.subdomain)
            .map_err(|e| Error::RustError(format!("dispatch get '{}': {e}", project.subdomain)))?
            .fetch_request(req.clone()?)
            .await?
    ).await?;

    // Add usage warning header
    if let Some(ref warning) = usage_result.warning {
        resp.headers_mut().set("X-Solobase-Warning", warning)?;
    }

    add_cors_headers(resp, req).await
}

// ---------------------------------------------------------------------------
// Static file serving from R2
// ---------------------------------------------------------------------------

async fn serve_static(env: &Env, prefix: &str, path: &str) -> Option<Response> {
    let bucket = env.bucket("STORAGE").ok()?;

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
            // HTML: no-cache so deploys take effect immediately.
            // Hashed assets (JS/CSS/wasm): cache for 1 year (filename changes on rebuild).
            if key.ends_with(".html") {
                resp.headers_mut().set("Cache-Control", "no-cache").ok()?;
            } else {
                resp.headers_mut().set("Cache-Control", "public, max-age=31536000, immutable").ok()?;
            }
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
                resp.headers_mut().set("Cache-Control", "no-cache").ok()?;
                return Some(resp);
            }
        }
    }

    // SPA fallback — only for /blocks/ paths
    if path.starts_with("/blocks/") {
        let spa_key = format!("{}index.html", prefix);
        if let Ok(Some(obj)) = bucket.get(&spa_key).execute().await {
            if let Some(body) = obj.body() {
                let bytes = body.bytes().await.ok()?;
                let resp = Response::from_bytes(bytes).ok()?;
                let mut resp = resp.with_status(200);
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

fn is_api_route(pathname: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "/health", "/nav", "/debug/",
        "/auth/", "/admin/", "/storage/",
        "/b/", "/ext/", "/profile/", "/settings/",
        "/internal/", "/_internal/",
    ];
    PREFIXES.iter().any(|p| {
        pathname == p.trim_end_matches('/') || pathname.starts_with(p)
    })
}

fn is_dev_env(env: &Env) -> bool {
    env.var("ENVIRONMENT")
        .map(|v| v.to_string() == "development")
        .unwrap_or(false)
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
        if host == "localhost" || host == "127.0.0.1" {
            return origin;
        }
        if host == "cloud.solobase.dev" || host == "solobase.dev"
            || host == "cloud.solobase-dev.dev" || host == "solobase-dev.dev"
            || host.ends_with(".solobase-dev.dev") {
            return origin;
        }
        let req_host = req.headers().get("host").ok().flatten().unwrap_or_default();
        let req_host_only = req_host.split(':').next().unwrap_or("");
        if !req_host_only.is_empty() && host == req_host_only {
            return origin;
        }
    }
    String::new()
}

async fn add_cors_headers(resp: Response, req: &Request) -> Result<Response> {
    // Responses from WfP dispatch have immutable headers, so we must
    // create a new mutable Response to add our headers.
    let mut resp = make_mutable(resp).await?;
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

/// Create a mutable copy of a response (works around immutable headers from WfP dispatch).
async fn make_mutable(mut resp: Response) -> Result<Response> {
    let status = resp.status_code();
    let orig_headers = resp.headers().clone();
    let body = resp.bytes().await.unwrap_or_default();
    let mut new_resp = Response::from_bytes(body)?.with_status(status);
    for (key, value) in orig_headers.entries() {
        let _ = new_resp.headers_mut().set(&key, &value);
    }
    Ok(new_resp)
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
