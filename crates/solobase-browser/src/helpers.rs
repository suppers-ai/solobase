//! Shared helpers for the browser adapter crates (wasm32-only).
//!
//! These are thin wrappers kept here — rather than duplicated in `convert.rs`
//! and `database.rs` — because `solobase-browser` does not depend on
//! `solobase-core`. When the dependency is ever added, these can be deleted and
//! callers switched to `solobase_core::blocks::helpers::*`.

/// Current UTC time as RFC 3339 string.
pub(crate) fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Decode a percent-encoded (URL-encoded) string.
///
/// Decodes `+` as a space (form-urlencoding semantics) and `%XX` hex
/// sequences. Matches the canonical implementation in
/// `solobase_core::blocks::helpers::urlencoding_decode`.
pub(crate) fn urlencoding_decode(s: &str) -> String {
    let s = s.replace('+', " ");
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}
