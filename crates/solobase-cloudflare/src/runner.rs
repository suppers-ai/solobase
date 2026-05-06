//! Worker entry orchestration for `solobase_cloudflare::run()`.
//!
//! Binding-name convention (matches `solobase-cloud/crates/solobase-worker`):
//! - D1 binding: `"DB"`
//! - R2 binding: `"STORAGE"`
//!
//! Consumers must use these names in their `wrangler.toml`. The CLI's
//! `helpers::cloudflare::wrangler` generator emits them as defaults.
//! v2 may take a `RunConfig` parameter for custom binding names.

pub(crate) const D1_BINDING: &str = "DB";
pub(crate) const R2_BINDING: &str = "STORAGE";

pub(crate) use solobase_core::blocks::admin::{BLOCK_SETTINGS_COLLECTION, VARIABLES_COLLECTION};
