//! Infrastructure and feature configuration for solobase instances.
//!
//! All config comes from environment variables (loaded from `.env` on startup).
//!
//! - `SOLOBASE_*` prefix: infrastructure (process-only, never in variables table)
//! - Unprefixed: app config (seeded into variables table, dashboard-editable)
//!
//! ```sh
//! # Infrastructure
//! SOLOBASE_LISTEN=0.0.0.0:8090
//! SOLOBASE_DB_TYPE=sqlite
//! SOLOBASE_DB_PATH=data/solobase.db
//! SOLOBASE_STORAGE_TYPE=local
//! SOLOBASE_STORAGE_ROOT=data/storage
//!
//! # App config
//! APP_NAME=My App
//! JWT_SECRET=secret
//! FEATURE_AUTH=true
//! ```

use std::collections::HashMap;

use serde_json::{json, Map, Value};
use solobase_core::features;

// ---------------------------------------------------------------------------
// InfraConfig — read from SOLOBASE_* env vars
// ---------------------------------------------------------------------------

/// Infrastructure configuration read from `SOLOBASE_*` environment variables.
///
/// These are process-level settings needed before the database exists.
/// They are NOT stored in the variables table.
#[derive(Debug)]
pub struct InfraConfig {
    /// HTTP listen address.
    pub listen: String,
    /// Database type: "sqlite" or "postgres".
    pub db_type: String,
    /// Path to the SQLite database file.
    pub db_path: String,
    /// PostgreSQL connection URL.
    pub db_url: Option<String>,
    /// Storage type: "local" or "s3".
    pub storage_type: String,
    /// Root directory for local storage.
    pub storage_root: String,
}

impl InfraConfig {
    /// Read infrastructure config from `SOLOBASE_*` environment variables.
    /// All values have sensible defaults.
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

    /// Expand this config into infrastructure block configs + aliases for WAFER.
    pub fn to_blocks_json(&self) -> (Map<String, Value>, Vec<(String, String)>) {
        let mut blocks = Map::new();
        let mut aliases: Vec<(String, String)> = Vec::new();

        // HTTP listener
        blocks.insert("wafer-run/http-listener".into(), json!({
            "flow": "site-main",
            "listen": self.listen
        }));

        // Web (SPA frontend)
        blocks.insert("wafer-run/web".into(), json!({
            "web_root": "site",
            "web_spa": "true",
            "web_index": "index.html"
        }));

        // Database
        let db_block_name = match self.db_type.as_str() {
            "postgres" | "postgresql" => "wafer-run/postgres",
            _ => "wafer-run/sqlite",
        };
        let mut db_config = json!({ "path": self.db_path });
        if let Some(ref url) = self.db_url {
            db_config["url"] = json!(url);
        }
        blocks.insert(db_block_name.into(), db_config);
        aliases.push(("db".into(), db_block_name.into()));
        aliases.push(("wafer-run/database".into(), db_block_name.into()));

        // Storage
        let storage_block_name = match self.storage_type.as_str() {
            "s3" => "wafer-run/s3",
            _ => "wafer-run/local-storage",
        };
        blocks.insert(storage_block_name.into(), json!({ "root": self.storage_root }));
        aliases.push(("storage".into(), storage_block_name.into()));
        aliases.push(("wafer-run/storage".into(), storage_block_name.into()));

        // Remaining infrastructure blocks
        blocks.insert("wafer-run/network".into(), json!({}));
        blocks.insert("wafer-run/logger".into(), json!({}));

        (blocks, aliases)
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

// ---------------------------------------------------------------------------
// FeatureSnapshot — built from the variables table
// ---------------------------------------------------------------------------

/// Frozen snapshot of feature flags, suitable for sharing via `Arc`.
///
/// Built from the `variables` table at startup. Feature keys use the
/// `FEATURE_{NAME}` convention (e.g. `FEATURE_AUTH=true`). All features
/// are enabled by default if the corresponding variable is absent.
pub struct FeatureSnapshot {
    pub auth: bool,
    pub admin: bool,
    pub files: bool,
    pub products: bool,
    pub projects: bool,
    pub legalpages: bool,
    pub userportal: bool,
}

impl FeatureSnapshot {
    /// Build a feature snapshot from a variables HashMap.
    ///
    /// Checks `FEATURE_AUTH`, `FEATURE_ADMIN`, etc. Features default to
    /// enabled if the key is absent. Set to `"false"` to disable.
    pub fn from_vars(vars: &HashMap<String, String>) -> Self {
        fn enabled(vars: &HashMap<String, String>, key: &str) -> bool {
            vars.get(key).map_or(true, |v| v != "false")
        }
        Self {
            auth: enabled(vars, "FEATURE_AUTH"),
            admin: enabled(vars, "FEATURE_ADMIN"),
            files: enabled(vars, "FEATURE_FILES"),
            products: enabled(vars, "FEATURE_PRODUCTS"),
            projects: enabled(vars, "FEATURE_PROJECTS"),
            legalpages: enabled(vars, "FEATURE_LEGALPAGES"),
            userportal: enabled(vars, "FEATURE_USERPORTAL"),
        }
    }

    /// Check if a feature is enabled by name (as used by `blocks::create_blocks`).
    pub fn is_enabled(&self, name: &str) -> bool {
        match name {
            "auth" => self.auth,
            "admin" => self.admin,
            "files" => self.files,
            "products" => self.products,
            "projects" => self.projects,
            "legalpages" => self.legalpages,
            "userportal" => self.userportal,
            // system and profile are always enabled
            "system" | "profile" => true,
            _ => false,
        }
    }
}

impl features::FeatureConfig for FeatureSnapshot {
    fn auth_enabled(&self) -> bool { self.auth }
    fn admin_enabled(&self) -> bool { self.admin }
    fn files_enabled(&self) -> bool { self.files }
    fn products_enabled(&self) -> bool { self.products }
    fn projects_enabled(&self) -> bool { self.projects }
    fn legalpages_enabled(&self) -> bool { self.legalpages }
    fn userportal_enabled(&self) -> bool { self.userportal }
}
