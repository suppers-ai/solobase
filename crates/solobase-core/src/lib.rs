//! solobase-core — shared platform abstraction for solobase.
//!
//! Contains solobase feature blocks, the shared request pipeline, routing table,
//! feature config trait, and the auth-token policy (`crypto` module, on top of
//! `wafer_block_crypto::primitives`) used by both the Cloudflare Worker and
//! native standalone binary.

pub mod blocks;
pub mod builder;
pub mod cache;
pub mod cache_key;
pub mod config_source;
pub mod config_vars;
pub mod crypto;
pub mod features;
pub mod flows;
pub mod messages_schema;
pub mod migration_helper;
pub mod migrations;
pub mod pipeline;
pub mod routing;
pub mod ui;

#[cfg(test)]
pub mod test_support;

pub use features::FeatureConfig;
pub use migration_helper::db_backend;
pub use pipeline::handle_request;
pub use routing::{ExtraRoute, RouteAccess};
