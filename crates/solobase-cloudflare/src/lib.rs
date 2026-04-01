//! Solobase Dispatch Worker — thin routing layer for Workers for Platforms.
//!
//! Routes requests based on hostname:
//!
//! 1. `/_control/*`, `/_webhooks/*` → handled directly
//! 2. Platform (cloud.solobase.dev) → all requests dispatched to "cloud" user worker
//!    (the cloud worker serves its own frontend via `wafer-run/web`)
//! 3. Project ({project}.solobase.dev) → all requests dispatched to project's user worker
//!    (each project has its own D1 + R2 bucket, user worker serves its own frontend)
//!
//! This worker has NO WAFER/block dependencies — all block execution
//! happens in user workers deployed to the dispatch namespace.

pub mod cf_api;
mod control;
mod helpers;
mod project;
mod provision;
mod schema;
mod storage_sync;
mod usage;
mod webhooks;

use worker::*;

use project::is_platform_host;

/// The main Cloudflare Worker fetch handler.
#[event(fetch)]
pub async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    console_log!("{} {}", req.method().to_string(), req.path());

    // 1. Handle CORS preflight
    if req.method() == Method::Options {
        return cors_preflight(&req);
    }

    let url = req.url()?;
    let pathname = url.path().to_string();
    let host = req.headers().get("host")?.unwrap_or_default();

    // 2. Control plane routes (/_control/*) — handled directly
    if pathname == "/_control" || pathname.starts_with("/_control/") {
        let sub_path = pathname.strip_prefix("/_control/").unwrap_or("");
        let mut req_clone = req.clone()?;
        let body = req_clone.bytes().await.unwrap_or_default();
        let resp = control::handle(&req, &env, sub_path, &body).await?;
        return add_cors_headers(resp, &req).await;
    }

    // 2b. Block internal paths — never dispatch to user workers
    if pathname == "/_internal" || pathname.starts_with("/_internal/") {
        let resp = helpers::error_json("forbidden", "internal endpoints are not externally accessible", 403)?;
        return add_cors_headers(resp, &req).await;
    }

    // 2c. Webhook routes (/_webhooks/*) — handled directly
    if pathname == "/_webhooks" || pathname.starts_with("/_webhooks/") {
        let sub_path = pathname.strip_prefix("/_webhooks/").unwrap_or("");
        let mut req_clone = req.clone()?;
        let body = req_clone.bytes().await.unwrap_or_default();
        let resp = webhooks::handle(&req, &env, sub_path, &body).await?;
        return add_cors_headers(resp, &req).await;
    }

    let is_dev = is_dev_env(&env);
    let is_platform = is_platform_host(&host, is_dev);

    // 3. Platform (cloud.solobase.dev) — dispatch ALL requests to "cloud" user worker.
    //    The cloud worker serves its own frontend via wafer-run/web from R2 {projectId}/site/.
    if is_platform {
        return handle_platform_request(&req, &env).await;
    }

    // 4. Project ({project}.solobase.dev) — dispatch ALL requests to user worker.
    //    Each project has its own R2 bucket, so the user worker's wafer-run/web
    //    block serves static files directly from the project's bucket.

    // Validate that the host matches an expected domain pattern (prevent arbitrary domain routing)
    let host_no_port = host.split(':').next().unwrap_or(&host);
    if !is_dev {
        let valid_host = host_no_port.ends_with(".solobase.dev")
            || host_no_port.ends_with(".solobase-dev.dev")
            || host_no_port == "solobase.dev"
            || host_no_port == "solobase-dev.dev";
        if !valid_host {
            let resp = helpers::error_json("bad_request", "unrecognized host", 400)?;
            return add_cors_headers(resp, &req).await;
        }
    }

    return handle_project_request(&req, &env, &ctx, &host, is_dev).await;
}

/// Scheduled (cron) handler — syncs R2 and D1 storage usage from the CF API.
///
/// Configure in wrangler.toml:
/// ```toml
/// [triggers]
/// crons = ["0 * * * *"]  # every hour
/// ```
#[event(scheduled)]
pub async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    console_log!("scheduled: starting storage usage sync");

    // Run schema migrations first (ensures d1_bytes column exists)
    if let Ok(db) = env.d1("DB") {
        let _ = schema::run_migrations(&db).await;
    }

    if let Err(e) = storage_sync::sync_all(&env).await {
        console_log!("scheduled: storage sync failed: {e}");
    }
}

// ---------------------------------------------------------------------------
// Platform — dispatch all requests to "cloud" user worker
// ---------------------------------------------------------------------------

async fn handle_platform_request(req: &Request, env: &Env) -> Result<Response> {
    let dispatcher = env.dynamic_dispatcher("DISPATCHER")
        .map_err(|e| Error::RustError(format!("dispatcher: {e}")))?;

    let resp = dispatcher.get("cloud")
        .map_err(|e| Error::RustError(format!("dispatch get 'cloud': {e}")))?
        .fetch_request(req.clone()?)
        .await?;

    add_cors_headers(resp, req).await
}

// ---------------------------------------------------------------------------
// Project — usage tracking + dispatch all requests to project's user worker
// ---------------------------------------------------------------------------

async fn handle_project_request(
    req: &Request,
    env: &Env,
    ctx: &Context,
    host: &str,
    is_dev: bool,
) -> Result<Response> {
    let db = env.d1("DB").map_err(|e| Error::RustError(format!("D1: {e}")))?;

    let project = match project::resolve_project(host, &db, is_dev).await {
        Ok(p) => p,
        Err(e) => {
            console_log!("project resolution failed for host '{}': {}", host, e);
            let resp = helpers::error_json("not_found", "project not found", 404)?;
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

    // Usage tracking: check limits (blocking read), increment counter (non-blocking write)
    let usage_result = usage::check_usage(&db, &project).await;

    if let Some(ref err) = usage_result.error {
        let resp = helpers::error_json("resource_exhausted", err, 429)?;
        return add_cors_headers(resp, req).await;
    }

    // Increment usage counter after response via waitUntil (non-blocking)
    let project_id = project.id.clone();
    let increment_db = env.d1("DB").map_err(|e| Error::RustError(format!("D1: {e}")))?;
    ctx.wait_until(async move {
        usage::increment_usage(&increment_db, &project_id).await;
    });

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
// Helpers
// ---------------------------------------------------------------------------

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
    headers.set("Strict-Transport-Security", "max-age=63072000; includeSubDomains; preload")?;
    headers.set("Referrer-Policy", "strict-origin-when-cross-origin")?;
    headers.set("Permissions-Policy", "camera=(), microphone=(), geolocation=()")?;
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
