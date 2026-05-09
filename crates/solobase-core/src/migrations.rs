//! Generate SQLite DDL from CollectionSchema declarations.
//!
//! `solobase build --target cloudflare` walks `all_block_infos()` and feeds
//! their collections into `generate_initial_schema()` to produce a single
//! migration SQL file consumed by `wrangler d1 migrations apply`.

use wafer_block::types::{CollectionSchema, IndexSchema};

/// Hand-authored migration SQL that can't be derived from `CollectionSchema`.
///
/// Auth declares its schema through the `service_blocks::auth::AuthBlock`,
/// whose `info()` method does not declare any `CollectionSchema` — so the
/// auto-generator can't reach those tables. They live as hand-authored SQL
/// shipped at `blocks/auth/migrations/*.sqlite.sql` and embedded here at
/// compile time.
///
/// Returned as `(filename, content)` pairs. The build script writes them
/// alongside the auto-generated `0001_initial_schema.sql`. Filenames begin
/// at `0002_` so `wrangler d1 migrations apply` runs them after the
/// auto-generated initial schema.
pub fn extra_migrations() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "0002_auth_schema.sql",
            include_str!("blocks/auth/migrations/001_auth_schema.sqlite.sql"),
        ),
        (
            "0003_reserved_orgs.sql",
            include_str!("blocks/auth/migrations/002_reserved_orgs.sqlite.sql"),
        ),
    ]
}

/// Render a single CREATE TABLE statement (without trailing semicolon).
pub fn render_create_table(schema: &CollectionSchema) -> String {
    let mut col_lines = vec!["    id TEXT PRIMARY KEY".to_string()];
    for f in &schema.fields {
        if matches!(f.name.as_str(), "id" | "created_at" | "updated_at") {
            continue; // emitted by the boilerplate
        }
        let mut line = format!("    {} {}", f.name, sql_type(&f.field_type));
        if f.unique {
            line.push_str(" UNIQUE");
        }
        col_lines.push(line);
    }
    col_lines.push("    created_at TEXT".to_string());
    col_lines.push("    updated_at TEXT".to_string());
    format!(
        "CREATE TABLE IF NOT EXISTS {} (\n{}\n)",
        schema.name,
        col_lines.join(",\n")
    )
}

/// Map CollectionSchema field_type strings to SQLite column types.
/// All non-text types fall back to TEXT — solobase historically stores
/// everything as text and parses on read.
fn sql_type(field_type: &str) -> &'static str {
    match field_type {
        "int" | "integer" | "i64" | "i32" => "INTEGER",
        "real" | "f64" | "f32" => "REAL",
        "blob" | "bytes" => "BLOB",
        _ => "TEXT",
    }
}

/// Render the full initial schema migration: CREATE TABLE for each collection,
/// followed by CREATE INDEX statements, separated by blank lines.
pub fn generate_initial_schema(collections: &[CollectionSchema]) -> String {
    if collections.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for s in collections {
        out.push_str(&render_create_table(s));
        out.push_str(";\n\n");
    }
    for s in collections {
        for idx in &s.indexes {
            out.push_str(&render_create_index(&s.name, idx));
            out.push('\n');
        }
    }
    out
}

fn render_create_index(table: &str, idx: &IndexSchema) -> String {
    let cols = idx.fields.join("__");
    let unique = if idx.unique { "UNIQUE " } else { "" };
    let cols_csv = idx.fields.join(", ");
    format!("CREATE {unique}INDEX IF NOT EXISTS idx_{table}__{cols} ON {table} ({cols_csv});")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_produces_empty_string() {
        assert_eq!(generate_initial_schema(&[]), "");
    }

    #[test]
    fn generate_initial_schema_emits_all_tables_and_indexes() {
        let widgets = CollectionSchema::new("suppers_ai__demo__widgets").field("name", "text");
        let things = CollectionSchema::new("suppers_ai__demo__things")
            .field("widget_id", "text")
            .index(&["widget_id"]);
        let sql = generate_initial_schema(&[widgets, things]);
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS suppers_ai__demo__widgets"));
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS suppers_ai__demo__things"));
        assert!(sql.contains(
            "CREATE INDEX IF NOT EXISTS idx_suppers_ai__demo__things__widget_id \
             ON suppers_ai__demo__things (widget_id);"
        ));
        assert!(sql.ends_with(";\n"));
    }

    #[test]
    fn render_create_table_skips_explicit_id_field() {
        let schema = CollectionSchema::new("suppers_ai__demo__rows")
            .field("id", "text")
            .field("label", "text")
            .field("created_at", "text")
            .field("updated_at", "text");
        let sql = render_create_table(&schema);
        let expected = "\
CREATE TABLE IF NOT EXISTS suppers_ai__demo__rows (
    id TEXT PRIMARY KEY,
    label TEXT,
    created_at TEXT,
    updated_at TEXT
)";
        assert_eq!(sql, expected);
    }

    #[test]
    fn render_create_table_unique_and_typed_fields() {
        let schema = CollectionSchema::new("suppers_ai__demo__items")
            .field_unique("slug", "text")
            .field("count", "int")
            .field("ratio", "real")
            .field("payload", "blob");
        let sql = render_create_table(&schema);
        let expected = "\
CREATE TABLE IF NOT EXISTS suppers_ai__demo__items (
    id TEXT PRIMARY KEY,
    slug TEXT UNIQUE,
    count INTEGER,
    ratio REAL,
    payload BLOB,
    created_at TEXT,
    updated_at TEXT
)";
        assert_eq!(sql, expected);
    }

    #[test]
    fn render_create_table_simple_text_fields() {
        let schema = CollectionSchema::new("suppers_ai__demo__widgets")
            .field("name", "text")
            .field("color", "text");
        let sql = render_create_table(&schema);
        let expected = "\
CREATE TABLE IF NOT EXISTS suppers_ai__demo__widgets (
    id TEXT PRIMARY KEY,
    name TEXT,
    color TEXT,
    created_at TEXT,
    updated_at TEXT
)";
        assert_eq!(sql, expected);
    }
}
