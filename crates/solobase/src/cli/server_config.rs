//! App-specific config loading (schema-aware, depends on solobase-core).
//!
//! `filter_to_declared_keys` sits between the library's raw-env-var
//! collection and the SQLite-backed variable seeding, preserving the
//! prior behavior of only persisting env vars that match a declared
//! block/shared config var key.

use std::collections::HashMap;

pub use solobase_core::features::BlockSettings;

pub fn filter_to_declared_keys(env_vars: HashMap<String, String>) -> Vec<(String, String)> {
    let block_infos = solobase_core::blocks::all_block_infos();
    let all_vars = solobase_core::config_vars::collect_all_config_vars(&block_infos);
    // Borrow the declared keys directly — the HashSet only lives for the
    // duration of the filter, so there's no need to allocate owned Strings.
    let known: std::collections::HashSet<&str> = all_vars.iter().map(|v| v.key.as_str()).collect();
    env_vars
        .into_iter()
        .filter(|(k, _)| known.contains(k.as_str()))
        .collect()
}

// ---------------------------------------------------------------------------
// Block settings — read from suppers_ai__admin__block_settings table + SOLOBASE_BLOCK_* env vars
// ---------------------------------------------------------------------------

/// Load block settings from the SQLite database.
///
/// Reads the `suppers_ai__admin__block_settings` table (if it exists) and returns a
/// `BlockSettings` with the enabled/disabled state of each block.
/// Also seeds from `SOLOBASE_BLOCK_ENABLED` / `SOLOBASE_BLOCK_DISABLED` env vars.
/// Default enablement for a known solobase block.
///
/// Tuple shape: `(full_name, default_enabled)`. Used by [`BLOCK_DEFAULTS`]
/// to seed the `suppers_ai__admin__block_settings` table on first run so
/// the admin UI shows the canonical solobase block roster even before any
/// `SOLOBASE_BLOCK_ENABLED` / `SOLOBASE_BLOCK_DISABLED` env override.
pub type BlockDefault = (&'static str, bool);

/// Known solobase blocks with their default enabled state.
/// Blocks not listed here default to enabled.
pub const BLOCK_DEFAULTS: &[BlockDefault] = &[
    ("suppers-ai/auth", true),
    ("suppers-ai/admin", true),
    ("suppers-ai/files", true),
    ("suppers-ai/products", true),
    ("suppers-ai/legalpages", false),
    ("suppers-ai/userportal", false),
    ("suppers-ai/system", true),
];

pub fn load_block_settings(db_path: &str) -> BlockSettings {
    let conn = match rusqlite::Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return BlockSettings::from_map(HashMap::new()),
    };

    // Ensure suppers_ai__admin__block_settings table exists
    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS suppers_ai__admin__block_settings (
            block_name TEXT PRIMARY KEY,
            enabled INTEGER DEFAULT 1,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );",
    );

    // Seed defaults for known blocks (INSERT OR IGNORE — existing DB values take priority).
    // Match the tolerant style of the rest of this function: if the prepare
    // fails (e.g. table schema mismatch from an older deploy), log and skip
    // seeding rather than panicking — `read all settings` below still works
    // off whatever rows already exist.
    match conn.prepare("INSERT OR IGNORE INTO suppers_ai__admin__block_settings (block_name, enabled) VALUES (?1, ?2)") {
        Ok(mut stmt) => {
            for &(name, default) in BLOCK_DEFAULTS {
                let _ = stmt.execute(rusqlite::params![name, i32::from(default)]);
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "skipped seeding block_settings defaults");
        }
    }

    // Seed from SOLOBASE_BLOCK_ENABLED env var (force-enable specific blocks)
    if let Ok(enabled) = std::env::var("SOLOBASE_BLOCK_ENABLED") {
        for name in enabled.split(',') {
            let name = name.trim();
            if !name.is_empty() {
                let _ = conn.execute(
                    "INSERT INTO suppers_ai__admin__block_settings (block_name, enabled) VALUES (?1, 1) \
                     ON CONFLICT (block_name) DO UPDATE SET enabled = 1",
                    rusqlite::params![name],
                );
            }
        }
    }

    // Seed from SOLOBASE_BLOCK_DISABLED env var (force-disable specific blocks)
    if let Ok(disabled) = std::env::var("SOLOBASE_BLOCK_DISABLED") {
        for name in disabled.split(',') {
            let name = name.trim();
            if !name.is_empty() {
                let _ = conn.execute(
                    "INSERT INTO suppers_ai__admin__block_settings (block_name, enabled) VALUES (?1, 0) \
                     ON CONFLICT (block_name) DO UPDATE SET enabled = 0",
                    rusqlite::params![name],
                );
            }
        }
    }

    // Read all settings
    let mut map = HashMap::new();
    if let Ok(mut stmt) =
        conn.prepare("SELECT block_name, enabled FROM suppers_ai__admin__block_settings")
    {
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)? != 0))
        });
        if let Ok(rows) = rows {
            for row in rows.flatten() {
                map.insert(row.0, row.1);
            }
        }
    }

    BlockSettings::from_map(map)
}

/// Load custom WRAP grants from the SQLite database.
///
/// Reads the `suppers_ai__admin__wrap_grants` table and returns a list of
/// `ResourceGrant` values that should be injected into the WAFER runtime.
pub fn load_wrap_grants(db_path: &str) -> Vec<wafer_run::ResourceGrant> {
    let conn = match rusqlite::Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut grants = Vec::new();
    let mut stmt = match conn.prepare(
        "SELECT grantee, resource, write, resource_type FROM suppers_ai__admin__wrap_grants",
    ) {
        Ok(s) => s,
        Err(_) => {
            // Fall back to query without resource_type (column may not exist yet)
            let mut stmt = match conn
                .prepare("SELECT grantee, resource, write FROM suppers_ai__admin__wrap_grants")
            {
                Ok(s) => s,
                Err(_) => return Vec::new(),
            };
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i32>(2)?,
                ))
            });
            if let Ok(rows) = rows {
                for row in rows.flatten() {
                    let (grantee, resource, write) = row;
                    grants.push(wafer_run::ResourceGrant {
                        grantee,
                        resource,
                        write: write != 0,
                        resource_type: None,
                    });
                }
            }
            return grants;
        }
    };

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i32>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    });

    if let Ok(rows) = rows {
        for row in rows.flatten() {
            let (grantee, resource, write, rt) = row;
            grants.push(wafer_run::ResourceGrant {
                grantee,
                resource,
                write: write != 0,
                resource_type: rt.and_then(|s| wafer_run::ResourceType::parse(&s)),
            });
        }
    }

    grants
}
