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
    // 1. Create the admin variables table if it does not exist.
    //    Schema mirrors admin migration `001_admin_schema.sqlite.sql` *exactly*
    //    (NOT NULL on every column the migration declares NOT NULL). Earlier
    //    revisions used a looser pre-create here, which made the admin
    //    migration's CREATE TABLE IF NOT EXISTS a no-op while still leaving
    //    the table in place — exactly the schema-drift hazard that caused
    //    the block_settings 401 bug. Keep this schema in lockstep with
    //    `crates/solobase-core/src/blocks/admin/migrations/001_admin_schema.sqlite.sql`.
    bridge::db_exec_raw(
        "CREATE TABLE IF NOT EXISTS suppers_ai__admin__variables (
            id          TEXT PRIMARY KEY,
            key         TEXT NOT NULL UNIQUE,
            name        TEXT NOT NULL DEFAULT '',
            description TEXT NOT NULL DEFAULT '',
            value       TEXT NOT NULL DEFAULT '',
            warning     TEXT NOT NULL DEFAULT '',
            sensitive   INTEGER NOT NULL DEFAULT 0,
            updated_by  TEXT NOT NULL DEFAULT '',
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        )",
        "[]",
    )
    .map_err(|e| bridge_err("create variables table", e))?;
    bridge::db_exec_raw(
        "CREATE UNIQUE INDEX IF NOT EXISTS suppers_ai__admin__variables_key_uniq ON suppers_ai__admin__variables (key)",
        "[]",
    )
    .map_err(|e| bridge_err("create variables key index", e))?;

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
    // First-time migration: users who visited a build before solobase #210
    // have a stale 4-column `suppers_ai__admin__block_settings` table
    // (block_name PK, enabled, created_at, updated_at). `CREATE TABLE IF
    // NOT EXISTS` below would no-op against it and the seed `INSERT` would
    // then fail with `no such column: id` — propagating up as an
    // `initialize()` error, which makes sw.js self-destruct, but loader.js's
    // recovery only wipes SW + Cache Storage, never OPFS, so the next page
    // load lands on the same stale schema and the loop repeats forever.
    //
    // Detect the stale schema via `pragma_table_info` and drop the table so
    // the strict CREATE below installs the canonical schema. Block_settings
    // is runtime state (enabled flags + per-block migration hashes), not
    // user data — losing the pre-seeded rows on this one-shot migration is
    // acceptable; they get re-seeded below from `defaults`.
    let table_info = bridge::db_query_raw(
        "SELECT name FROM pragma_table_info('suppers_ai__admin__block_settings')",
        "[]",
    )
    .unwrap_or_else(|_| "[]".to_string());
    let table_exists = table_info != "[]" && !table_info.is_empty();
    let has_id_column = table_info.contains("\"id\"");
    if table_exists && !has_id_column {
        bridge::db_exec_raw(
            "DROP TABLE IF EXISTS suppers_ai__admin__block_settings",
            "[]",
        )
        .map_err(|e| bridge_err("drop stale block_settings table", e))?;
    }

    // Ensure the table exists with the **canonical** strict schema from admin
    // migration `001_admin_schema.sqlite.sql`. Previously this function pre-
    // created a stale 4-column schema (block_name PK, enabled, timestamps) and
    // seeded default rows; admin migration 001's CREATE TABLE IF NOT EXISTS
    // then no-op'd against the existing table, so the migration columns
    // (`id`, `current_hash`, `blessed_hash`) never landed. The first block to
    // run `migration_helper::write_state` then found its pre-seeded row,
    // entered the UPDATE branch, and failed with `no such column:
    // blessed_hash` — surfacing as `auth migrations: block_settings update:
    // Internal: internal database error`, which aborted auth's `lifecycle(Init)`
    // permanently. With auth Init aborted, `auth::bootstrap::run` never ran
    // and every demo login returned 401. Mirror of the native fix in
    // solobase #182 (`server_config::load_block_settings`).
    bridge::db_exec_raw(
        "CREATE TABLE IF NOT EXISTS suppers_ai__admin__block_settings (
            id            TEXT PRIMARY KEY,
            block_name    TEXT NOT NULL UNIQUE,
            enabled       INTEGER NOT NULL DEFAULT 1,
            current_hash  TEXT NOT NULL DEFAULT '',
            blessed_hash  TEXT NOT NULL DEFAULT '',
            created_at    TEXT NOT NULL,
            updated_at    TEXT NOT NULL
        )",
        "[]",
    )
    .map_err(|e| bridge_err("create block_settings table", e))?;
    bridge::db_exec_raw(
        "CREATE UNIQUE INDEX IF NOT EXISTS suppers_ai__admin__block_settings_block_name_uniq \
         ON suppers_ai__admin__block_settings (block_name)",
        "[]",
    )
    .map_err(|e| bridge_err("create block_settings block_name index", e))?;

    // Seed defaults for known blocks. The strict schema demands `id`,
    // `created_at`, `updated_at` — generate them inline via SQLite builtins
    // so the seed remains a one-shot SQL statement per block.
    let defaults: &[(&str, bool)] = &[
        ("suppers-ai/auth", true),
        ("suppers-ai/admin", true),
        ("suppers-ai/files", true),
        ("suppers-ai/products", true),
        ("suppers-ai/legalpages", false),
        ("suppers-ai/userportal", false),
        ("suppers-ai/system", true),
    ];

    for &(name, default) in defaults {
        let params = serde_json::json!([name, default as i32]);
        bridge::db_exec_raw(
            "INSERT OR IGNORE INTO suppers_ai__admin__block_settings \
             (id, block_name, enabled, created_at, updated_at) \
             VALUES (lower(hex(randomblob(16))), ?, ?, datetime('now'), datetime('now'))",
            &params.to_string(),
        )
        .map_err(|e| bridge_err(&format!("seed block_setting {name}"), e))?;
    }

    // Read all settings
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
