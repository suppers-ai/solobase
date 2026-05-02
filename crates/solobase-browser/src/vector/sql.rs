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
}
