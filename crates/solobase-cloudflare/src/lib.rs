//! Solobase on Cloudflare Workers — multi-tenant WAFER runtime.
//!
//! Uses the real `wafer-run` runtime (`Wafer`, flows, `RuntimeContext`) for
//! request handling. Cloudflare services (D1, R2, KV) are registered as
//! Block implementations, so `ctx.call_block("wafer-run/database", ...)`
//! routes through the standard runtime to the D1 block.
//!
//! # Architecture
//!
//! ```text
//! Request → Worker fetch()
//!   → resolve tenant from hostname (KV lookup)
//!   → create Wafer runtime with tenant's D1/R2/KV bindings
//!   → register solobase blocks + CF service blocks
//!   → register site-main flow
//!   → convert HTTP Request → WAFER Message
//!   → wafer.execute("site-main", &mut msg)
//!   → convert Result_ → HTTP Response
//! ```

mod control;
mod convert;
mod database;
mod helpers;
mod provision;
mod schema;
mod service_blocks;
mod storage;
mod tenant;

use std::collections::HashMap;
use std::sync::Arc;

use worker::*;

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

    // 5. Build env vars map
    let mut env_vars = HashMap::new();
    env_vars.insert("JWT_SECRET".to_string(), jwt_secret.clone());
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

    // 6. Create Wafer runtime with tenant's CF service blocks
    let mut wafer = wafer_run::runtime::Wafer::new();

    // Register CF service blocks (infrastructure)
    let db_service = D1DatabaseService::new(db);
    let storage_service = R2StorageService::new(bucket, tenant.id.clone());
    wafer.register_block("wafer-run/d1", Arc::new(service_blocks::D1Block::new(db_service)));
    wafer.register_block("wafer-run/r2", Arc::new(service_blocks::R2Block::new(storage_service)));
    wafer.register_block("wafer-run/config", Arc::new(service_blocks::ConfigBlock::new(env_vars)));
    wafer.register_block("wafer-run/crypto", Arc::new(service_blocks::CryptoBlock::new(jwt_secret.clone())));
    wafer.register_block("wafer-run/network", Arc::new(service_blocks::NetworkBlock));
    wafer.register_block("wafer-run/logger", Arc::new(service_blocks::LoggerBlock));

    // Aliases: solobase blocks call "wafer-run/database" and "wafer-run/storage"
    wafer.add_alias("wafer-run/database", "wafer-run/d1");
    wafer.add_alias("db", "wafer-run/d1");
    wafer.add_alias("wafer-run/storage", "wafer-run/r2");
    wafer.add_alias("storage", "wafer-run/r2");

    // Register flow middleware blocks as pass-throughs.
    // On Cloudflare, these concerns are handled at the platform level
    // (CF security headers, CF rate limiting rules, etc.).
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

    // Register wafer-run/web for serving the SPA frontend from R2
    wafer_block_web::register(&mut wafer);
    wafer.add_block_config("wafer-run/web", serde_json::json!({
        "web_root": "site",
        "web_spa": "true",
        "web_index": "index.html"
    }));

    // Register solobase feature blocks via the router block
    let auth_header = req.headers().get("authorization")?;

    // Create block instances for this request
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

    // Register the solobase router block (handles JWT validation, feature gates, block dispatch)
    let features: Arc<dyn solobase_core::FeatureConfig> =
        Arc::new(tenant.config.clone());
    use solobase::blocks::router::{NativeBlockFactory, SolobaseRouterBlock};
    let factory = NativeBlockFactory::new(shared_blocks);
    let router = SolobaseRouterBlock::new(jwt_secret, features, factory);
    wafer.register_block("suppers-ai/router", Arc::new(router));

    // Register the site-main flow (from solobase's flow definitions)
    let flow_def: wafer_run::FlowDef = serde_json::from_str(solobase::flows::site_main::JSON)
        .expect("invalid site-main flow JSON");
    wafer.add_flow_def(&flow_def);

    // Resolve (lifecycle init + flow node resolution)
    wafer.start_without_bind().await.map_err(|e| Error::RustError(e))?;

    // 7. Convert HTTP request to WAFER Message
    let mut msg = convert::worker_request_to_message(&req).await?;

    // 8. Execute the site-main flow through the Wafer runtime
    let auth_header_clone = auth_header.clone();
    // Set auth header in message meta for the router block to validate
    if let Some(ref auth) = auth_header_clone {
        msg.set_meta("http.header.authorization", auth);
    }

    let result = wafer.execute("site-main", &mut msg).await;

    // 9. Convert Result_ to HTTP Response + CORS headers
    let response = convert::wafer_result_to_worker_response(result)?;
    add_cors_headers(response)
}


/// Resolve tenant config from hostname subdomain.
async fn resolve_tenant(host: &str, env: &Env) -> std::result::Result<TenantConfig, String> {
    let host_no_port = host.split(':').next().unwrap_or(host);
    let subdomain = host_no_port
        .split('.')
        .next()
        .ok_or_else(|| "invalid hostname".to_string())?;

    if subdomain == "localhost" || subdomain == "127" {
        return Ok(TenantConfig {
            id: "dev".to_string(),
            subdomain: "localhost".to_string(),
            plan: "hobby".to_string(),
            db_id: None,
            db_binding: Some("DB".to_string()),
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

use helpers::error_json;
