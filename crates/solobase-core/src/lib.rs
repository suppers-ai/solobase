//! solobase-core — shared platform abstraction for solobase.
//!
//! Contains solobase feature blocks, the shared request pipeline, routing table,
//! feature config trait, and crypto (argon2 + JWT) used by both the Cloudflare
//! Worker and native standalone binary.

pub mod blocks;
pub mod config_vars;
pub mod crypto;
pub mod features;
pub mod pipeline;
pub mod routing;
pub mod ui;

pub use features::FeatureConfig;
pub use pipeline::handle_request;
pub use routing::BlockFactory;
