//! Table-name constants and helpers for the suppers-ai/vector block.
//!
//! Vector indexes are stored as tables in the underlying database with a
//! fixed prefix. User-facing index names (e.g. `"docs"`) are mapped to the
//! prefixed storage name (e.g. `"suppers_ai__vector__docs"`) at the block
//! boundary — no magic mapping elsewhere in the stack.

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
