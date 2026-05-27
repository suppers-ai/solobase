//! Browser-side variable seeding and loading.
//!
//! Mirrors the native `seed_and_load_variables()` / `load_block_settings()` from
//! `solobase/src/main.rs`, but uses the JS bridge (`bridge::db_exec_raw` /
//! `bridge::db_query_raw`) instead of `rusqlite`.

use std::collections::HashMap;

use solobase_browser::bridge;
use wasm_bindgen::JsValue;

/// Format a JS bridge error into a JsValue carrying a stable, contextual prefix.
fn bridge_err(ctx: &str, e: JsValue) -> JsValue {
    let detail = e.as_string().unwrap_or_else(|| format!("{e:?}"));
    JsValue::from_str(&format!("solobase-web config: {ctx}: {detail}"))
}

// ---------------------------------------------------------------------------
// Variable seeding and loading
// ---------------------------------------------------------------------------

/// Ensure the variables table exists, auto-generate secrets for config vars
/// marked `auto_generate: true`, and return all variables from the DB.
///
/// This is the browser equivalent of the native `seed_and_load_variables()`.
/// There are no env vars to seed from in the browser — only auto-generated
/// secrets and previously-stored values.
pub fn seed_and_load_variables() -> Result<HashMap<String, String>, JsValue> {
    // PRECONDITION: `wafer.init_block(suppers-ai/admin)` must have already
    // run before this function is called — admin's migration is the single
    // source of schema truth for `suppers_ai__admin__variables`, and we no
    // longer pre-create the table here. See `initialize()` in `lib.rs` for
    // the ordering. Removing the pre-create eliminates the schema-drift
    // class of bug that took down the demo in #210/#211: any future change
    // to `001_admin_schema.sqlite.sql` now propagates without needing a
    // mirrored CREATE in this crate.

    // 2. Seed default admin account for browser build.
    //    Email: admin@example.com / Password: admin123
    //    This is local-only (OPFS) so a simple default is acceptable.
    bridge::db_exec_raw(
        "INSERT OR IGNORE INTO suppers_ai__admin__variables (id, key, name, description, value, sensitive, created_at, updated_at)
         VALUES ('var_admin_email', 'SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_EMAIL', 'Admin Email', 'Admin account email', 'admin@example.com', 0, datetime('now'), datetime('now'))",
        "[]",
    )
    .map_err(|e| bridge_err("seed admin email var", e))?;
    bridge::db_exec_raw(
        "INSERT OR IGNORE INTO suppers_ai__admin__variables (id, key, name, description, value, sensitive, created_at, updated_at)
         VALUES ('var_admin_pass', 'SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_PASSWORD', 'Admin Password', 'Admin account password', 'admin123', 1, datetime('now'), datetime('now'))",
        "[]",
    )
    .map_err(|e| bridge_err("seed admin password var", e))?;

    // Inject the page-side WebLLM engine into every SSR-rendered page.
    // Native/server targets leave this var unset and skip the injection.
    bridge::db_exec_raw(
        "INSERT OR IGNORE INTO suppers_ai__admin__variables (id, key, name, description, value, sensitive, created_at, updated_at)
         VALUES ('var_embedded_scripts', 'SOLOBASE_SHARED__EMBEDDED_SCRIPTS', 'Embedded Scripts', 'Module-type script URLs embedded in every page', '/webllm-engine.js', 0, datetime('now'), datetime('now'))",
        "[]",
    )
    .map_err(|e| bridge_err("seed embedded scripts var", e))?;

    // 3. Auto-generate secrets for config vars marked with auto_generate
    seed_auto_generated()?;

    // 4. Load all variables
    let json = bridge::db_query_raw("SELECT key, value FROM suppers_ai__admin__variables", "[]")
        .map_err(|e| bridge_err("load variables", e))?;
    let rows: Vec<serde_json::Value> = serde_json::from_str(&json)
        .map_err(|e| JsValue::from_str(&format!("solobase-web config: parse variables: {e}")))?;

    let mut vars = HashMap::new();
    for row in rows {
        let key = row.get("key").and_then(|v| v.as_str()).unwrap_or_default();
        let value = row
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if !key.is_empty() {
            vars.insert(key.to_string(), value.to_string());
        }
    }

    Ok(vars)
}

/// Auto-generate values for config vars marked with `auto_generate: true`.
///
/// Reads all block config var declarations, finds those needing auto-generation,
/// and generates random hex values for any that don't already exist in the
/// variables table.
fn seed_auto_generated() -> Result<(), JsValue> {
    let block_infos = solobase_core::blocks::all_block_infos();
    let all_vars = solobase_core::config_vars::collect_all_config_vars(&block_infos);

    for var in &all_vars {
        if !var.auto_generate {
            continue;
        }

        // Generate a random 32-byte hex secret
        let mut bytes = [0u8; 32];
        if let Err(e) = getrandom::getrandom(&mut bytes) {
            web_sys::console::warn_1(
                &format!("solobase: getrandom failed for {}: {e}", var.key).into(),
            );
            continue;
        }
        let secret: String = bytes.iter().map(|b| format!("{b:02x}")).collect();

        let id = format!("var_{}", uuid::Uuid::new_v4());
        let sensitive: i32 = if var.is_sensitive() { 1 } else { 0 };

        // INSERT OR IGNORE — existing DB values take priority. We still
        // propagate bridge errors: a failing INSERT here means the var
        // never lands in the snapshot returned to `initialize()`, and any
        // block that needs it will fail later with a far less obvious
        // "missing config var" error.
        let params = serde_json::json!([
            id,
            var.key,
            var.name,
            var.description,
            secret,
            var.warning,
            sensitive
        ]);
        bridge::db_exec_raw(
            "INSERT OR IGNORE INTO suppers_ai__admin__variables (id, key, name, description, value, warning, sensitive, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))",
            &params.to_string(),
        )
        .map_err(|e| bridge_err(&format!("seed auto-generated var {}", var.key), e))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Block settings
// ---------------------------------------------------------------------------

/// Load block settings from the browser database.
///
/// Reads the `suppers_ai__admin__block_settings` table (creating it if needed)
/// and returns a `BlockSettings` with the enabled/disabled state of each block.
pub fn load_block_settings() -> Result<solobase_core::features::BlockSettings, JsValue> {
    // PRECONDITION: `wafer.init_block(suppers-ai/admin)` must have already
    // run before this function is called — admin's migration is the single
    // source of schema truth for `suppers_ai__admin__block_settings`. We no
    // longer pre-create the table here.
    //
    // One-shot migration for legacy OPFS state: users who visited a build
    // before this restructure (and before #210/#211) have a stale 4-column
    // `block_settings` table that admin's `CREATE TABLE IF NOT EXISTS` would
    // no-op against. Detect that case via `pragma_table_info` and drop the
    // table BEFORE admin runs — but at this point in initialize() admin has
    // already run, so any stale schema would already have tripped its
    // migration. We still keep a defensive drop here for the narrow window
    // where a user upgraded directly from a pre-#210 build through this
    // restructure (admin migration tolerated the stale schema enough to
    // succeed but write_state still failed downstream). Future-builds-only
    // could remove this drop after a deprecation window.
    let table_info = bridge::db_query_raw(
        "SELECT name FROM pragma_table_info('suppers_ai__admin__block_settings')",
        "[]",
    )
    .unwrap_or_else(|_| "[]".to_string());
    let has_id_column = table_info.contains("\"id\"");
    let table_exists = table_info != "[]" && !table_info.is_empty();
    if table_exists && !has_id_column {
        // Stale schema detected. Drop and rely on admin migration to recreate
        // — except we already passed admin's Init, so admin won't re-run.
        // Re-create the canonical schema inline as a one-shot recovery.
        bridge::db_exec_raw(
            "DROP TABLE IF EXISTS suppers_ai__admin__block_settings",
            "[]",
        )
        .map_err(|e| bridge_err("drop stale block_settings table", e))?;
        bridge::db_exec_raw(
            "CREATE TABLE suppers_ai__admin__block_settings (
                id                 TEXT PRIMARY KEY,
                block_name         TEXT NOT NULL UNIQUE,
                enabled            INTEGER NOT NULL DEFAULT 1,
                current_hash       TEXT NOT NULL DEFAULT '',
                blessed_hash       TEXT NOT NULL DEFAULT '',
                seed_defaults_hash TEXT NOT NULL DEFAULT '',
                created_at         TEXT NOT NULL,
                updated_at         TEXT NOT NULL
            )",
            "[]",
        )
        .map_err(|e| bridge_err("recreate block_settings table", e))?;
        bridge::db_exec_raw(
            "CREATE UNIQUE INDEX IF NOT EXISTS suppers_ai__admin__block_settings_block_name_uniq \
             ON suppers_ai__admin__block_settings (block_name)",
            "[]",
        )
        .map_err(|e| bridge_err("recreate block_settings block_name index", e))?;
    }

    // Hash-gated seed: read existing rows including their stored
    // `seed_defaults_hash`, then ask the planner what (if anything) needs
    // to change. Steady state is an empty plan → zero writes.
    let read_json = bridge::db_query_raw(
        "SELECT block_name, enabled, seed_defaults_hash FROM suppers_ai__admin__block_settings",
        "[]",
    )
    .map_err(|e| bridge_err("read block_settings for hash-gate", e))?;
    let existing_rows: Vec<serde_json::Value> = serde_json::from_str(&read_json).map_err(|e| {
        JsValue::from_str(&format!(
            "solobase-web config: parse block_settings (pre-seed): {e}"
        ))
    })?;

    let mut existing = HashMap::new();
    for row in existing_rows {
        let name = row
            .get("block_name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if name.is_empty() {
            continue;
        }
        let enabled = row.get("enabled").and_then(|v| v.as_i64()).unwrap_or(1) != 0;
        let hash = row
            .get("seed_defaults_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        existing.insert(name, solobase_core::features::ExistingRow { enabled, hash });
    }

    let decisions = solobase_core::features::plan_seed_decisions(&existing);
    for d in &decisions {
        let enabled_int: i64 = if d.enabled { 1 } else { 0 };
        match d.op {
            solobase_core::features::SeedOp::Insert => {
                let params = serde_json::json!([d.block_name, enabled_int, d.hash]);
                bridge::db_exec_raw(
                    "INSERT INTO suppers_ai__admin__block_settings \
                     (id, block_name, enabled, seed_defaults_hash, created_at, updated_at) \
                     VALUES (lower(hex(randomblob(16))), ?, ?, ?, datetime('now'), datetime('now'))",
                    &params.to_string(),
                )
                .map_err(|e| bridge_err(&format!("seed insert {}", d.block_name), e))?;
            }
            solobase_core::features::SeedOp::Update => {
                let params = serde_json::json!([enabled_int, d.hash, d.block_name]);
                bridge::db_exec_raw(
                    "UPDATE suppers_ai__admin__block_settings \
                     SET enabled = ?, seed_defaults_hash = ?, updated_at = datetime('now') \
                     WHERE block_name = ?",
                    &params.to_string(),
                )
                .map_err(|e| bridge_err(&format!("seed update {}", d.block_name), e))?;
            }
        }
    }

    // Read post-seed state.
    let json = bridge::db_query_raw(
        "SELECT block_name, enabled FROM suppers_ai__admin__block_settings",
        "[]",
    )
    .map_err(|e| bridge_err("read block_settings", e))?;
    let rows: Vec<serde_json::Value> = serde_json::from_str(&json).map_err(|e| {
        JsValue::from_str(&format!("solobase-web config: parse block_settings: {e}"))
    })?;

    let mut map = HashMap::new();
    for row in rows {
        let name = row
            .get("block_name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let enabled = row.get("enabled").and_then(|v| v.as_i64()).unwrap_or(1) != 0;
        if !name.is_empty() {
            map.insert(name, enabled);
        }
    }

    Ok(solobase_core::features::BlockSettings::from_map(map))
}
