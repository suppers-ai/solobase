//! Pure SQL-string and BLOB-packing helpers for `BrowserVectorService`.
//!
//! Kept dep-free and side-effect-free so they unit-test on native.

/// Returns the DDL statements to create a vector index. Tables:
/// - `{name}_vectors` — id PK, vector BLOB, metadata TEXT, [text TEXT]
/// - `{name}_fts` — fts5(id UNINDEXED, text) — only when keyword_search=true
/// - `{name}_meta` — id PK, rowid INTEGER, metadata TEXT, [text TEXT]
pub fn build_create_index_sql(prefixed_name: &str, keyword_search: bool) -> Vec<String> {
    let v = format!("{prefixed_name}_vectors");
    let m = format!("{prefixed_name}_meta");

    let text_col = if keyword_search { ", text TEXT" } else { "" };

    let mut out = vec![
        format!(
            r#"CREATE TABLE "{v}" (id TEXT PRIMARY KEY, vector BLOB NOT NULL, metadata TEXT{text_col})"#
        ),
    ];
    if keyword_search {
        let f = format!("{prefixed_name}_fts");
        out.push(format!(
            r#"CREATE VIRTUAL TABLE "{f}" USING fts5(id UNINDEXED, text)"#
        ));
    }
    out.push(format!(
        r#"CREATE TABLE "{m}" (id TEXT PRIMARY KEY, rowid INTEGER, metadata TEXT{text_col})"#
    ));
    out
}

pub fn build_delete_index_sql(prefixed_name: &str, keyword_search: bool) -> Vec<String> {
    let mut out = vec![format!(
        r#"DROP TABLE IF EXISTS "{prefixed_name}_vectors""#
    )];
    if keyword_search {
        out.push(format!(r#"DROP TABLE IF EXISTS "{prefixed_name}_fts""#));
    }
    out.push(format!(r#"DROP TABLE IF EXISTS "{prefixed_name}_meta""#));
    out
}

pub fn build_count_sql(prefixed_name: &str) -> String {
    format!(r#"SELECT COUNT(*) AS n FROM "{prefixed_name}_meta""#)
}

/// Returns `(statements, params)`. Statements share the same parameter list.
/// Each statement targets one of the index's tables.
pub fn build_delete_ids_sql(
    prefixed_name: &str,
    ids: &[String],
    keyword_search: bool,
) -> (Vec<String>, Vec<String>) {
    if ids.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let placeholders = vec!["?"; ids.len()].join(", ");
    let mut out = vec![format!(
        r#"DELETE FROM "{prefixed_name}_vectors" WHERE id IN ({placeholders})"#
    )];
    if keyword_search {
        out.push(format!(
            r#"DELETE FROM "{prefixed_name}_fts" WHERE id IN ({placeholders})"#
        ));
    }
    out.push(format!(
        r#"DELETE FROM "{prefixed_name}_meta" WHERE id IN ({placeholders})"#
    ));
    (out, ids.to_vec())
}

/// Pack `&[f32]` as little-endian bytes for storage in a sql.js BLOB column.
pub fn pack_vector_blob(v: &[f32]) -> Vec<u8> {
    let bytes: &[u8] = bytemuck::cast_slice(v);
    bytes.to_vec()
}

/// Unpack a BLOB into `Vec<f32>`. Errors if the byte length does not equal
/// `4 * expected_dims`.
pub fn parse_vector_blob(bytes: &[u8], expected_dims: u32) -> Result<Vec<f32>, String> {
    let want = (expected_dims as usize) * 4;
    if bytes.len() != want {
        return Err(format!(
            "vector blob length {} != expected {} ({}d × 4 bytes)",
            bytes.len(),
            want,
            expected_dims
        ));
    }
    let floats: &[f32] = bytemuck::try_cast_slice(bytes)
        .map_err(|e| format!("blob alignment error: {e}"))?;
    Ok(floats.to_vec())
}

#[derive(Clone, Debug)]
pub struct SqlUpsertEntry {
    pub id: String,
    /// Base64-encoded packed f32 BLOB.
    pub vector_blob_b64: String,
    pub metadata_json: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PreparedStmt {
    pub sql: String,
    /// JSON array string (the format `bridge::db_exec_raw` expects).
    pub params_json: String,
}

/// Builds `INSERT OR REPLACE` statements. One statement per table per row
/// keeps each blob param self-contained — sql.js's positional binding handles
/// strings and base64 blobs uniformly via the JSON-array convention used by
/// the rest of the bridge.
pub fn build_upsert_sql_stmts(
    prefixed_name: &str,
    keyword_search: bool,
    entries: &[SqlUpsertEntry],
) -> Vec<PreparedStmt> {
    let mut out = Vec::with_capacity(entries.len() * if keyword_search { 3 } else { 2 });
    for e in entries {
        let (sql_v, params_v) = if keyword_search {
            (
                format!(
                    r#"INSERT OR REPLACE INTO "{prefixed_name}_vectors" (id, vector, metadata, text) VALUES (?, base64_decode(?), ?, ?)"#
                ),
                serde_json::json!([
                    e.id,
                    e.vector_blob_b64,
                    e.metadata_json,
                    e.text.clone().unwrap_or_default()
                ]),
            )
        } else {
            (
                format!(
                    r#"INSERT OR REPLACE INTO "{prefixed_name}_vectors" (id, vector, metadata) VALUES (?, base64_decode(?), ?)"#
                ),
                serde_json::json!([e.id, e.vector_blob_b64, e.metadata_json]),
            )
        };
        out.push(PreparedStmt { sql: sql_v, params_json: params_v.to_string() });

        if keyword_search {
            out.push(PreparedStmt {
                sql: format!(
                    r#"INSERT OR REPLACE INTO "{prefixed_name}_fts" (id, text) VALUES (?, ?)"#
                ),
                params_json: serde_json::json!([e.id, e.text.clone().unwrap_or_default()])
                    .to_string(),
            });
        }

        let (sql_m, params_m) = if keyword_search {
            (
                format!(
                    r#"INSERT OR REPLACE INTO "{prefixed_name}_meta" (id, rowid, metadata, text) VALUES (?, NULL, ?, ?)"#
                ),
                serde_json::json!([
                    e.id,
                    e.metadata_json,
                    e.text.clone().unwrap_or_default()
                ]),
            )
        } else {
            (
                format!(
                    r#"INSERT OR REPLACE INTO "{prefixed_name}_meta" (id, rowid, metadata) VALUES (?, NULL, ?)"#
                ),
                serde_json::json!([e.id, e.metadata_json]),
            )
        };
        out.push(PreparedStmt { sql: sql_m, params_json: params_m.to_string() });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_index_with_keyword_emits_three_tables() {
        let sqls = build_create_index_sql("suppers_ai__vector__docs", true);
        assert_eq!(sqls.len(), 3);
        assert!(sqls[0].contains(r#"CREATE TABLE "suppers_ai__vector__docs_vectors""#));
        assert!(sqls[0].contains("vector BLOB"));
        assert!(sqls[0].contains("text TEXT"));
        assert!(sqls[1].contains(r#"CREATE VIRTUAL TABLE "suppers_ai__vector__docs_fts""#));
        assert!(sqls[1].contains("USING fts5(id UNINDEXED, text)"));
        assert!(sqls[2].contains(r#"CREATE TABLE "suppers_ai__vector__docs_meta""#));
        assert!(sqls[2].contains("text TEXT"), "expected _meta to include text column when keyword_search=true");
    }

    #[test]
    fn create_index_without_keyword_emits_two_tables() {
        let sqls = build_create_index_sql("suppers_ai__vector__docs", false);
        assert_eq!(sqls.len(), 2);
        assert!(sqls[0].contains("vectors"));
        assert!(!sqls[0].contains("text TEXT"));
        assert!(sqls[1].contains("meta"));
        assert!(!sqls[1].contains("text TEXT"), "expected _meta to omit text column when keyword_search=false");
        assert!(!sqls.iter().any(|s| s.contains("USING fts5")));
    }

    #[test]
    fn delete_index_drops_all_three_tables() {
        let sqls = build_delete_index_sql("suppers_ai__vector__docs", true);
        assert_eq!(sqls.len(), 3);
        assert!(sqls.iter().any(|s| s.contains("DROP TABLE IF EXISTS \"suppers_ai__vector__docs_vectors\"")));
        assert!(sqls.iter().any(|s| s.contains("DROP TABLE IF EXISTS \"suppers_ai__vector__docs_fts\"")));
        assert!(sqls.iter().any(|s| s.contains("DROP TABLE IF EXISTS \"suppers_ai__vector__docs_meta\"")));
    }

    #[test]
    fn delete_index_without_keyword_drops_two() {
        let sqls = build_delete_index_sql("suppers_ai__vector__docs", false);
        assert_eq!(sqls.len(), 2);
        assert!(!sqls.iter().any(|s| s.contains("_fts")));
    }

    #[test]
    fn count_sql_targets_meta_table() {
        assert_eq!(
            build_count_sql("suppers_ai__vector__docs"),
            r#"SELECT COUNT(*) AS n FROM "suppers_ai__vector__docs_meta""#
        );
    }

    #[test]
    fn delete_by_ids_uses_in_clause() {
        let (sqls, params) = build_delete_ids_sql("suppers_ai__vector__docs", &["a".into(), "b".into()], true);
        assert_eq!(sqls.len(), 3);
        assert!(sqls[0].contains(r#"DELETE FROM "suppers_ai__vector__docs_vectors" WHERE id IN (?, ?)"#));
        assert!(sqls[1].contains(r#"DELETE FROM "suppers_ai__vector__docs_fts" WHERE id IN (?, ?)"#));
        assert!(sqls[2].contains(r#"DELETE FROM "suppers_ai__vector__docs_meta" WHERE id IN (?, ?)"#));
        assert_eq!(params, vec!["a", "b"]);
    }

    #[test]
    fn delete_by_ids_empty_returns_no_statements() {
        let (sqls, params) = build_delete_ids_sql("suppers_ai__vector__docs", &[], true);
        assert!(sqls.is_empty());
        assert!(params.is_empty());
    }

    #[test]
    fn pack_then_unpack_roundtrip() {
        let v = vec![0.1f32, -0.5, 1e-7, f32::INFINITY, 0.0];
        let packed = pack_vector_blob(&v);
        assert_eq!(packed.len(), v.len() * 4);
        let unpacked = parse_vector_blob(&packed, v.len() as u32).expect("parse");
        assert_eq!(unpacked, v);
    }

    #[test]
    fn parse_rejects_wrong_byte_length() {
        let blob = vec![0u8; 10]; // not divisible by 4
        assert!(parse_vector_blob(&blob, 1).is_err());
    }

    #[test]
    fn parse_rejects_dimension_mismatch() {
        let v = vec![0.1f32; 4];
        let packed = pack_vector_blob(&v);
        assert!(parse_vector_blob(&packed, 5).is_err());
    }

    #[test]
    fn upsert_emits_three_statements_with_keyword() {
        let entry = SqlUpsertEntry {
            id: "doc1".into(),
            vector_blob_b64: "AAAA".into(),
            metadata_json: "{}".into(),
            text: Some("hello".into()),
        };
        let stmts = build_upsert_sql_stmts("suppers_ai__vector__docs", true, &[entry.clone()]);
        assert_eq!(stmts.len(), 3, "expected vectors + fts + meta upserts");
        assert!(stmts[0].sql.contains("INSERT OR REPLACE INTO \"suppers_ai__vector__docs_vectors\""));
        assert!(stmts[1].sql.contains("INSERT OR REPLACE INTO \"suppers_ai__vector__docs_fts\""));
        assert!(stmts[2].sql.contains("INSERT OR REPLACE INTO \"suppers_ai__vector__docs_meta\""));
    }

    #[test]
    fn upsert_without_keyword_skips_fts() {
        let entry = SqlUpsertEntry {
            id: "doc1".into(),
            vector_blob_b64: "AAAA".into(),
            metadata_json: "{}".into(),
            text: None,
        };
        let stmts = build_upsert_sql_stmts("suppers_ai__vector__docs", false, &[entry]);
        assert_eq!(stmts.len(), 2);
        assert!(!stmts.iter().any(|s| s.sql.contains("_fts")));
    }
}
