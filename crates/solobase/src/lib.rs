//! Solobase — self-hosted backend platform powered by the WAFER runtime.
//!
//! This library exposes the solobase feature blocks and flow definitions
//! for use by different deployment targets (standalone binary, wafer-local,
//! Cloudflare Workers adapter).

pub mod app_config;
pub mod blocks;
#[cfg(not(target_arch = "wasm32"))]
pub mod flows;
