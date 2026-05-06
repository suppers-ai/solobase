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
mod runner;
pub mod schema;
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

#[cfg(test)]
mod api_surface {
    //! Compile-time check that the public `make_*` surface exists and is
    //! callable. No runtime assertions — D1/R2 require a worker runtime to
    //! instantiate. If any of the 6 symbols are renamed or removed, this
    //! stops compiling.
    #[allow(dead_code)]
    fn _signatures_compile() {
        let _: fn() -> _ = super::make_fetch_network_service;
        let _: fn() -> _ = super::make_console_logger;
        let _ = super::make_d1_database_service;
        let _ = super::make_r2_storage_service;
        let _ = super::make_jwt_crypto_service;
        let _ = super::make_config_service;
    }
}
