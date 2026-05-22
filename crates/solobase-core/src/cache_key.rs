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

use wafer_block::db::{FilterOp, ListOptions};

/// Minimum `limit` value treated as "all matching rows". Matches the
/// `D1ConfigSource` and admin block list shapes. Anything smaller is
/// treated as paginated and bypasses cache.
const FULL_LIMIT_THRESHOLD: i64 = 10_000;

/// Returns Some(kv_key) iff `opts` matches the canonical
/// "load all rows for one block" shape.
pub fn read_key(table: CachedTable, opts: &ListOptions) -> Option<String> {
    if opts.filters.len() != 1
        || !opts.skip_count
        || opts.offset != 0
        || opts.limit < FULL_LIMIT_THRESHOLD
        || !opts.sort.is_empty()
    {
        return None;
    }
    let f = &opts.filters[0];
    if !matches!(f.operator, FilterOp::Equal) {
        return None;
    }
    let expected_col = match table {
        CachedTable::Variables => "block",
        CachedTable::BlockSettings => "block_name",
    };
    if f.field != expected_col {
        return None;
    }
    let value_str = f.value.as_str()?;
    Some(format_key(table, value_str))
}

fn format_key(table: CachedTable, value: &str) -> String {
    let tag = match table {
        CachedTable::Variables => "variables",
        CachedTable::BlockSettings => "block_settings",
    };
    format!("cfg:v1:{tag}:{value}")
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

    use wafer_block::db::{Filter, FilterOp, ListOptions};

    fn canonical_opts(field: &str, value: &str) -> ListOptions {
        ListOptions {
            filters: vec![Filter {
                field: field.into(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String(value.into()),
            }],
            limit: 10_000,
            offset: 0,
            skip_count: true,
            ..Default::default()
        }
    }

    #[test]
    fn read_key_variables_canonical_shape() {
        let opts = canonical_opts("block", "SUPPERS_AI__AUTH");
        assert_eq!(
            read_key(CachedTable::Variables, &opts),
            Some("cfg:v1:variables:SUPPERS_AI__AUTH".to_string())
        );
    }

    #[test]
    fn read_key_block_settings_canonical_shape() {
        let opts = canonical_opts("block_name", "wafer-run/registry");
        assert_eq!(
            read_key(CachedTable::BlockSettings, &opts),
            Some("cfg:v1:block_settings:wafer-run/registry".to_string())
        );
    }

    #[test]
    fn read_key_wrong_column_returns_none() {
        let opts = canonical_opts("key", "SOME_VAR");
        assert_eq!(read_key(CachedTable::Variables, &opts), None);
    }

    #[test]
    fn read_key_multiple_filters_returns_none() {
        let mut opts = canonical_opts("block", "SUPPERS_AI__AUTH");
        opts.filters.push(Filter {
            field: "key".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String("JWT_SECRET".into()),
        });
        assert_eq!(read_key(CachedTable::Variables, &opts), None);
    }

    #[test]
    fn read_key_no_filters_returns_none() {
        let opts = ListOptions {
            limit: 10_000,
            skip_count: true,
            ..Default::default()
        };
        assert_eq!(read_key(CachedTable::Variables, &opts), None);
    }

    #[test]
    fn read_key_non_equal_op_returns_none() {
        let mut opts = canonical_opts("block", "SUPPERS_AI__AUTH");
        opts.filters[0].operator = FilterOp::NotEqual;
        assert_eq!(read_key(CachedTable::Variables, &opts), None);
    }

    #[test]
    fn read_key_skip_count_false_returns_none() {
        let mut opts = canonical_opts("block", "SUPPERS_AI__AUTH");
        opts.skip_count = false;
        assert_eq!(read_key(CachedTable::Variables, &opts), None);
    }

    #[test]
    fn read_key_nonzero_offset_returns_none() {
        let mut opts = canonical_opts("block", "SUPPERS_AI__AUTH");
        opts.offset = 100;
        assert_eq!(read_key(CachedTable::Variables, &opts), None);
    }

    #[test]
    fn read_key_small_limit_returns_none() {
        let mut opts = canonical_opts("block", "SUPPERS_AI__AUTH");
        opts.limit = 50;
        assert_eq!(read_key(CachedTable::Variables, &opts), None);
    }

    #[test]
    fn read_key_non_string_value_returns_none() {
        let mut opts = canonical_opts("block", "SUPPERS_AI__AUTH");
        opts.filters[0].value = serde_json::Value::Number(42.into());
        assert_eq!(read_key(CachedTable::Variables, &opts), None);
    }
}
