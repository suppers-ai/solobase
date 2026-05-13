//! Open-redirect validation shared across the auth-ui surface.
//!
//! [SEC-033] Login, signup, and OAuth callback all accept user-supplied
//! "next URL" values that get plugged into HTTP `Location` headers or
//! page-rendered `<a href>`. Without a tight check, an attacker can
//! craft links like `?redirect=//evil.com` or `?redirect=/\evil.com`
//! that some browsers/proxies route to a foreign origin.
//!
//! `is_safe_local_redirect` is the single canonical check: it accepts
//! paths that unambiguously stay on the current origin and rejects
//! anything that could be interpreted as protocol-relative, scheme-bearing,
//! or carrying header-injection control characters.
//!
//! Reject rules (case-insensitive for the percent-encoded forms):
//! - Doesn't start with `/`
//! - Starts with `//` (protocol-relative URL)
//! - Starts with `/\` (Windows protocol-relative — IE/Edge historically)
//! - Contains a backslash anywhere
//! - Contains `\r`, `\n`, `\t`, or any other ASCII control char
//! - Contains `%2F%2F` (encoded `//`) or `%5C` (encoded `\`)

/// Returns `true` only when `path` is safe to plug into a `Location:` header
/// or an `<a href>` without enabling an open redirect.
pub fn is_safe_local_redirect(path: &str) -> bool {
    // Must start with a single slash.
    if !path.starts_with('/') {
        return false;
    }
    // Reject `//foo` (protocol-relative) and `/\foo` (legacy IE protocol-relative).
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && (bytes[1] == b'/' || bytes[1] == b'\\') {
        return false;
    }
    // Reject any backslash or control character anywhere.
    if path
        .chars()
        .any(|c| c == '\\' || (c.is_control() && c != ' '))
    {
        return false;
    }
    // Reject URL-encoded forms that decode to the above.
    // Match case-insensitively so `%2f%2F`, `%5c`, etc. all fail.
    let lower = path.to_ascii_lowercase();
    if lower.contains("%2f%2f") || lower.contains("%5c") {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_relative_paths() {
        assert!(is_safe_local_redirect("/"));
        assert!(is_safe_local_redirect("/b/admin/"));
        assert!(is_safe_local_redirect("/b/admin/users?page=2"));
        assert!(is_safe_local_redirect("/path/with/multiple/segments"));
        assert!(is_safe_local_redirect("/with-fragment#section"));
    }

    #[test]
    fn rejects_non_slash_prefix() {
        assert!(!is_safe_local_redirect(""));
        assert!(!is_safe_local_redirect("foo"));
        assert!(!is_safe_local_redirect("https://evil.com"));
        assert!(!is_safe_local_redirect("javascript:alert(1)"));
    }

    #[test]
    fn rejects_protocol_relative() {
        assert!(!is_safe_local_redirect("//evil.com"));
        assert!(!is_safe_local_redirect("//evil.com/path"));
    }

    #[test]
    fn rejects_backslash_protocol_relative() {
        assert!(!is_safe_local_redirect("/\\evil.com"));
        assert!(!is_safe_local_redirect("/\\evil.com/path"));
    }

    #[test]
    fn rejects_backslash_anywhere() {
        assert!(!is_safe_local_redirect("/path\\with\\backslash"));
        assert!(!is_safe_local_redirect("/trailing\\"));
    }

    #[test]
    fn rejects_crlf_and_control_chars() {
        assert!(!is_safe_local_redirect(
            "/foo\r\nLocation: https://evil.com"
        ));
        assert!(!is_safe_local_redirect("/foo\nbar"));
        assert!(!is_safe_local_redirect("/foo\rbar"));
        assert!(!is_safe_local_redirect("/foo\tbar"));
        assert!(!is_safe_local_redirect("/foo\x07bar"));
        assert!(!is_safe_local_redirect("/foo\x00bar"));
    }

    #[test]
    fn rejects_encoded_double_slash() {
        assert!(!is_safe_local_redirect("/%2F%2Fevil.com"));
        assert!(!is_safe_local_redirect("/%2f%2fevil.com"));
        assert!(!is_safe_local_redirect("/%2F%2Fevil"));
        assert!(!is_safe_local_redirect("/path?%2F%2Fnested"));
    }

    #[test]
    fn rejects_encoded_backslash() {
        assert!(!is_safe_local_redirect("/%5Cevil.com"));
        assert!(!is_safe_local_redirect("/%5cevil.com"));
        assert!(!is_safe_local_redirect("/path%5cmore"));
    }
}
