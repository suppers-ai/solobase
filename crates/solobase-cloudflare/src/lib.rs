//! Solobase on Cloudflare Workers — multi-tenant WAFER runtime adapter.
//!
//! This crate implements a Cloudflare Worker that serves the Solobase API
//! for all tenants. Each request is routed to the appropriate tenant,
//! a CloudflareContext is created with D1/R2/KV bindings, and the
//! **same solobase blocks** used by the standalone binary handle the request.
//!
//! # Architecture
//!
//! ```text
//! Request → Worker fetch()
//!   → resolve tenant from hostname (KV lookup)
//!   → create CloudflareContext (D1, R2, KV, JWT)
//!   → convert HTTP Request → WAFER Message
//!   → validate JWT → set auth.* meta
//!   → instantiate solobase block for request path
//!   → block.handle(&cf_ctx, &mut msg) → Result_
//!   → convert Result_ → HTTP Response
//! ```
//!
//! # Bindings
//!
//! - `DB` — Default D1 database (fallback for dev/localhost)
//! - `DB_{subdomain}` — Per-tenant D1 databases (one per tenant)
//! - `STORAGE` — R2 bucket (per-tenant prefix)
//! - `TENANTS` — KV namespace (tenant config)
//! - `JWT_SECRET` — Secret for token signing

mod cf_context;
mod control;
mod convert;
mod database;
mod helpers;
mod provision;
mod schema;
mod storage;
mod tenant;

use std::collections::HashMap;
use std::sync::Arc;

use worker::*;

use cf_context::CloudflareContext;
use database::D1DatabaseService;
use storage::R2StorageService;
use tenant::TenantConfig;

use solobase::blocks;
use solobase_core::routing::BlockId;

/// The main Cloudflare Worker fetch handler.
#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_log!("{} {}", req.method().to_string(), req.path());

    // 1. Handle CORS preflight
    if req.method() == Method::Options {
        return cors_preflight();
    }

    // 2. Handle control plane requests (platform admin, not per-tenant)
    let path = req.path();
    if path.starts_with("/_control/") {
        let sub_path = path.strip_prefix("/_control/").unwrap_or("");
        let mut req_clone = req.clone()?;
        let body = req_clone.bytes().await.unwrap_or_default();
        return control::handle(&req, &env, sub_path, &body).await;
    }

    // 3. Resolve tenant from hostname
    let host = req.headers().get("host")?.unwrap_or_default();
    let tenant = match resolve_tenant(&host, &env).await {
        Ok(t) => t,
        Err(e) => {
            return error_json("not_found", &format!("tenant not found: {e}"), 404);
        }
    };

    // 4. Get bindings — resolve tenant's own D1 database, fall back to shared DB
    let db_binding = tenant.db_binding.as_deref().unwrap_or("DB");
    let db = env
        .d1(db_binding)
        .map_err(|e| Error::RustError(format!("D1 binding '{}' error: {e}", db_binding)))?;
    let bucket = env
        .bucket("STORAGE")
        .map_err(|e| Error::RustError(format!("R2 binding error: {e}")))?;
    let jwt_secret = env
        .secret("JWT_SECRET")
        .map(|s| s.to_string())
        .unwrap_or_else(|_| {
            env.var("JWT_SECRET")
                .map(|v| v.to_string())
                .unwrap_or_default()
        });

    // 5. Build env vars map for CloudflareContext config
    let mut env_vars = HashMap::new();
    env_vars.insert("JWT_SECRET".to_string(), jwt_secret.clone());
    // Copy known config keys from env
    for key in &[
        "STRIPE_SECRET_KEY", "STRIPE_WEBHOOK_SECRET", "STRIPE_PRICE_ID",
        "SITE_NAME", "SITE_URL", "ADMIN_EMAIL",
        "STORAGE_MAX_FILE_SIZE", "STORAGE_QUOTA_MB",
        "CONTROL_PLANE_URL", "CONTROL_PLANE_SECRET",
    ] {
        if let Ok(val) = env.var(key).map(|v| v.to_string()) {
            env_vars.insert(key.to_string(), val);
        }
    }

    // 6. Create tenant-scoped services and CloudflareContext
    let db_service = D1DatabaseService::new(db);
    let storage_service = R2StorageService::new(bucket, tenant.id.clone());
    let cf_ctx = CloudflareContext::new(
        db_service,
        storage_service,
        jwt_secret.clone(),
        env_vars,
    );

    // 7. Convert HTTP request to WAFER Message
    let mut msg = convert::worker_request_to_message(&req).await?;

    // 8. Shared pipeline: strip /api prefix, validate JWT, route to block
    let auth_header = req.headers().get("authorization")?;
    let factory = SolobaseBlockFactory;
    let result = solobase_core::handle_request(
        &cf_ctx,
        &mut msg,
        auth_header.as_deref(),
        &jwt_secret,
        &tenant.config,
        &factory,
    )
    .await;

    // 10. Convert Result_ to HTTP Response + CORS headers
    let response = convert::wafer_result_to_worker_response(result)?;
    add_cors_headers(response)
}

// ---------------------------------------------------------------------------
// Block factory — creates solobase block instances for the router
// ---------------------------------------------------------------------------

struct SolobaseBlockFactory;

impl solobase_core::BlockFactory for SolobaseBlockFactory {
    /// Create a fresh block instance for each request.
    ///
    /// NOTE: Blocks like Auth, Files, Products, and Deployments contain in-memory
    /// `UserRateLimiter` instances. On Cloudflare Workers, each request gets a new
    /// block instance, so these rate limiters never accumulate counts and are effectively
    /// no-ops. This is intentional — the wasm32 build of `UserRateLimiter::check()` always
    /// returns `Ok` (see `rate_limit.rs`). Per-request rate limiting on CF should be handled
    /// at the platform level (e.g. Cloudflare Rate Limiting rules).
    fn create(&self, block_id: BlockId) -> Arc<dyn wafer_run::block::Block> {
        match block_id {
            BlockId::System      => Arc::new(blocks::system::SystemBlock),
            BlockId::Auth        => Arc::new(blocks::auth::AuthBlock::new()),
            BlockId::Admin       => Arc::new(blocks::admin::AdminBlock),
            BlockId::Files       => Arc::new(blocks::files::FilesBlock::new()),
            BlockId::LegalPages  => Arc::new(blocks::legalpages::LegalPagesBlock),
            BlockId::Products    => Arc::new(blocks::products::ProductsBlock::new()),
            BlockId::Deployments => Arc::new(blocks::deployments::DeploymentsBlock::new()),
            BlockId::UserPortal  => Arc::new(blocks::userportal::UserPortalBlock),
            BlockId::Profile     => Arc::new(blocks::profile::ProfileBlock),
        }
    }
}

/// Resolve tenant config from hostname subdomain.
async fn resolve_tenant(host: &str, env: &Env) -> std::result::Result<TenantConfig, String> {
    let subdomain = host
        .split('.')
        .next()
        .ok_or_else(|| "invalid hostname".to_string())?;

    // For localhost development, use a default tenant with all features enabled
    if subdomain == "localhost" || subdomain == "127" {
        return Ok(TenantConfig {
            id: "dev".to_string(),
            subdomain: "localhost".to_string(),
            plan: "hobby".to_string(),
            db_id: None,
            db_binding: Some("DB".to_string()), // use shared DB binding for dev
            config: tenant::TenantAppConfig::all_enabled(),
            blocks: Vec::new(),
        });
    }

    let kv = env
        .kv("TENANTS")
        .map_err(|e| format!("KV binding error: {e}"))?;

    let key = format!("tenant:{}:config", subdomain);
    let config = kv
        .get(&key)
        .json::<TenantConfig>()
        .await
        .map_err(|e| format!("KV get error: {e}"))?
        .ok_or_else(|| format!("tenant '{}' not found", subdomain))?;

    Ok(config)
}

/// Return a CORS preflight response.
fn cors_preflight() -> Result<Response> {
    let resp = Response::empty()?;
    let mut resp = resp.with_status(204);
    let headers = resp.headers_mut();
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set("Access-Control-Allow-Methods", "GET, POST, PUT, PATCH, DELETE, OPTIONS")?;
    headers.set("Access-Control-Allow-Headers", "Content-Type, Authorization")?;
    headers.set("Access-Control-Max-Age", "86400")?;
    Ok(resp)
}

/// Add CORS headers to a response.
fn add_cors_headers(mut resp: Response) -> Result<Response> {
    let headers = resp.headers_mut();
    headers.set("Access-Control-Allow-Origin", "*")?;
    Ok(resp)
}

// Use shared JSON error helper from helpers module.
use helpers::error_json;
