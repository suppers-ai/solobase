//! Generate SQLite DDL from CollectionSchema declarations.
//!
//! `solobase build --target cloudflare` walks `all_block_infos()` and feeds
//! their collections into `generate_initial_schema()` to produce a single
//! migration SQL file consumed by `wrangler d1 migrations apply`.

use wafer_block::types::{CollectionSchema, IndexSchema};

/// Render a single CREATE TABLE statement (without trailing semicolon).
pub fn render_create_table(schema: &CollectionSchema) -> String {
    todo!("Task 2")
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
}
