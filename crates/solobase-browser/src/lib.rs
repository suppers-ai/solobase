//! Browser platform services + Service-Worker plumbing for WAFER-in-browser
//! applications.
//!
//! This crate is the browser half of Solobase's framework layer. It provides
//! factory functions for platform services (sql.js database, OPFS storage,
//! fetch network, browser crypto, console logger, SW asset loader), an async
//! `db_init` helper, and thread_local SW runtime plumbing
//! (`store_wafer`/`dispatch_request`/`is_initialized`). Consumers compose these
//! in their own `#[wasm_bindgen]` entrypoints using any app-level builder.

pub mod asset_loader;
pub mod assets;
pub mod bridge;
pub mod convert;
pub mod crypto;
pub mod database;
pub mod logger;
pub mod network;
pub mod runtime;
pub mod storage;
pub mod tools;

pub use asset_loader::make_sw_asset_loader;
pub use crypto::make_crypto_service;
pub use database::make_database_service;
pub use logger::make_console_logger;
pub use network::make_network_service;
pub use runtime::{dispatch_request, is_initialized, store_wafer};
pub use storage::make_storage_service;
