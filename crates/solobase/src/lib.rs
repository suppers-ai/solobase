//! Solobase — self-hosted backend platform powered by the WAFER runtime.
//!
//! This library exposes the solobase feature blocks and flow definitions
//! for use by different deployment targets (standalone binary, Cloudflare
//! Workers adapter).

pub mod app_config;
pub use solobase_core::blocks;
pub mod flows;
