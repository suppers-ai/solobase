//! Table-name constants and helpers for the suppers-ai/vector block.
//!
//! Vector indexes are stored as tables in the underlying database with a
//! fixed prefix. User-facing index names (e.g. `"docs"`) are mapped to the
//! prefixed storage name (e.g. `"suppers_ai__vector__docs"`) at the block
//! boundary — no magic mapping elsewhere in the stack.

use wafer_core::clients::database::{self as db, Filter, FilterOp, ListOptions, SortField};
use wafer_run::{
    context::Context,
    types::{ErrorCode, WaferError},
};
use wafer_sql_utils::{introspect, Backend};

/// Per-row data fed to the vector index list table renderer.
///
/// Lives in the service layer alongside the loader (`list_index_rows`) so
/// the data shape and its query stay co-located. The UI layer imports it
/// for rendering only.
#[derive(Clone, Debug)]
pub struct IndexRow {
    pub name: String,
    pub model: String,
    pub dimensions: u32,
    pub vector_count: u64,
    pub keyword_search: bool,
}

/// All tables created for a vector index are named with this prefix.
pub const TABLE_PREFIX: &str = "suppers_ai__vector__";

/// Convert a user-facing index name (e.g. `"docs"`) into the fully prefixed
/// name that is actually stored in the database (e.g. `"suppers_ai__vector__docs"`).
pub fn prefixed_index_name(user_name: &str) -> String {
    format!("{TABLE_PREFIX}{user_name}")
}

/// Strip the prefix for display to users. Returns the input unchanged if it
/// does not carry the prefix.
pub fn display_index_name(stored: &str) -> &str {
    stored.strip_prefix(TABLE_PREFIX).unwrap_or(stored)
}

/// Validate that an index name only contains characters that are safe
/// to interpolate into SQL identifiers (alphanumeric + underscore).
///
/// Index names flow through [`prefixed_index_name`] into SQL via
/// `format!` interpolation in several hot paths (e.g. the re-ingest
/// cleanup query in `handle_ingest`). Relying on the driver to reject
/// multi-statement input is not defense-in-depth; validating the name
/// at the route boundary protects every downstream SQL consumer uniformly.
///
/// The allowed set must match what `wafer_sql_utils::ident::sanitize_ident`
/// keeps. `sanitize_ident` strips everything non-alphanumeric except `_`,
/// so allowing hyphens here would diverge the registry name from the
/// actual SQL table name (e.g. `foo-bar` registered, but the SQL table
/// is `foobar_meta`) and break any `format!`-built query that reuses the
/// original name — like the re-ingest cleanup which would emit
/// `suppers_ai__vector__foo-bar_meta` (invalid SQL).
///
/// Returns the name on success so callers can chain it at the use site.
pub fn validate_index_name(name: &str) -> Result<&str, WaferError> {
    if name.is_empty() {
        return Err(WaferError {
            code: ErrorCode::InvalidArgument,
            message: "index name must not be empty".to_string(),
            meta: vec![],
        });
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(WaferError {
            code: ErrorCode::InvalidArgument,
            message: format!("invalid index name '{name}': only [A-Za-z0-9_] allowed"),
            meta: vec![],
        });
    }
    Ok(name)
}

/// Map one registry record into an `IndexRow`, querying the matching
/// `_meta` table for the live vector count. Returns `None` if the row
/// has no `prefixed_name`. Shared between the list and detail loaders
/// so the column-extraction quirks (TEXT-as-string round-trip from the
/// SQLite service) live in exactly one place.
async fn map_index_row(ctx: &dyn Context, rec: &db::Record) -> Option<IndexRow> {
    let storage_name = rec
        .data
        .get("prefixed_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if storage_name.is_empty() {
        return None;
    }
    let model = rec
        .data
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    // The SQLite service stores all auto-created columns as TEXT, so the
    // numeric registry values arrive as JSON strings (e.g. `"384"`)
    // rather than numbers. Try a number first for backends that round-trip
    // them faithfully, then fall back to parsing the string.
    let dimensions = rec
        .data
        .get("dimensions")
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0) as u32;
    let keyword_search = rec
        .data
        .get("keyword_search")
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .map(|n| n != 0)
        .unwrap_or(false);

    // Vectors live in `{prefixed}_meta` (see `pages.rs::ingest`/`reindex`),
    // not in the registry's `prefixed_name` table. Counting that meta
    // table is what gives a correct per-index vector count.
    let meta_table = format!("{storage_name}_meta");
    let count = db::count(ctx, &meta_table, &[]).await.unwrap_or(0);

    Some(IndexRow {
        name: storage_name,
        model,
        dimensions,
        vector_count: count.max(0) as u64,
        keyword_search,
    })
}

/// Read every registered vector index plus its current row count from
/// the meta table. Caller decides what to do with errors — returning
/// `Result` (rather than swallowing) keeps the helper testable in
/// isolation, while the page handler maps any failure to the empty
/// state.
///
/// On a fresh database the registry table doesn't exist yet; the
/// SQLite service returns an empty `RecordList` for unknown
/// collections, so `db::list` gives us `Ok(empty)` rather than an
/// error. Same for `db::count` against the per-index meta table.
pub async fn list_index_rows(ctx: &dyn Context) -> Result<Vec<IndexRow>, WaferError> {
    let opts = ListOptions {
        limit: 1000,
        sort: vec![SortField {
            field: "prefixed_name".to_string(),
            desc: false,
        }],
        ..Default::default()
    };
    let result = db::list(ctx, "suppers_ai__vector__registry", &opts).await?;

    let mut rows = Vec::with_capacity(result.records.len());
    for rec in result.records {
        if let Some(row) = map_index_row(ctx, &rec).await {
            rows.push(row);
        }
    }
    Ok(rows)
}

/// Look up a single registry row by its prefixed (storage) name. Returns
/// `None` if no such row exists. Used by the detail page handler.
pub async fn get_index_row(
    ctx: &dyn Context,
    storage_name: &str,
) -> Result<Option<IndexRow>, WaferError> {
    let opts = ListOptions {
        filters: vec![Filter {
            field: "prefixed_name".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(storage_name.to_string()),
        }],
        limit: 1,
        ..Default::default()
    };
    let result = db::list(ctx, "suppers_ai__vector__registry", &opts).await?;
    let Some(rec) = result.records.into_iter().next() else {
        return Ok(None);
    };
    Ok(map_index_row(ctx, &rec).await)
}

/// Introspect the columns of a per-index storage table, returning
/// `(name, sql_type)` pairs in declaration order.
///
/// Schema introspection is intrinsically backend-specific (SQLite
/// `PRAGMA table_info` vs Postgres `information_schema.columns`) — the
/// SQL is built via the `wafer_sql_utils::introspect::build_table_info`
/// portable builder so backends stay swappable, but the actual
/// projection still has to flow through `query_raw`. This is the same
/// pattern the admin database explorer uses and is the only path
/// available short of teaching `wafer-core::clients::database` a typed
/// `introspect_columns` API. On unknown tables the SQLite service
/// returns an empty result rather than erroring, so the detail page
/// renders cleanly even on a fresh DB.
pub async fn introspect_columns(
    ctx: &dyn Context,
    table: &str,
) -> Result<Vec<(String, String)>, WaferError> {
    let (sql, args) = introspect::build_table_info(table, Backend::Sqlite);
    let rows = db::query_raw(ctx, &sql, &args).await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| {
            let name = r.data.get("name").and_then(|v| v.as_str())?.to_string();
            let ty = r
                .data
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Some((name, ty))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_roundtrip() {
        let p = prefixed_index_name("docs");
        assert_eq!(p, "suppers_ai__vector__docs");
        assert_eq!(display_index_name(&p), "docs");
    }

    #[test]
    fn display_passes_through_unprefixed() {
        assert_eq!(display_index_name("other"), "other");
    }
}

#[cfg(test)]
mod tests_validate {
    use super::*;

    #[test]
    fn accepts_valid_names() {
        assert!(validate_index_name("docs").is_ok());
        assert!(validate_index_name("my_index").is_ok());
        assert!(validate_index_name("index_42").is_ok());
    }

    #[test]
    fn rejects_empty() {
        assert!(validate_index_name("").is_err());
    }

    #[test]
    fn rejects_special_chars() {
        assert!(validate_index_name("docs; DROP TABLE users").is_err());
        assert!(validate_index_name("doc's").is_err());
        assert!(validate_index_name("my.index").is_err());
        assert!(
            validate_index_name("index-42").is_err(),
            "hyphens no longer allowed — sanitize_ident strips them, \
             so the registry name and SQL table name would diverge"
        );
    }
}
