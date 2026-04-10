//! Infrastructure and feature configuration for solobase instances.
//!
//! All config comes from environment variables (loaded from `.env` on startup).
//!
//! - `SOLOBASE_*` prefix: infrastructure (process-only, never in variables table)
//! - Unprefixed: app config (seeded into variables table, dashboard-editable)

use std::collections::HashMap;

use serde_json::{json, Map, Value};
pub use solobase_core::features::BlockSettings;

// ---------------------------------------------------------------------------
// InfraConfig — read from SOLOBASE_* env vars
// ---------------------------------------------------------------------------

/// Infrastructure configuration read from `SOLOBASE_*` environment variables.
#[derive(Debug)]
pub struct InfraConfig {
    pub listen: String,
    pub db_type: String,
    pub db_path: String,
    pub db_url: Option<String>,
    pub storage_type: String,
    pub storage_root: String,
}

impl InfraConfig {
    pub fn from_env() -> Self {
        Self {
            listen: env_or("SOLOBASE_LISTEN", "0.0.0.0:8090"),
            db_type: env_or("SOLOBASE_DB_TYPE", "sqlite"),
            db_path: env_or("SOLOBASE_DB_PATH", "data/solobase.db"),
            db_url: std::env::var("SOLOBASE_DB_URL").ok(),
            storage_type: env_or("SOLOBASE_STORAGE_TYPE", "local"),
            storage_root: env_or("SOLOBASE_STORAGE_ROOT", "data/storage"),
        }
    }

    pub fn to_blocks_json(&self) -> (Map<String, Value>, Vec<(String, String)>) {
        let mut blocks = Map::new();
        let aliases: Vec<(String, String)> = Vec::new();

        blocks.insert(
            "wafer-run/http-listener".into(),
            json!({
                "flow": "site-main",
                "listen": self.listen
            }),
        );

        blocks.insert(
            "wafer-run/web".into(),
            json!({
                "web_root": "site",
                "web_spa": "true",
                "web_index": "index.html"
            }),
        );

        (blocks, aliases)
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

// ---------------------------------------------------------------------------
// Block settings — read from suppers_ai__admin__block_settings table + SOLOBASE_BLOCK_* env vars
// ---------------------------------------------------------------------------

/// Load block settings from the SQLite database.
///
/// Reads the `suppers_ai__admin__block_settings` table (if it exists) and returns a
/// `BlockSettings` with the enabled/disabled state of each block.
/// Also seeds from `SOLOBASE_BLOCK_ENABLED` / `SOLOBASE_BLOCK_DISABLED` env vars.
/// Block default: (full_name, default_enabled).
/// Used to seed the suppers_ai__admin__block_settings table on first run.
pub type BlockDefault = (&'static str, bool);

/// Known solobase blocks with their default enabled state.
/// Blocks not listed here default to enabled.
pub const BLOCK_DEFAULTS: &[BlockDefault] = &[
    ("suppers-ai/auth", true),
    ("suppers-ai/admin", true),
    ("suppers-ai/files", true),
    ("suppers-ai/products", true),
    ("suppers-ai/projects", false),
    ("suppers-ai/legalpages", false),
    ("suppers-ai/userportal", false),
    ("suppers-ai/system", true),
];

#[cfg(feature = "server")]
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

    // Seed defaults for known blocks (INSERT OR IGNORE — existing DB values take priority)
    {
        let mut stmt = conn
            .prepare("INSERT OR IGNORE INTO suppers_ai__admin__block_settings (block_name, enabled) VALUES (?1, ?2)")
            .unwrap();
        for &(name, default) in BLOCK_DEFAULTS {
            let _ = stmt.execute(rusqlite::params![name, default as i32]);
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
    if let Ok(mut stmt) = conn.prepare("SELECT block_name, enabled FROM suppers_ai__admin__block_settings") {
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
#[cfg(feature = "server")]
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
            let mut stmt = match conn.prepare(
                "SELECT grantee, resource, write FROM suppers_ai__admin__wrap_grants",
            ) {
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
