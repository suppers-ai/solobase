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
//! - `DB` — D1 database (per-tenant data with tenant_id column)
//! - `STORAGE` — R2 bucket (per-tenant prefix)
//! - `TENANTS` — KV namespace (tenant config)
//! - `JWT_SECRET` — Secret for token signing

mod cf_context;
mod control;
mod convert;
mod database;
mod provision;
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

    // 4. Get bindings
    let db = env
        .d1("DB")
        .map_err(|e| Error::RustError(format!("D1 binding error: {e}")))?;
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
    let db_service = D1DatabaseService::new(db, tenant.id.clone());
    let storage_service = R2StorageService::new(bucket, tenant.id.clone());
    let cf_ctx = CloudflareContext::new(
        db_service,
        storage_service,
        jwt_secret.clone(),
        env_vars,
        tenant.id.clone(),
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

    // 9. Route to appropriate solobase block based on path
    let result = route_to_block(&cf_ctx, &mut msg).await;

    // 10. Convert Result_ to HTTP Response + CORS headers
    let response = convert::wafer_result_to_worker_response(result)?;
    add_cors_headers(response)
}

/// Route the message to the appropriate solobase block based on the request path.
///
/// Paths have already been stripped of the `/api` prefix by the caller.
/// Block routing mirrors the site-main flow definitions.
async fn route_to_block(ctx: &CloudflareContext, msg: &mut wafer_run::types::Message) -> wafer_run::types::Result_ {
    let path = msg.path().to_string();

    // Health / system
    if path == "/health" {
        return blocks::system::SystemBlock.handle(ctx, msg).await;
    }
    if path == "/nav" || path.starts_with("/debug/") {
        return blocks::system::SystemBlock.handle(ctx, msg).await;
    }

    // Auth: /auth/**
    if path.starts_with("/auth/") || path.starts_with("/internal/oauth/") {
        return blocks::auth::AuthBlock::new().handle(ctx, msg).await;
    }

    // Admin: /admin/** (require admin role)
    if path.starts_with("/admin/") {
        if !msg.is_admin() {
            return wafer_run::helpers::err_forbidden(msg, "admin access required");
        }
        // Settings sub-routes
        if path.starts_with("/admin/settings/") || path.starts_with("/settings/") {
            // Settings are handled by the admin block
            return blocks::admin::AdminBlock.handle(ctx, msg).await;
        }
        // Storage/files via admin
        if path.starts_with("/admin/storage/") || path.starts_with("/admin/ext/cloudstorage/") {
            return blocks::files::FilesBlock::new().handle(ctx, msg).await;
        }
        // Legal pages via admin
        if path.starts_with("/admin/legalpages/") {
            return blocks::legalpages::LegalPagesBlock.handle(ctx, msg).await;
        }
        // Products via admin
        if path.starts_with("/admin/ext/products/") || path.starts_with("/admin/ext/products") {
            return blocks::products::ProductsBlock::new().handle(ctx, msg).await;
        }
        // Deployments via admin
        if path.starts_with("/admin/ext/deployments/") || path.starts_with("/admin/ext/deployments") {
            return blocks::deployments::DeploymentsBlock::new().handle(ctx, msg).await;
        }
        // All other admin paths (users, database, logs, iam, wafer, custom-tables)
        return blocks::admin::AdminBlock.handle(ctx, msg).await;
    }

    // Storage/files: /storage/**
    if path.starts_with("/storage/") || path.starts_with("/ext/cloudstorage/") {
        return blocks::files::FilesBlock::new().handle(ctx, msg).await;
    }

    // Products: /ext/products/**
    if path.starts_with("/ext/products/") || path.starts_with("/ext/products") {
        return blocks::products::ProductsBlock::new().handle(ctx, msg).await;
    }

    // Legal pages: /ext/legalpages/**
    if path.starts_with("/ext/legalpages/") || path.starts_with("/ext/legalpages") {
        return blocks::legalpages::LegalPagesBlock.handle(ctx, msg).await;
    }

    // Deployments: /ext/deployments/**
    if path.starts_with("/ext/deployments/") || path.starts_with("/ext/deployments") {
        return blocks::deployments::DeploymentsBlock::new().handle(ctx, msg).await;
    }

    // User portal: /ext/userportal/**
    if path.starts_with("/ext/userportal/") || path.starts_with("/ext/userportal") {
        return blocks::userportal::UserPortalBlock.handle(ctx, msg).await;
    }

    // Profile: /profile/**
    if path.starts_with("/profile/") || path.starts_with("/profile") {
        return blocks::profile::ProfileBlock.handle(ctx, msg).await;
    }

    // 404
    wafer_run::helpers::err_not_found(msg, "endpoint not found")
}

/// Resolve tenant config from hostname subdomain.
async fn resolve_tenant(host: &str, env: &Env) -> std::result::Result<TenantConfig, String> {
    let subdomain = host
        .split('.')
        .next()
        .ok_or_else(|| "invalid hostname".to_string())?;

    // For localhost development, use a default tenant
    if subdomain == "localhost" || subdomain == "127" {
        return Ok(TenantConfig {
            id: "dev".to_string(),
            schema: "dev".to_string(),
            subdomain: "localhost".to_string(),
            plan: "hobby".to_string(),
            features: Default::default(),
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
