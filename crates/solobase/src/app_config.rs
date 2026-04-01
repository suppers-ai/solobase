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

        blocks.insert("wafer-run/http-listener".into(), json!({
            "flow": "site-main",
            "listen": self.listen
        }));

        blocks.insert("wafer-run/web".into(), json!({
            "web_root": "site",
            "web_spa": "true",
            "web_index": "index.html"
        }));

        (blocks, aliases)
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

// ---------------------------------------------------------------------------
// Block settings — read from block_settings table + BLOCK_DISABLED env var
// ---------------------------------------------------------------------------

/// Load block settings from the SQLite database.
///
/// Reads the `block_settings` table (if it exists) and returns a
/// `BlockSettings` with the enabled/disabled state of each block.
/// Also seeds from the `BLOCK_DISABLED` env var on first run.
/// Block default: (full_name, default_enabled).
/// Used to seed the block_settings table on first run.
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
    ("suppers-ai/email", true),
    ("suppers-ai/profile", true),
    ("suppers-ai/system", true),
];

pub fn load_block_settings(db_path: &str) -> BlockSettings {
    let conn = match rusqlite::Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return BlockSettings::from_map(HashMap::new()),
    };

    // Ensure block_settings table exists
    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS block_settings (
            block_name TEXT PRIMARY KEY,
            enabled INTEGER DEFAULT 1,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );"
    );

    // Seed defaults for known blocks (INSERT OR IGNORE — existing DB values take priority)
    {
        let mut stmt = conn.prepare(
            "INSERT OR IGNORE INTO block_settings (block_name, enabled) VALUES (?1, ?2)"
        ).unwrap();
        for &(name, default) in BLOCK_DEFAULTS {
            let _ = stmt.execute(rusqlite::params![name, default as i32]);
        }
    }

    // Seed from BLOCK_DISABLED env var (overrides defaults on first run)
    if let Ok(disabled) = std::env::var("BLOCK_DISABLED") {
        for name in disabled.split(',') {
            let name = name.trim();
            if !name.is_empty() {
                let _ = conn.execute(
                    "INSERT INTO block_settings (block_name, enabled) VALUES (?1, 0) \
                     ON CONFLICT (block_name) DO UPDATE SET enabled = 0",
                    rusqlite::params![name],
                );
            }
        }
    }

    // Read all settings
    let mut map = HashMap::new();
    if let Ok(mut stmt) = conn.prepare("SELECT block_name, enabled FROM block_settings") {
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i32>(1)? != 0,
            ))
        });
        if let Ok(rows) = rows {
            for row in rows.flatten() {
                map.insert(row.0, row.1);
            }
        }
    }

    BlockSettings::from_map(map)
}
