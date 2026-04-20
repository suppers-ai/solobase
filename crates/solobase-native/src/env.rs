//! Environment-variable bootstrap helpers for native WAFER apps.

use std::collections::HashMap;

/// Load `.env`. Honors `SOLOBASE_ENV_FILE` for an explicit path; otherwise
/// auto-detects `.env` in the current working directory. Failures on the
/// explicit-path form are logged to stderr but do not abort.
pub fn load_dotenv() {
    if let Ok(path) = std::env::var("SOLOBASE_ENV_FILE") {
        match dotenvy::from_filename(&path) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("warning: failed to load env file '{path}': {e}");
            }
        }
        return;
    }
    let _ = dotenvy::dotenv();
}

/// Collect env vars that are NOT prefixed `SOLOBASE_`. These are the
/// app-level config vars the consumer may want to seed into a config
/// service or a `variables` table.
///
/// Consumers who want additional filtering (e.g., only env vars that
/// match declared config var keys) should apply their own filter on top
/// of this result.
pub fn collect_app_env_vars() -> HashMap<String, String> {
    std::env::vars()
        .filter(|(k, _)| !k.starts_with("SOLOBASE_"))
        .collect()
}

/// Infrastructure config read from `SOLOBASE_*` env vars.
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
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
