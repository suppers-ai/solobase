//! Encode/decode of the short-lived `wafer_oauth_state` cookie used to carry
//! the anti-CSRF `state` value, the PKCE `code_verifier`, and the post-login
//! `next` redirect across the provider round-trip.
//!
//! Shape: JSON-serialize the [`StatePayload`], base64url-encode the bytes,
//! stuff the result into a `Set-Cookie` value with `HttpOnly`, `Secure`,
//! `SameSite=Lax`, `Max-Age=600`. Browsers treat the 10-minute TTL as the
//! outer bound on the OAuth dance; expiry + clear-on-callback are the two
//! paths by which the cookie goes away.

use base64ct::{Base64UrlUnpadded, Encoding};
use serde::{Deserialize, Serialize};

/// Name of the cookie on both the wire and in lookups.
pub const COOKIE_NAME: &str = "wafer_oauth_state";

/// Lifetime of the state cookie in seconds (10 minutes — the authorize→
/// callback round-trip budget).
pub const MAX_AGE_SECONDS: u32 = 600;

/// Payload serialised into the cookie. `next` is optional so a login flow
/// can omit it and fall back to the block's default post-login destination.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatePayload {
    pub state: String,
    pub pkce_verifier: String,
    pub next: Option<String>,
}

/// Errors returned by [`parse_cookie_value`] and [`set_cookie_header`].
#[derive(Debug, thiserror::Error)]
pub enum StateCookieError {
    #[error("serialize: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("base64 decode: {0}")]
    Decode(String),
    #[error("parse: {0}")]
    Parse(String),
}

/// Build the `Set-Cookie` header value that persists `payload`.
///
/// JSON → base64url(unpadded) → `wafer_oauth_state=<encoded>; …`. The JSON
/// layer means tampered cookies fail the `serde_json::from_slice` step in
/// [`parse_cookie_value`] rather than silently producing garbage.
pub fn set_cookie_header(payload: &StatePayload) -> Result<String, StateCookieError> {
    let json = serde_json::to_vec(payload)?;
    let encoded = Base64UrlUnpadded::encode_string(&json);
    Ok(format!(
        "{COOKIE_NAME}={encoded}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age={MAX_AGE_SECONDS}"
    ))
}

/// Build a `Set-Cookie` header that clears the cookie (`Max-Age=0`). Emitted
/// by the callback handler immediately on entry so a replayed callback URL
/// can't be abused.
pub fn clear_cookie_header() -> String {
    format!("{COOKIE_NAME}=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0")
}

/// Parse a raw cookie value (the part after `wafer_oauth_state=`) back into
/// a [`StatePayload`]. Rejects non-base64url input and JSON that doesn't
/// match the payload shape.
pub fn parse_cookie_value(raw: &str) -> Result<StatePayload, StateCookieError> {
    let bytes =
        Base64UrlUnpadded::decode_vec(raw).map_err(|e| StateCookieError::Decode(e.to_string()))?;
    let payload: StatePayload =
        serde_json::from_slice(&bytes).map_err(|e| StateCookieError::Parse(e.to_string()))?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_header_has_httponly_secure_lax_600() {
        let payload = StatePayload {
            state: "s".into(),
            pkce_verifier: "v".into(),
            next: Some("/dashboard".into()),
        };
        let header = set_cookie_header(&payload).unwrap();
        assert!(header.starts_with("wafer_oauth_state="));
        assert!(header.contains("HttpOnly"));
        assert!(header.contains("Secure"));
        assert!(header.contains("SameSite=Lax"));
        assert!(header.contains("Max-Age=600"));
        assert!(header.contains("Path=/"));
    }

    #[test]
    fn clear_header_max_age_zero() {
        let header = clear_cookie_header();
        assert!(header.contains("Max-Age=0"));
        assert!(header.starts_with("wafer_oauth_state="));
    }

    #[test]
    fn roundtrip_encode_parse() {
        let payload = StatePayload {
            state: "abc".into(),
            pkce_verifier: "xyz".into(),
            next: Some("/x".into()),
        };
        let header = set_cookie_header(&payload).unwrap();
        let raw = header
            .strip_prefix("wafer_oauth_state=")
            .unwrap()
            .split(';')
            .next()
            .unwrap();
        let got = parse_cookie_value(raw).unwrap();
        assert_eq!(got, payload);
    }

    #[test]
    fn roundtrip_with_no_next() {
        let payload = StatePayload {
            state: "abc".into(),
            pkce_verifier: "xyz".into(),
            next: None,
        };
        let header = set_cookie_header(&payload).unwrap();
        let raw = header
            .strip_prefix("wafer_oauth_state=")
            .unwrap()
            .split(';')
            .next()
            .unwrap();
        let got = parse_cookie_value(raw).unwrap();
        assert_eq!(got, payload);
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(parse_cookie_value("not-base64-!!!").is_err());
    }

    #[test]
    fn parse_rejects_tampered_json() {
        // Valid base64url of bytes that don't deserialise to StatePayload.
        let bogus = Base64UrlUnpadded::encode_string(b"{\"state\":1}");
        assert!(parse_cookie_value(&bogus).is_err());
    }
}
