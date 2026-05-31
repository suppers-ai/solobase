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
pub(crate) const KV_BINDING: &str = "CONFIG_CACHE";

use std::{collections::HashMap, sync::Arc};

pub(crate) use solobase_core::blocks::admin::BLOCK_SETTINGS_TABLE;
use solobase_core::{blocks::admin::VARIABLES_TABLE, cache_key, features::BlockSettings};
use wafer_block::db::ListOptions;
use wafer_core::interfaces::database::service::DatabaseService;

/// Read the admin block-settings collection and convert to `BlockSettings`.
///
/// Returns `BlockSettings::default()` on missing collection or query
/// failure — matches the existing solobase-cloud worker's error tolerance.
pub(crate) async fn load_block_settings(db: &Arc<dyn DatabaseService>) -> BlockSettings {
    use solobase_core::features::{BlockState, ExistingRow, MigrationState, SeedOp};

    let opts = ListOptions {
        offset: 0,
        limit: 10_000,
        skip_count: true,
        ..Default::default()
    };
    let record_list = match db.list(BLOCK_SETTINGS_TABLE, &opts).await {
        Ok(rl) => rl,
        Err(e) => {
            worker::console_log!("warn: load_block_settings failed: {e}");
            return BlockSettings::default();
        }
    };

    // Build the existing-row map for the hash-gate planner.
    let existing: HashMap<String, ExistingRow> = record_list
        .records
        .iter()
        .filter_map(|r| {
            let name = r.data.get("block_name")?.as_str()?.to_string();
            let enabled = r.data.get("enabled")?.as_i64()? != 0;
            let hash = r
                .data
                .get("seed_defaults_hash")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Some((name, ExistingRow { enabled, hash }))
        })
        .collect();

    // `block_name` → row `id`, so the `SeedOp::Update` branch can do a
    // single-row `db.update` (which the KV wrapper invalidates) instead of
    // `db.update_where` (which hard-errors on cached tables, so a changed
    // `ENABLED_DEFAULTS` hash would never propagate to existing rows).
    let id_by_block: HashMap<String, String> = record_list
        .records
        .iter()
        .filter_map(|r| {
            let name = r.data.get("block_name")?.as_str()?.to_string();
            let id = r.data.get("id")?.as_str()?.to_string();
            Some((name, id))
        })
        .collect();

    // Plan + apply. Steady state: empty decisions → zero D1 writes.
    let decisions = solobase_core::features::plan_seed_decisions(&existing);
    let any_writes = !decisions.is_empty();
    for d in &decisions {
        let enabled_val = serde_json::Value::Number(serde_json::Number::from(if d.enabled {
            1i64
        } else {
            0i64
        }));
        let hash_val = serde_json::Value::String(d.hash.clone());
        let now = chrono::Utc::now().to_rfc3339();
        match d.op {
            SeedOp::Insert => {
                let id = format!("bs_{}", uuid::Uuid::new_v4());
                let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                data.insert("id".into(), serde_json::Value::String(id));
                data.insert(
                    "block_name".into(),
                    serde_json::Value::String(d.block_name.to_string()),
                );
                data.insert("enabled".into(), enabled_val);
                data.insert("seed_defaults_hash".into(), hash_val);
                data.insert("created_at".into(), serde_json::Value::String(now.clone()));
                data.insert("updated_at".into(), serde_json::Value::String(now));
                if let Err(e) = db.create(BLOCK_SETTINGS_TABLE, data).await {
                    worker::console_log!("warn: seed insert {} failed: {e}", d.block_name);
                }
            }
            SeedOp::Update => {
                // Update is planned only for rows already present in
                // `existing`, so the id should always resolve; skip
                // defensively (rather than fall back to update_where, which
                // would hard-error) if a row somehow lacks one.
                let Some(id) = id_by_block.get(d.block_name) else {
                    worker::console_log!("warn: seed update {} skipped: no row id", d.block_name);
                    continue;
                };
                let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                data.insert("enabled".into(), enabled_val);
                data.insert("seed_defaults_hash".into(), hash_val);
                data.insert("updated_at".into(), serde_json::Value::String(now));
                if let Err(e) = db.update(BLOCK_SETTINGS_TABLE, id, data).await {
                    worker::console_log!("warn: seed update {} failed: {e}", d.block_name);
                }
            }
        }
    }

    // If we wrote anything, re-read for the post-seed view. Costs 1 extra
    // D1 read only when a default actually changed (rare).
    let final_records = if any_writes {
        match db.list(BLOCK_SETTINGS_TABLE, &opts).await {
            Ok(rl) => rl.records,
            Err(e) => {
                worker::console_log!("warn: post-seed re-read failed: {e}");
                record_list.records
            }
        }
    } else {
        record_list.records
    };

    let blocks: HashMap<String, BlockState> = final_records
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
    // `block` column matches what admin migration 002 backfills from `key`
    // (key's first two `__`-delimited segments). We compute it from the
    // block name via the canonical helper so both paths converge on the
    // same SCREAMING_SNAKE prefix even if a key were ever introduced with
    // a non-matching shape. Computed up front so the existence check below
    // can filter on it.
    let block_col = crate::config_source::D1ConfigSource::screaming_block(block_name);

    // Existence check via the per-block list shape the KV cache recognizes
    // (`cache_key::block_list_opts`). This shares the cache entry
    // `D1ConfigSource` populates for this block on init, so on a warm isolate
    // it's a KV hit instead of an uncached `key`-filtered D1 read on every
    // request. We then match `var.key` in memory.
    let exists_opts = cache_key::block_list_opts(cache_key::CachedTable::Variables, &block_col);
    let listed = db.list(VARIABLES_TABLE, &exists_opts).await?;
    if listed
        .records
        .iter()
        .any(|r| r.data.get("key").and_then(|v| v.as_str()) == Some(var.key.as_str()))
    {
        return Ok(());
    }

    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).map_err(|e| format!("getrandom: {e}"))?;
    let secret: String = bytes.iter().map(|b| format!("{b:02x}")).collect();

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
