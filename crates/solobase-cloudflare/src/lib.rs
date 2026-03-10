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
pub mod d1_block;
mod database;
mod provision;
pub mod r2_block;
mod schema;
mod storage;
mod tenant;

use std::collections::HashMap;

use worker::*;

use cf_context::CloudflareContext;
use database::D1DatabaseService;
use storage::R2StorageService;
use tenant::TenantConfig;

use solobase::blocks;
use wafer_run::block::Block;
use wafer_run::meta::*;

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

    // Strip /api prefix from resource path — solobase blocks expect paths like /auth/login,
    // not /api/auth/login. The /api prefix is a Cloudflare adapter convention.
    let resource = msg.path().to_string();
    let stripped = resource.strip_prefix("/api").unwrap_or(&resource);
    if stripped != resource {
        msg.set_meta(META_REQ_RESOURCE, stripped);
    }

    // 8. Validate JWT and set auth meta (if Authorization header present)
    if let Some(auth_header) = req.headers().get("authorization")? {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            if let Ok(claims) = cf_context::verify_jwt_public(token, &jwt_secret) {
                if let Some(sub) = claims.get("sub").and_then(|v| v.as_str()) {
                    msg.set_meta(META_AUTH_USER_ID, sub);
                }
                if let Some(email) = claims.get("email").and_then(|v| v.as_str()) {
                    msg.set_meta(META_AUTH_USER_EMAIL, email);
                }
                // Roles: check for "roles" array or legacy "role" string
                let roles = if let Some(roles_arr) = claims.get("roles").and_then(|v| v.as_array()) {
                    roles_arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                } else if let Some(role) = claims.get("role").and_then(|v| v.as_str()) {
                    role.to_string()
                } else {
                    String::new()
                };
                msg.set_meta(META_AUTH_USER_ROLES, &roles);
            }
        }
    }

    // 9. Route to appropriate solobase block based on path + tenant features
    let result = route_to_block(&cf_ctx, &mut msg, &tenant).await;

    // 10. Convert Result_ to HTTP Response + CORS headers
    let response = convert::wafer_result_to_worker_response(result)?;
    add_cors_headers(response)
}

/// Block identifier for the routing table.
enum BlockId { System, Auth, Admin, Files, LegalPages, Products, Deployments, UserPortal, Profile }

/// Routing table entry: prefix to match, whether admin role is required, and which block to use.
const ROUTES: &[(&str, bool, BlockId)] = {
    use BlockId::*;
    &[
        // System
        ("/health",                    false, System),
        ("/nav",                       false, System),
        ("/debug/",                    false, System),
        // Auth
        ("/auth/",                     false, Auth),
        ("/internal/oauth/",           false, Auth),
        // Admin sub-routes (order matters — more specific before general)
        ("/admin/settings/",           true,  Admin),
        ("/settings/",                 true,  Admin),
        ("/admin/storage/",            true,  Files),
        ("/admin/b/cloudstorage/",   true,  Files),
        ("/admin/legalpages/",         true,  LegalPages),
        ("/admin/b/products",        true,  Products),
        ("/admin/b/deployments",     true,  Deployments),
        ("/admin/",                    true,  Admin),
        // Non-admin feature routes
        ("/storage/",                  false, Files),
        ("/b/cloudstorage/",         false, Files),
        ("/b/products",              false, Products),
        ("/b/legalpages",            false, LegalPages),
        ("/b/deployments",           false, Deployments),
        ("/b/userportal",            false, UserPortal),
        ("/profile",                   false, Profile),
    ]
};

/// Route the message to the appropriate solobase block based on the request path.
/// Feature flags from the tenant's app config are enforced — disabled features return 404.
async fn route_to_block(ctx: &CloudflareContext, msg: &mut wafer_run::types::Message, tenant: &TenantConfig) -> wafer_run::types::Result_ {
    let path = msg.path().to_string();
    let features = &tenant.config;

    for &(prefix, requires_admin, ref block_id) in ROUTES {
        let matches = path == prefix || path.starts_with(prefix);
        if !matches { continue; }

        // Check feature is enabled for this tenant
        let enabled = match block_id {
            BlockId::System | BlockId::Profile => true, // always on
            BlockId::Auth        => features.auth_enabled(),
            BlockId::Admin       => features.admin_enabled(),
            BlockId::Files       => features.files_enabled(),
            BlockId::Products    => features.products_enabled(),
            BlockId::Deployments => features.deployments_enabled(),
            BlockId::LegalPages  => features.legalpages_enabled(),
            BlockId::UserPortal  => features.userportal_enabled(),
        };
        if !enabled {
            return wafer_run::helpers::err_not_found(msg, "endpoint not found");
        }

        if requires_admin && !msg.is_admin() {
            return wafer_run::helpers::err_forbidden(msg, "admin access required");
        }

        return match block_id {
            BlockId::System      => blocks::system::SystemBlock.handle(ctx, msg).await,
            BlockId::Auth        => blocks::auth::AuthBlock::new().handle(ctx, msg).await,
            BlockId::Admin       => blocks::admin::AdminBlock.handle(ctx, msg).await,
            BlockId::Files       => blocks::files::FilesBlock::new().handle(ctx, msg).await,
            BlockId::LegalPages  => blocks::legalpages::LegalPagesBlock.handle(ctx, msg).await,
            BlockId::Products    => blocks::products::ProductsBlock::new().handle(ctx, msg).await,
            BlockId::Deployments => blocks::deployments::DeploymentsBlock::new().handle(ctx, msg).await,
            BlockId::UserPortal  => blocks::userportal::UserPortalBlock.handle(ctx, msg).await,
            BlockId::Profile     => blocks::profile::ProfileBlock.handle(ctx, msg).await,
        };
    }

    wafer_run::helpers::err_not_found(msg, "endpoint not found")
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

/// JSON error response helper.
fn error_json(code: &str, message: &str, status: u16) -> Result<Response> {
    let body = serde_json::json!({"error": code, "message": message}).to_string();
    let resp = Response::ok(body)?;
    let mut resp = resp.with_status(status);
    resp.headers_mut().set("Content-Type", "application/json")?;
    Ok(resp)
}
