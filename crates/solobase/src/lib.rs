//! Solobase — self-hosted backend platform powered by the WAFER runtime.
//!
//! This library is platform-agnostic. It provides the `SolobaseBuilder` for
//! unified runtime setup, flow definitions, and the router block. Platform-specific
//! code (native binary, browser WASM, Cloudflare Workers) lives in separate crates
//! that depend on this one.

pub mod builder;
pub use solobase_core::blocks;
pub mod flows;
