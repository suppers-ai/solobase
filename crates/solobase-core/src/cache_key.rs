//! Pure functions translating `(table, ListOptions)` and `(table, row)`
//! into KV cache keys. Determines which `DatabaseService` calls qualify
//! for caching and how to derive their key.
//!
//! Consumed by `solobase-cloudflare::kv_cached_db`. Pure data-mapping
//! logic lives here so it's host-testable; `solobase-cloudflare` is
//! excluded from `cargo test --workspace`.

use crate::blocks::admin::{BLOCK_SETTINGS_TABLE, VARIABLES_TABLE};

/// Tables that this wrapper caches in KV.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachedTable {
    /// `suppers_ai__admin__variables` — config var rows, keyed by `block` column.
    Variables,
    /// `suppers_ai__admin__block_settings` — per-block migration state, keyed by `block_name` column.
    BlockSettings,
}

/// Returns Some when `table` is one of the cached tables.
pub fn classify_table(table: &str) -> Option<CachedTable> {
    match table {
        t if t == VARIABLES_TABLE => Some(CachedTable::Variables),
        t if t == BLOCK_SETTINGS_TABLE => Some(CachedTable::BlockSettings),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_table_variables() {
        assert_eq!(
            classify_table("suppers_ai__admin__variables"),
            Some(CachedTable::Variables)
        );
    }

    #[test]
    fn classify_table_block_settings() {
        assert_eq!(
            classify_table("suppers_ai__admin__block_settings"),
            Some(CachedTable::BlockSettings)
        );
    }

    #[test]
    fn classify_table_unknown_returns_none() {
        assert_eq!(classify_table("suppers_ai__auth__users"), None);
        assert_eq!(classify_table(""), None);
        assert_eq!(classify_table("variables"), None);
    }
}
