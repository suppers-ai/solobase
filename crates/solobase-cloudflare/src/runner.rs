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
use solobase_core::{blocks::admin::VARIABLES_TABLE, features::BlockSettings};
use wafer_core::interfaces::database::service::{DatabaseService, Filter, FilterOp, ListOptions};

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
                    let seed_defaults_hash = r
                        .data
                        .get("seed_defaults_hash")
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
                            seed_defaults_hash,
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

/// Auto-generate random secrets for every `ConfigVar` declared with
/// `.auto_generate()` that doesn't yet have a row in the admin variables
/// table. Native solobase does this in the CLI before runtime start
/// (`solobase/src/cli/server.rs::seed_auto_generated`); the CF runner is
/// the corresponding eager-seed pass for the Cloudflare target.
///
/// MUST be called after the admin block's `lifecycle(Init)` has finished
/// (so admin migration 002 has added the `block` column the `D1ConfigSource`
/// queries by) and BEFORE [`wafer_run::Wafer::init_all_blocks`] runs the
/// rest of the inits — otherwise blocks that require an auto-gen key (e.g.
/// `suppers-ai/products` and `SUPPERS_AI__PRODUCTS__WEBHOOK_SECRET`) hit
/// [`wafer_run::ConfigError::MissingRequired`], which is mapped to
/// `InitError::Permanent` and cached for the slot's lifetime.
///
/// Idempotent: rows whose `key` already exists are left untouched.
/// Per-key failures are logged but do not abort the loop — operators retain
/// the manual `wrangler d1 execute` fallback for any key that fails to seed.
pub(crate) async fn seed_auto_generated(db: &Arc<dyn DatabaseService>) {
    let block_infos = solobase_core::blocks::all_block_infos();
    for info in &block_infos {
        for var in &info.config_keys {
            if !var.auto_generate {
                continue;
            }
            if let Err(e) = seed_one(db, &info.name, var).await {
                worker::console_log!(
                    "warn: seed_auto_generated failed for {} ({}): {e}",
                    var.key,
                    info.name,
                );
            }
        }
    }
}

/// Insert a single auto-generated secret row when no row with `var.key`
/// already exists. Returns `Ok(())` on either insert or skip; bubbles up
/// only D1 / RNG errors so the caller can log per-key.
async fn seed_one(
    db: &Arc<dyn DatabaseService>,
    block_name: &str,
    var: &wafer_block::ConfigVar,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let exists_opts = ListOptions {
        filters: vec![Filter {
            field: "key".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(var.key.clone()),
        }],
        limit: 1,
        skip_count: true,
        ..Default::default()
    };
    let listed = db.list(VARIABLES_TABLE, &exists_opts).await?;
    if !listed.records.is_empty() {
        return Ok(());
    }

    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).map_err(|e| format!("getrandom: {e}"))?;
    let secret: String = bytes.iter().map(|b| format!("{b:02x}")).collect();

    // `block` column matches what admin migration 002 backfills from `key`
    // (key's first two `__`-delimited segments). We compute it from the
    // block name via the canonical helper so both paths converge on the
    // same SCREAMING_SNAKE prefix even if a key were ever introduced with
    // a non-matching shape.
    let block_col = crate::config_source::D1ConfigSource::screaming_block(block_name);
    let now = chrono::Utc::now().to_rfc3339();
    let id = format!("var_{}", uuid::Uuid::new_v4());

    let mut data: HashMap<String, serde_json::Value> = HashMap::new();
    data.insert("id".into(), serde_json::Value::String(id));
    data.insert("key".into(), serde_json::Value::String(var.key.clone()));
    data.insert("name".into(), serde_json::Value::String(var.name.clone()));
    data.insert(
        "description".into(),
        serde_json::Value::String(var.description.clone()),
    );
    data.insert("value".into(), serde_json::Value::String(secret));
    data.insert(
        "warning".into(),
        serde_json::Value::String(var.warning.clone()),
    );
    data.insert(
        "sensitive".into(),
        serde_json::Value::Number(serde_json::Number::from(1)),
    );
    data.insert("block".into(), serde_json::Value::String(block_col));
    data.insert("created_at".into(), serde_json::Value::String(now.clone()));
    data.insert("updated_at".into(), serde_json::Value::String(now));

    db.create(VARIABLES_TABLE, data).await?;
    worker::console_log!(
        "auto-generated secret seeded for {} (no row existed)",
        var.key
    );
    Ok(())
}
