//! Generate SQLite DDL from CollectionSchema declarations.
//!
//! `solobase build --target cloudflare` walks `all_block_infos()` and feeds
//! their collections into `generate_initial_schema()` to produce a single
//! migration SQL file consumed by `wrangler d1 migrations apply`.

use wafer_block::types::{CollectionSchema, IndexSchema};

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
    todo!("Task 5 will implement non-empty path")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_produces_empty_string() {
        assert_eq!(generate_initial_schema(&[]), "");
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
