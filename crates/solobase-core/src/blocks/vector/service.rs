//! Table-name constants and helpers for the suppers-ai/vector block.
//!
//! Vector indexes are stored as tables in the underlying database with a
//! fixed prefix. User-facing index names (e.g. `"docs"`) are mapped to the
//! prefixed storage name (e.g. `"suppers_ai__vector__docs"`) at the block
//! boundary — no magic mapping elsewhere in the stack.

use wafer_run::types::{ErrorCode, WaferError};

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
/// to interpolate into SQL identifiers (alphanumeric, underscore, hyphen).
///
/// Index names flow through [`prefixed_index_name`] into SQL via
/// `format!` interpolation in several hot paths (e.g. the re-ingest
/// cleanup query in `handle_ingest`). Relying on the driver to reject
/// multi-statement input is not defense-in-depth; validating the name
/// at the route boundary protects every downstream SQL consumer uniformly.
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
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(WaferError {
            code: ErrorCode::InvalidArgument,
            message: format!("invalid index name '{name}': only [A-Za-z0-9_-] allowed"),
            meta: vec![],
        });
    }
    Ok(name)
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
        assert!(validate_index_name("index-42").is_ok());
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
    }
}
