//! Cloudflare Workers adapter for solobase: D1 database service, R2 storage
//! service, wasm-compatible crypto/network services, and worker entry helpers.
//!
//! Consumed by:
//! - `solobase-cloud`'s `solobase-worker` (multi-tenant dispatch user worker).
//! - The `solobase build --target cloudflare` flow (single-worker consumers
//!   like wafer-site).
//!
//! This crate is wasm-only; building for native targets is not supported.

pub mod config_service;
pub mod convert;
pub mod crypto_service;
pub mod database;
pub mod helpers;
pub mod logger_service;
pub mod network_service;
pub mod schema;
pub mod storage;

// ---------------------------------------------------------------------------
// Public `make_*` constructors — mirrors `solobase-native`'s API surface.
// Consumers construct services through these helpers rather than importing
// internal types directly.
// ---------------------------------------------------------------------------

use std::collections::HashMap;
use std::sync::Arc;
use wafer_core::interfaces::config::service::ConfigService;
use wafer_core::interfaces::crypto::service::CryptoService;
use wafer_core::interfaces::database::service::DatabaseService;
use wafer_core::interfaces::logger::service::LoggerService;
use wafer_core::interfaces::network::service::NetworkService;
use wafer_core::interfaces::storage::service::StorageService;

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
pub fn make_crypto_service(jwt_secret: String) -> Arc<dyn CryptoService> {
    Arc::new(crypto_service::SolobaseCryptoService::new(jwt_secret))
}

/// Construct a [`NetworkService`] backed by the CF Worker global `fetch` API.
pub fn make_network_service() -> Arc<dyn NetworkService> {
    Arc::new(network_service::WorkerFetchService)
}

/// Construct a [`LoggerService`] that writes to `worker::console_log`.
pub fn make_logger_service() -> Arc<dyn LoggerService> {
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
