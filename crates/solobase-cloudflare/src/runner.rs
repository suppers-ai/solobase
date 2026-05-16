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

use std::{collections::HashMap, sync::Arc};

pub(crate) use solobase_core::blocks::admin::BLOCK_SETTINGS_TABLE;
use solobase_core::features::BlockSettings;
use wafer_core::interfaces::database::service::{DatabaseService, ListOptions};

/// Read the admin block-settings collection and convert to `BlockSettings`.
///
/// Returns `BlockSettings::default()` on missing collection or query
/// failure — matches the existing solobase-cloud worker's error tolerance.
pub(crate) async fn load_block_settings(db: &Arc<dyn DatabaseService>) -> BlockSettings {
    use solobase_core::features::{BlockState, MigrationState};

    let opts = ListOptions {
        offset: 0,
        limit: 10_000,
        skip_count: true,
        ..Default::default()
    };
    match db.list(BLOCK_SETTINGS_TABLE, &opts).await {
        Ok(record_list) => {
            let blocks: HashMap<String, BlockState> = record_list
                .records
                .into_iter()
                .filter_map(|r| {
                    let name = r.data.get("block_name")?.as_str()?.to_string();
                    let enabled = r.data.get("enabled")?.as_i64()? != 0;
                    let current_hash = r
                        .data
                        .get("current_hash")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let blessed_hash = r
                        .data
                        .get("blessed_hash")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some((
                        name,
                        BlockState {
                            enabled,
                            migration: MigrationState {
                                current_hash,
                                blessed_hash,
                            },
                        },
                    ))
                })
                .collect();
            BlockSettings::from_blocks(blocks)
        }
        Err(e) => {
            worker::console_log!("warn: load_block_settings failed: {e}");
            BlockSettings::default()
        }
    }
}
