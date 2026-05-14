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
    filter_app_env_vars(std::env::vars())
}

/// Pure filter: drops any pair whose key starts with `SOLOBASE_`.
///
/// Split out so tests can exercise the filter without mutating the
/// process environment (which is `unsafe` in Rust 2024 and races with
/// parallel test runs).
pub(crate) fn filter_app_env_vars<I>(iter: I) -> HashMap<String, String>
where
    I: IntoIterator<Item = (String, String)>,
{
    iter.into_iter()
        .filter(|(k, _)| !k.starts_with("SOLOBASE_"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_app_env_vars_excludes_solobase_prefix() {
        let input = vec![
            ("SOLOBASE_INFRA_X".to_string(), "1".to_string()),
            ("APP_Y".to_string(), "2".to_string()),
            ("SOLOBASE_SHARED__FOO".to_string(), "3".to_string()),
            ("PATH".to_string(), "/usr/bin".to_string()),
        ];
        let out = filter_app_env_vars(input);
        assert!(!out.contains_key("SOLOBASE_INFRA_X"));
        assert!(!out.contains_key("SOLOBASE_SHARED__FOO"));
        assert_eq!(out.get("APP_Y").map(String::as_str), Some("2"));
        assert_eq!(out.get("PATH").map(String::as_str), Some("/usr/bin"));
    }

    #[test]
    fn filter_app_env_vars_empty_iterator_returns_empty_map() {
        let out = filter_app_env_vars(std::iter::empty());
        assert!(out.is_empty());
    }
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
