//! Cloudflare Workers adapter for solobase: D1 database service, R2 storage
//! service, wasm-compatible crypto/network services, and worker entry helpers.
//!
//! Consumed by:
//! - `solobase-cloud`'s `solobase-worker` (multi-tenant dispatch user worker).
//! - The `solobase build --target cloudflare` flow (single-worker consumers
//!   like wafer-site).
//!
//! This crate is wasm-only; building for native targets is not supported.

pub mod convert;
pub mod helpers;
pub mod schema;
