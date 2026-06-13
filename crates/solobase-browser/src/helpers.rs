//! Shared helpers for the browser adapter crates (wasm32-only).
//!
//! Thin wrappers kept here — rather than duplicated in `database.rs` — because
//! `solobase-browser` does not depend on `solobase-core`. When the dependency
//! is ever added, these can be deleted and callers switched to
//! `solobase_core::blocks::helpers::*`.

/// Current UTC time as RFC 3339 string.
pub(crate) fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}
