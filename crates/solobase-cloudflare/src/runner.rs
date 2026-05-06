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

/// Variables-table query: `(key, value)` rows from `suppers_ai__admin__variables`.
pub(crate) const VARIABLES_TABLE: &str = "suppers_ai__admin__variables";

/// Block-settings table: `(block_name, enabled)` rows.
pub(crate) const BLOCK_SETTINGS_TABLE: &str = "suppers_ai__admin__block_settings";
