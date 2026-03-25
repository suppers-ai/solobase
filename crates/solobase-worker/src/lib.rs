//! Solobase User Worker — per-project block execution.
//!
//! Deployed to a Workers for Platforms dispatch namespace. Each instance
//! has its own D1 database binding and runs the full solobase block pipeline.
//!
//! Receives requests forwarded from the dispatch worker. Does NOT handle
//! CORS (dispatch adds those headers), usage tracking, or static files.

mod convert;
mod database;
mod helpers;
mod schema;
mod service_blocks;
mod storage;

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use worker::*;

use database::D1DatabaseService;
use storage::R2StorageService;

use solobase::blocks;
use solobase_core::features;
use solobase_core::routing::BlockId;

/// App config — mirrors solobase.json feature flags.
/// Deserialized from the PROJECT_CONFIG env var set at upload time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ProjectAppConfig {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub app: Option<Value>,
    #[serde(default)]
    pub auth: Option<Value>,
    #[serde(default)]
    pub admin: Option<Value>,
    #[serde(default)]
    pub files: Option<Value>,
    #[serde(default)]
    pub products: Option<Value>,
    #[serde(default)]
    pub deployments: Option<Value>,
    #[serde(default)]
    pub legalpages: Option<Value>,
    #[serde(default)]
    pub userportal: Option<Value>,
}

impl features::FeatureConfig for ProjectAppConfig {
    fn auth_enabled(&self) -> bool { features::is_feature_enabled(&self.auth) }
    fn admin_enabled(&self) -> bool { features::is_feature_enabled(&self.admin) }
    fn files_enabled(&self) -> bool { features::is_feature_enabled(&self.files) }
    fn products_enabled(&self) -> bool { features::is_feature_enabled(&self.products) }
    fn deployments_enabled(&self) -> bool { features::is_feature_enabled(&self.deployments) }
    fn legalpages_enabled(&self) -> bool { features::is_feature_enabled(&self.legalpages) }
    fn userportal_enabled(&self) -> bool { features::is_feature_enabled(&self.userportal) }
}

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

    helpers::json_response(&serde_json::json!({"ok": true}), 200)
}

// ---------------------------------------------------------------------------
// Block execution — sets up WAFER runtime and runs the request through it
// ---------------------------------------------------------------------------

async fn handle_request(req: &Request, env: &Env) -> Result<Response> {
    let project_id = get_env_str(env, "PROJECT_ID");
    let project_config: ProjectAppConfig = env
        .var("PROJECT_CONFIG")
        .map(|v| v.to_string())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    // Get bindings
    let db = env.d1("DB")
        .map_err(|e| Error::RustError(format!("D1: {e}")))?;
    let bucket = env.bucket("STORAGE")
        .map_err(|e| Error::RustError(format!("R2: {e}")))?;
    let jwt_secret = get_env_str(env, "JWT_SECRET");

    // Build env vars map
    let mut env_vars = HashMap::new();
    env_vars.insert("JWT_SECRET".to_string(), jwt_secret.clone());
    for key in &[
        "STRIPE_SECRET_KEY", "STRIPE_WEBHOOK_SECRET",
        "STRIPE_PRICE_STARTER", "STRIPE_PRICE_PRO",
        "MAILGUN_API_KEY", "MAILGUN_DOMAIN", "MAILGUN_FROM", "MAILGUN_REPLY_TO",
        "AUTH_REQUIRE_VERIFICATION", "AUTH_ALLOWED_EMAIL_DOMAINS",
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

    // Create WAFER runtime
    let mut wafer = wafer_run::runtime::Wafer::new();

    // Register CF service blocks
    let db_service = D1DatabaseService::new(db);
    let storage_service = R2StorageService::new(bucket, project_id.clone());
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
    let features: Arc<dyn solobase_core::FeatureConfig> = Arc::new(project_config);
    use solobase::blocks::router::{NativeBlockFactory, SolobaseRouterBlock};
    let factory = NativeBlockFactory::new(shared_blocks);
    let router = SolobaseRouterBlock::new(jwt_secret, features, factory);
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

    // Convert result to HTTP response
    convert::wafer_result_to_worker_response(result)
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
