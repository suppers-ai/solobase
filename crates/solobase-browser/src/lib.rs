//! Browser platform services + Service-Worker plumbing for WAFER-in-browser
//! applications.
//!
//! This crate is the browser half of Solobase's framework layer. It provides
//! factory functions for platform services (sql.js database, OPFS storage,
//! fetch network, browser crypto, console logger, SW asset loader), an async
//! `db_init` helper, and thread_local SW runtime plumbing
//! (`store_wafer`/`dispatch_request`/`is_initialized`). Consumers compose these
//! in their own `#[wasm_bindgen]` entrypoints using any app-level builder.
//!
//! The `assets` and `tools` modules are unconditionally available (native +
//! wasm32) and are used by the native `export-assets` binary. All other modules
//! are wasm32-only.

// Always available — used by native tooling (export-assets bin) and wasm32.
pub mod assets;
pub mod tools;

// wasm32-only — use wasm-bindgen, web-sys, js-sys.
#[cfg(target_arch = "wasm32")]
pub mod asset_loader;
#[cfg(target_arch = "wasm32")]
pub mod bridge;
#[cfg(target_arch = "wasm32")]
pub mod convert;
#[cfg(target_arch = "wasm32")]
pub mod crypto;
#[cfg(target_arch = "wasm32")]
pub mod database;
#[cfg(target_arch = "wasm32")]
pub mod logger;
#[cfg(target_arch = "wasm32")]
pub mod network;
#[cfg(target_arch = "wasm32")]
pub mod runtime;
#[cfg(target_arch = "wasm32")]
pub mod storage;

#[cfg(target_arch = "wasm32")]
pub use asset_loader::make_sw_asset_loader;
#[cfg(target_arch = "wasm32")]
pub use crypto::make_crypto_service;
#[cfg(target_arch = "wasm32")]
pub use database::make_database_service;
#[cfg(target_arch = "wasm32")]
pub use logger::make_console_logger;
#[cfg(target_arch = "wasm32")]
pub use network::make_network_service;
#[cfg(target_arch = "wasm32")]
pub use runtime::{dispatch_request, is_initialized, store_wafer};
#[cfg(target_arch = "wasm32")]
pub use storage::make_storage_service;

/// Load sql.js WASM and open (or create) the OPFS-backed database.
/// Idempotent-safe to call once at startup, before constructing platform
/// services. Wraps `bridge::dbInit()`.
#[cfg(target_arch = "wasm32")]
pub async fn db_init() {
    bridge::dbInit().await;
}
