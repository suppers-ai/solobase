//! D1ConfigSource — Cloudflare target's [`ConfigSource`] impl.
//!
//! Reads block-declared env-var config keys from the admin block's
//! `suppers_ai__admin__variables` D1 table. Filters by the new `block`
//! column (added by migration 002) for an indexed per-block lookup — no
//! full-table scan, no `LIKE prefix%` scan.
//!
//! Optionally layers an in-memory overlay (e.g. `worker::Env` secrets such
//! as `SUPPERS_AI__AUTH__JWT_SECRET`) on top of the D1 rows. Overlay values
//! win over D1 — overlay represents CF env bindings that must override
//! whatever an admin happens to have stored in the variables table.
//!
//! Spec: docs/superpowers/specs/2026-05-15-lazy-block-init-design.md §2, §6

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use solobase_core::blocks::admin::VARIABLES_TABLE;
use wafer_block::ConfigVar;
use wafer_core::interfaces::database::service::{DatabaseService, Filter, FilterOp, ListOptions};
use wafer_run::{ConfigError, ConfigSource, EnvBlockConfig};

/// Reads block-declared config keys from a D1-backed
/// [`DatabaseService`], falling back to each [`ConfigVar`]'s `default`
/// when the row is missing or its `value` is empty.
///
/// Returns [`ConfigError::MissingRequired`] for keys with `optional ==
/// false` where neither the D1 row nor a non-empty default supplies a
/// value. D1 query failures surface as [`ConfigError::Transient`] —
/// callers may retry on the next request because the runtime does
/// not cache transient errors in the block slot.
///
pub struct D1ConfigSource {
    db: Arc<dyn DatabaseService>,
    /// Static overlay applied on top of D1 rows in `load_for_block`.
    /// Keys present here override D1 — used for `worker::Env` secrets
    /// (e.g. `SUPPERS_AI__AUTH__JWT_SECRET`) that must not live in D1.
    /// Empty when constructed via [`Self::new`].
    overlay: HashMap<String, String>,
}

impl D1ConfigSource {
    pub fn new(db: Arc<dyn DatabaseService>) -> Self {
        Self {
            db,
            overlay: HashMap::new(),
        }
    }

    /// Construct with a static overlay of values that win over D1 rows.
    /// Intended for `worker::Env` secrets such as
    /// `SUPPERS_AI__AUTH__JWT_SECRET` that admins manage via
    /// `wrangler secret put` rather than the admin dashboard.
    pub fn with_overlay(db: Arc<dyn DatabaseService>, overlay: HashMap<String, String>) -> Self {
        Self { db, overlay }
    }

    /// Map a kebab-case block name like `"suppers-ai/auth"` to the
    /// SCREAMING_SNAKE prefix stored in the `block` column.
    ///
    /// Conversion rules:
    /// - `-` → `_` (within each segment)
    /// - `/` → `__` (segment separator)
    /// - uppercase
    ///
    /// Examples:
    /// - `"suppers-ai/auth"` → `"SUPPERS_AI__AUTH"`
    /// - `"wafer-run/sqlite"` → `"WAFER_RUN__SQLITE"`
    pub(crate) fn screaming_block(name: &str) -> String {
        let (org, block) = name.split_once('/').unwrap_or((name, ""));
        let org_upper = org.replace('-', "_").to_uppercase();
        if block.is_empty() {
            org_upper
        } else {
            let block_upper = block.replace('-', "_").to_uppercase();
            format!("{org_upper}__{block_upper}")
        }
    }

    /// Fetch all rows in the variables table whose `block` column equals
    /// `screaming_block`. Uses [`DatabaseService::list`] with a single
    /// [`FilterOp::Equal`] filter; the new index on `(block)` (migration
    /// 002) makes this an indexed lookup, not a scan.
    pub(crate) async fn fetch_block_variables(
        &self,
        screaming_block: &str,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error + Send + Sync>> {
        let opts = ListOptions {
            filters: vec![Filter {
                field: "block".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String(screaming_block.to_string()),
            }],
            limit: 10_000,
            offset: 0,
            skip_count: true,
            ..Default::default()
        };
        let rows = self.db.list(VARIABLES_TABLE, &opts).await?;
        Ok(rows
            .records
            .into_iter()
            .filter_map(|r| {
                let key = r.data.get("key")?.as_str()?.to_string();
                let value = r.data.get("value")?.as_str()?.to_string();
                Some((key, value))
            })
            .collect())
    }

    /// Core resolution logic — applied per [`ConfigVar`] against rows
    /// already fetched from D1, with an optional overlay layered on top.
    /// Overlay entries win over D1 rows; both must contain non-empty
    /// values to be considered (an empty string falls through to the
    /// `ConfigVar::default` fallback).
    pub(crate) fn resolve(
        block: &str,
        rows: &HashMap<String, String>,
        overlay: &HashMap<String, String>,
        declared_keys: &[ConfigVar],
    ) -> Result<EnvBlockConfig, ConfigError> {
        let mut out = HashMap::with_capacity(declared_keys.len());
        for var in declared_keys {
            // Overlay wins (CF env secrets), then D1, then default.
            let from_overlay = overlay.get(&var.key).filter(|s| !s.is_empty()).cloned();
            let from_db = rows.get(&var.key).filter(|s| !s.is_empty()).cloned();
            let resolved = from_overlay.or(from_db).or_else(|| {
                if var.default.is_empty() {
                    None
                } else {
                    Some(var.default.clone())
                }
            });

            match resolved {
                Some(v) => {
                    out.insert(var.key.clone(), v);
                }
                None if !var.optional => {
                    return Err(ConfigError::MissingRequired {
                        block: block.to_string(),
                        key: var.key.clone(),
                    });
                }
                None => {
                    // optional + no value + no default: skip; the
                    // EnvBlockConfig::get() call returns None at the block.
                }
            }
        }
        Ok(EnvBlockConfig::new(out))
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ConfigSource for D1ConfigSource {
    async fn load_for_block(
        &self,
        block: &str,
        declared_keys: &[ConfigVar],
    ) -> Result<EnvBlockConfig, ConfigError> {
        let screaming = Self::screaming_block(block);
        let rows =
            self.fetch_block_variables(&screaming)
                .await
                .map_err(|e| ConfigError::Transient {
                    block: block.to_string(),
                    source: e,
                })?;
        Self::resolve(block, &rows, &self.overlay, declared_keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screaming_block_handles_two_segments() {
        assert_eq!(
            D1ConfigSource::screaming_block("suppers-ai/auth"),
            "SUPPERS_AI__AUTH"
        );
        assert_eq!(
            D1ConfigSource::screaming_block("wafer-run/sqlite"),
            "WAFER_RUN__SQLITE"
        );
    }

    #[test]
    fn screaming_block_handles_org_only() {
        assert_eq!(D1ConfigSource::screaming_block("suppers-ai"), "SUPPERS_AI");
    }

    #[test]
    fn resolve_returns_db_value_when_present() {
        let mut rows = HashMap::new();
        rows.insert("KEY".to_string(), "from-db".to_string());
        let overlay = HashMap::new();
        let declared = vec![ConfigVar::new("KEY", "doc", "default")];
        let cfg = D1ConfigSource::resolve("test/block", &rows, &overlay, &declared).unwrap();
        assert_eq!(cfg.get("KEY"), Some("from-db"));
    }

    #[test]
    fn resolve_falls_back_to_default_when_db_missing() {
        let rows = HashMap::new();
        let overlay = HashMap::new();
        let declared = vec![ConfigVar::new("KEY", "doc", "fallback")];
        let cfg = D1ConfigSource::resolve("test/block", &rows, &overlay, &declared).unwrap();
        assert_eq!(cfg.get("KEY"), Some("fallback"));
    }

    #[test]
    fn resolve_falls_back_to_default_when_db_value_empty() {
        let mut rows = HashMap::new();
        rows.insert("KEY".to_string(), "".to_string());
        let overlay = HashMap::new();
        let declared = vec![ConfigVar::new("KEY", "doc", "fallback")];
        let cfg = D1ConfigSource::resolve("test/block", &rows, &overlay, &declared).unwrap();
        assert_eq!(cfg.get("KEY"), Some("fallback"));
    }

    #[test]
    fn resolve_required_missing_returns_error() {
        let rows = HashMap::new();
        let overlay = HashMap::new();
        let declared = vec![ConfigVar::new("KEY", "doc", "")];
        let result = D1ConfigSource::resolve("test/block", &rows, &overlay, &declared);
        assert!(matches!(result, Err(ConfigError::MissingRequired { .. })));
    }

    #[test]
    fn resolve_optional_missing_is_skipped() {
        let rows = HashMap::new();
        let overlay = HashMap::new();
        let declared = vec![ConfigVar::new("KEY", "doc", "").optional()];
        let cfg = D1ConfigSource::resolve("test/block", &rows, &overlay, &declared).unwrap();
        assert_eq!(cfg.get("KEY"), None);
    }

    #[test]
    fn resolve_overlay_wins_over_db() {
        let mut rows = HashMap::new();
        rows.insert("KEY".to_string(), "from-db".to_string());
        let mut overlay = HashMap::new();
        overlay.insert("KEY".to_string(), "from-overlay".to_string());
        let declared = vec![ConfigVar::new("KEY", "doc", "default")];
        let cfg = D1ConfigSource::resolve("test/block", &rows, &overlay, &declared).unwrap();
        assert_eq!(cfg.get("KEY"), Some("from-overlay"));
    }

    #[test]
    fn resolve_overlay_supplies_required_value_when_db_empty() {
        let rows = HashMap::new();
        let mut overlay = HashMap::new();
        overlay.insert("KEY".to_string(), "secret".to_string());
        let declared = vec![ConfigVar::new("KEY", "doc", "")];
        let cfg = D1ConfigSource::resolve("test/block", &rows, &overlay, &declared).unwrap();
        assert_eq!(cfg.get("KEY"), Some("secret"));
    }
}
