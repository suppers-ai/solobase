//! Cloudflare Workers adapter for solobase: D1 database service, R2 storage
//! service, wasm-compatible crypto/network services, and worker entry helpers.
//!
//! Consumed by:
//! - `solobase-cloud`'s `solobase-worker` (multi-tenant dispatch user worker).
//! - The `solobase build --target cloudflare` flow (single-worker consumers
//!   like wafer-site).
//!
//! This crate is wasm-only; building for native targets is not supported.

mod config_cache;
pub mod config_service;
pub mod convert;
pub mod crypto_service;
pub mod database;
pub mod helpers;
pub mod logger_service;
pub mod network_service;
mod runner;
pub mod storage;

// ---------------------------------------------------------------------------
// Public `make_*` constructors — mirrors `solobase-native`'s API surface.
// Consumers construct services through these helpers rather than importing
// internal types directly.
// ---------------------------------------------------------------------------

use std::{collections::HashMap, sync::Arc};

use wafer_core::interfaces::{
    config::service::ConfigService, crypto::service::CryptoService,
    database::service::DatabaseService, logger::service::LoggerService,
    network::service::NetworkService, storage::service::StorageService,
};

/// Construct a D1-backed [`DatabaseService`] from a worker `Env` and the D1
/// binding name.
///
/// The binding name must match a `[[d1_databases]]` entry in the consumer's
/// `wrangler.toml` (e.g. `"DB"`).
pub fn make_d1_database_service(
    env: &worker::Env,
    binding: &str,
) -> Result<Arc<dyn DatabaseService>, worker::Error> {
    let db = env.d1(binding)?;
    Ok(Arc::new(database::D1DatabaseService::new(db)))
}

/// Construct an R2-backed [`StorageService`] from a worker `Env` and the R2
/// bucket binding name.
///
/// The binding name must match a `[[r2_buckets]]` entry in the consumer's
/// `wrangler.toml` (e.g. `"STORAGE"`).
pub fn make_r2_storage_service(
    env: &worker::Env,
    binding: &str,
) -> Result<Arc<dyn StorageService>, worker::Error> {
    let bucket = env.bucket(binding)?;
    Ok(Arc::new(storage::R2StorageService::new(bucket)))
}

/// Construct a wasm-compatible [`CryptoService`] (HMAC + SHA-256 via
/// pure-Rust crates).
///
/// `jwt_secret` is the HMAC secret used to sign and verify JWTs.
pub fn make_jwt_crypto_service(jwt_secret: String) -> Arc<dyn CryptoService> {
    Arc::new(crypto_service::SolobaseCryptoService::new(jwt_secret))
}

/// Construct a [`NetworkService`] backed by the CF Worker global `fetch` API.
pub fn make_fetch_network_service() -> Arc<dyn NetworkService> {
    Arc::new(network_service::WorkerFetchService)
}

/// Construct a [`LoggerService`] that writes to `worker::console_log`.
pub fn make_console_logger() -> Arc<dyn LoggerService> {
    Arc::new(logger_service::ConsoleLoggerService)
}

/// Construct a [`ConfigService`] from a pre-loaded key/value map.
///
/// In a CF Worker, callers typically load variables from the D1 `variables`
/// table (and merge any protected worker env bindings) before calling this
/// function.  The returned service is read-only; `set()` is a no-op because
/// CF Workers are stateless.
pub fn make_config_service(vars: HashMap<String, String>) -> Arc<dyn ConfigService> {
    Arc::new(config_service::HashMapConfigService::new(vars))
}

use solobase_core::builder::SolobaseBuilder;

/// Worker entry shim: load D1 vars, wire services, run the consumer's
/// block registrations, dispatch the request through WAFER.
///
/// Two consumer hooks:
/// - `register_blocks` runs against the `SolobaseBuilder` after the 6
///   services are attached and before `builder.build()`. Use builder
///   methods (`extra_block`, `add_route`, `block_config`).
/// - `register_post_build` runs against `&mut Wafer` after build and
///   before start, and additionally receives the configured R2-backed
///   `StorageService` so consumers can register blocks that need direct
///   (un-namespaced) access to the bucket — for example, a static
///   asset-serving block that reads a fixed key prefix uploaded by
///   `solobase deploy --target cloudflare`.
///
/// Binding names are hardcoded: D1 = `"DB"`, R2 = `"STORAGE"`. Consumers'
/// `wrangler.toml` must use these names.
///
/// On error in any step, returns a 500 response with the error message.
/// The error is also logged via `worker::console_log!`.
pub async fn run<F, G>(
    req: worker::Request,
    env: worker::Env,
    _ctx: worker::Context,
    register_blocks: F,
    register_post_build: G,
) -> worker::Result<worker::Response>
where
    F: FnOnce(SolobaseBuilder) -> Result<SolobaseBuilder, Box<dyn std::error::Error>>,
    G: FnOnce(
        &mut wafer_run::Wafer,
        Arc<dyn StorageService>,
    ) -> Result<(), Box<dyn std::error::Error>>,
{
    match run_inner(req, env, register_blocks, register_post_build).await {
        Ok(response) => Ok(response),
        Err(e) => {
            worker::console_log!("solobase-cloudflare run error: {e}");
            worker::Response::error(format!("solobase: {e}"), 500)
        }
    }
}

async fn run_inner<F, G>(
    req: worker::Request,
    env: worker::Env,
    register_blocks: F,
    register_post_build: G,
) -> Result<worker::Response, Box<dyn std::error::Error>>
where
    F: FnOnce(SolobaseBuilder) -> Result<SolobaseBuilder, Box<dyn std::error::Error>>,
    G: FnOnce(
        &mut wafer_run::Wafer,
        Arc<dyn StorageService>,
    ) -> Result<(), Box<dyn std::error::Error>>,
{
    // 1. Construct D1 service first — env vars live in D1.
    let db = make_d1_database_service(&env, runner::D1_BINDING)
        .map_err(|e| format!("D1 binding {:?}: {e}", runner::D1_BINDING))?;

    // 2. Load env vars + block settings via per-isolate cache.
    //    First request after isolate cold-start hits D1 twice; subsequent
    //    requests serve from RAM. Merge protected worker::Env bindings on
    //    top freshly each request — they're CF env bindings, not D1 vars.
    let snapshot = config_cache::get_or_load(|| async {
        let env_vars = runner::load_env_vars(&db).await;
        let block_settings = runner::load_block_settings(&db).await;
        (env_vars, block_settings)
    })
    .await;
    let mut env_vars = snapshot.0.clone();
    for key in PROTECTED_ENV_KEYS {
        if let Ok(secret) = env.secret(key) {
            env_vars.insert(key.to_string(), secret.to_string());
        }
    }
    // Fan-out block_settings into the config snapshot so consumer blocks
    // (e.g. userportal) can read enablement state via `ctx.config_get`
    // without re-querying the `block_settings` D1 table per request.
    env_vars.insert(
        solobase_core::features::BLOCK_SETTINGS_CONFIG_KEY.to_string(),
        snapshot.1.to_config_json(),
    );

    // 3. Construct remaining 5 services.
    let bucket: Arc<dyn StorageService> = make_r2_storage_service(&env, runner::R2_BINDING)
        .map_err(|e| format!("R2 binding {:?}: {e}", runner::R2_BINDING))?;
    let jwt_secret = env_vars
        .get(solobase_core::blocks::auth::JWT_SECRET_KEY)
        .cloned()
        .unwrap_or_default();
    let crypto = make_jwt_crypto_service(jwt_secret);
    let network = make_fetch_network_service();
    let logger = make_console_logger();
    let cfg_svc = make_config_service(env_vars);

    // 4. Build SolobaseBuilder, attach services + block settings. Clone the
    //    bucket Arc — `.storage()` consumes one and the post-build hook
    //    receives the other so it can register blocks that need direct R2
    //    access (e.g. a static-asset content block).
    let block_settings = snapshot.1.clone();
    let builder = SolobaseBuilder::new()
        .database(db)
        .storage(bucket.clone())
        .config(cfg_svc)
        .crypto(crypto)
        .network(network)
        .logger(logger)
        .block_settings(block_settings);

    // 5. Consumer registers its blocks.
    let builder = register_blocks(builder)?;

    // 6. Build runtime.
    let (mut wafer, storage_block) = builder.build().map_err(|e| format!("builder.build: {e}"))?;

    // 6b. Consumer post-build hook (override flows / configs before start).
    //     Receives the R2-backed StorageService so consumers can register
    //     blocks that need direct un-namespaced bucket access.
    register_post_build(&mut wafer, bucket).map_err(|e| format!("register_post_build: {e}"))?;

    // 6c. Start the runtime.
    wafer
        .start_without_bind()
        .await
        .map_err(|e| format!("wafer.start: {e}"))?;
    solobase_core::builder::post_start(&wafer, &storage_block);

    // 7. Convert request → message; preserve auth header in meta.
    let auth_header = req.headers().get("authorization")?;
    let (mut msg, input) = convert::worker_request_to_message(&req).await?;
    if let Some(ref auth) = auth_header {
        msg.set_meta("http.header.authorization", auth);
    }

    // 8. Dispatch and convert response.
    let output = wafer.run("site-main", msg, input).await;
    Ok(convert::output_to_response(output).await?)
}

/// Worker `Env` bindings that override D1 variables (set via
/// `wrangler secret put`). Most config belongs in D1 so admins can
/// manage it through the dashboard — this list stays short.
const PROTECTED_ENV_KEYS: &[&str] = &[solobase_core::blocks::auth::JWT_SECRET_KEY];
