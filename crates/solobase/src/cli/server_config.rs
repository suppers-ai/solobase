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
/// Read the `suppers_ai__admin__block_settings` table tolerantly and apply
/// any `SOLOBASE_BLOCK_ENABLED` / `SOLOBASE_BLOCK_DISABLED` env-var
/// overrides to the resulting in-memory map.
///
/// The table is **not** created here — admin block's migration
/// (`001_admin_schema.{sqlite,postgres}.sql`) is the single source of
/// schema truth. On a fresh boot the table doesn't exist yet; we return
/// whatever the env vars say (or an empty map, which `BlockSettings`
/// interprets as "all blocks enabled by default"). The first request
/// triggers admin block's lazy Init, which runs the migration and creates
/// the table with the canonical schema.
///
/// Env-var overrides affect only the in-memory snapshot fed into the
/// runtime — they aren't persisted to the DB. Operators who want a
/// persistent disable should set it via the admin UI; `SOLOBASE_BLOCK_*`
/// remains a boot-time override only.
pub fn load_block_settings(db_path: &str) -> BlockSettings {
    let mut map = HashMap::new();

    if let Ok(conn) = rusqlite::Connection::open(db_path) {
        if let Ok(mut stmt) =
            conn.prepare("SELECT block_name, enabled FROM suppers_ai__admin__block_settings")
        {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)? != 0))
            }) {
                for row in rows.flatten() {
                    map.insert(row.0, row.1);
                }
            }
        }
    }

    if let Ok(enabled) = std::env::var("SOLOBASE_BLOCK_ENABLED") {
        for name in enabled.split(',') {
            let name = name.trim();
            if !name.is_empty() {
                map.insert(name.to_string(), true);
            }
        }
    }
    if let Ok(disabled) = std::env::var("SOLOBASE_BLOCK_DISABLED") {
        for name in disabled.split(',') {
            let name = name.trim();
            if !name.is_empty() {
                map.insert(name.to_string(), false);
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
