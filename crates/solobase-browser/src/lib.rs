//! Browser platform services + Service-Worker plumbing for WAFER-in-browser
//! applications.
//!
//! This crate is the browser half of Solobase's framework layer. It provides
//! factory functions for platform services (sql.js database, OPFS storage,
//! fetch network, browser crypto, console logger, SW asset loader), an async
//! `db_init` helper, and thread_local SW runtime plumbing
//! (`store_wafer`/`dispatch_request`/`is_initialized`). Consumers compose these
//! in their own `#[wasm_bindgen]` entrypoints using any app-level builder.

pub mod tools;
pub mod assets;
pub mod bridge;
