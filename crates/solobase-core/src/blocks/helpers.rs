use std::collections::HashMap;

use serde::Serialize;
use sha2::{Digest, Sha256};
use wafer_core::clients::database::Record;
use wafer_run::{
    ErrorCode, MetaEntry, OutputStream, WaferError, META_RESP_CONTENT_TYPE,
    META_RESP_COOKIE_PREFIX, META_RESP_HEADER_PREFIX, META_RESP_STATUS,
};

/// Current UTC time as RFC 3339 string.
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Current time in milliseconds (wasm-safe — uses chrono which uses js_sys on wasm32).
pub fn now_millis() -> u64 {
    chrono::Utc::now().timestamp_millis() as u64
}

/// Convert a serde_json::json!({...}) value into a HashMap for the database client.
pub fn json_map(val: serde_json::Value) -> HashMap<String, serde_json::Value> {
    match val {
        serde_json::Value::Object(map) => map.into_iter().collect(),
        _ => HashMap::new(),
    }
}

/// Extension trait for convenient field access on database Records.
pub trait RecordExt {
    fn str_field(&self, key: &str) -> &str;
    fn i64_field(&self, key: &str) -> i64;
    fn bool_field(&self, key: &str) -> bool;
}

impl RecordExt for Record {
    fn str_field(&self, key: &str) -> &str {
        self.data.get(key).and_then(|v| v.as_str()).unwrap_or("")
    }

    fn i64_field(&self, key: &str) -> i64 {
        self.data.get(key).and_then(|v| v.as_i64()).unwrap_or(0)
    }

    fn bool_field(&self, key: &str) -> bool {
        match self.data.get(key) {
            Some(serde_json::Value::Bool(b)) => *b,
            Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
            Some(serde_json::Value::String(s)) => s == "true" || s == "1",
            _ => false,
        }
    }
}

/// Get a field value as a string regardless of whether the DB returned it as string or number.
pub fn field_as_string(record: &Record, key: &str) -> String {
    match record.data.get(key) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// Insert created_at + updated_at timestamps into a data map.
pub fn stamp_created(data: &mut std::collections::HashMap<String, serde_json::Value>) {
    let now = now_rfc3339();
    data.entry("created_at".to_string())
        .or_insert_with(|| serde_json::Value::String(now.clone()));
    data.entry("updated_at".to_string())
        .or_insert_with(|| serde_json::Value::String(now));
}

/// Insert updated_at timestamp into a data map.
pub fn stamp_updated(data: &mut std::collections::HashMap<String, serde_json::Value>) {
    data.insert(
        "updated_at".to_string(),
        serde_json::Value::String(now_rfc3339()),
    );
}

/// Encode bytes as lowercase hex string.
pub fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        // SAFETY: writing to a String via fmt::Write never fails (the only
        // error variant in fmt::Error is for formatter errors that String
        // doesn't surface).
        let _ = write!(s, "{:02x}", b);
    }
    s
}

/// Check if the current user has admin role from the message metadata.
pub fn is_admin(msg: &wafer_run::Message) -> bool {
    msg.get_meta("auth.user_roles")
        .split(',')
        .any(|r| r.trim() == "admin")
}

/// Compute SHA-256 and return as hex string. Used for deterministic hashing (API keys, etc.).
pub fn sha256_hex(data: &[u8]) -> String {
    hex_encode(&sha256(data))
}

/// Compute SHA-256 hash.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Percent-encode a string for use as a URL path segment. Encodes everything
/// except RFC 3986 unreserved characters (`A-Z a-z 0-9 - _ . ~`). Spaces become
/// `%20`, `/` becomes `%2F`, etc. Use this when constructing `<a href>` URLs
/// from caller-supplied data (object keys, bucket names, etc.) — maud's HTML
/// escaping does NOT URL-encode.
pub fn url_path_encode(s: &str) -> String {
    s.as_bytes()
        .iter()
        .map(|&b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                String::from(b as char)
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

/// Percent-encode a string for use as an OAuth / `application/x-www-form-urlencoded`
/// query parameter. Delegates to [`url::form_urlencoded::byte_serialize`] which
/// encodes spaces as `+` and everything outside the unreserved set as `%XX`.
/// Prefer this over hand-rolling percent-encoding in OAuth / form-body contexts.
pub fn urlencode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

/// Percent-encode a string for use as an `application/x-www-form-urlencoded`
/// value. Same alphabet as [`url_path_encode`] but spaces become `+` (per the
/// form-encoding convention) rather than `%20`. Use this for HTTP form bodies
/// and `multipart/form-data` field values.
pub fn form_url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                use std::fmt::Write;
                let _ = write!(out, "%{:02X}", b);
            }
        }
    }
    out
}

/// Decode a percent-encoded (URL-encoded) string.
pub fn urlencoding_decode(s: &str) -> String {
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

/// Parse URL-encoded form body (htmx default) into a HashMap.
pub fn parse_form_body(data: &[u8]) -> HashMap<String, String> {
    let body = String::from_utf8_lossy(data);
    let mut map = HashMap::new();
    for pair in body.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            let key = urlencoding_decode(k);
            let value = urlencoding_decode(v);
            map.insert(key, value);
        }
    }
    map
}

/// Parse a request body as either JSON or URL-encoded form into a JSON Value.
///
/// Inspects the first non-whitespace byte: `{` → JSON, anything else →
/// URL-encoded form (then promoted to a flat object). Lets one handler
/// accept both htmx form posts and programmatic JSON clients without
/// duplicating parse logic.
pub fn parse_body_value(data: &[u8]) -> serde_json::Value {
    let trimmed_start = data
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(0);
    if data.get(trimmed_start) == Some(&b'{') || data.get(trimmed_start) == Some(&b'[') {
        serde_json::from_slice(data).unwrap_or(serde_json::Value::Null)
    } else {
        let mut obj = serde_json::Map::new();
        for (k, v) in parse_form_body(data) {
            obj.insert(k, serde_json::Value::String(v));
        }
        serde_json::Value::Object(obj)
    }
}

// ---------------------------------------------------------------------------
// Response construction helpers for the streaming block protocol.
// ---------------------------------------------------------------------------

/// Serialize `value` to JSON and return a successful `OutputStream`
/// with `Content-Type: application/json`.
/// Returns `OutputStream::error(Internal)` if serialization fails.
pub fn ok_json<T: Serialize>(value: &T) -> OutputStream {
    match serde_json::to_vec(value) {
        Ok(body) => OutputStream::respond_with_meta(
            body,
            vec![MetaEntry {
                key: META_RESP_CONTENT_TYPE.to_string(),
                value: "application/json".to_string(),
            }],
        ),
        Err(e) => err_internal("response serialization failed", e),
    }
}

/// Return an empty 200-OK `OutputStream` (no body).
pub fn ok_empty() -> OutputStream {
    OutputStream::respond(vec![])
}

/// Return a 400 Bad Request `OutputStream`.
pub fn err_bad_request(message: &str) -> OutputStream {
    OutputStream::error(WaferError {
        code: ErrorCode::InvalidArgument,
        message: message.to_string(),
        meta: vec![],
    })
}

/// Return a 401 Unauthorized `OutputStream`.
pub fn err_unauthorized(message: &str) -> OutputStream {
    OutputStream::error(WaferError {
        code: ErrorCode::Unauthenticated,
        message: message.to_string(),
        meta: vec![],
    })
}

/// Return a 403 Forbidden `OutputStream`.
pub fn err_forbidden(message: &str) -> OutputStream {
    OutputStream::error(WaferError {
        code: ErrorCode::PermissionDenied,
        message: message.to_string(),
        meta: vec![],
    })
}

/// Return a 404 Not Found `OutputStream`.
pub fn err_not_found(message: &str) -> OutputStream {
    OutputStream::error(WaferError {
        code: ErrorCode::NotFound,
        message: message.to_string(),
        meta: vec![],
    })
}

/// Return a 409 Conflict `OutputStream`.
pub fn err_conflict(message: &str) -> OutputStream {
    OutputStream::error(WaferError {
        code: ErrorCode::AlreadyExists,
        message: message.to_string(),
        meta: vec![],
    })
}

/// Return a 500 Internal Server Error `OutputStream`.
///
/// **Do not pass raw error text to the client.** This helper generates a short
/// correlation ID, logs the full error detail server-side via
/// [`tracing::error!`], and returns only `"Internal server error (ref: <id>)"`
/// to the caller. The `context` argument is a short, fixed label (no
/// interpolated error text) used for log grouping — e.g. `"Database error"`,
/// `"Storage error"`, `"Failed to update profile"`. The `error` argument is
/// the underlying error/cause; it is logged but NEVER echoed to the client.
///
/// ```ignore
/// .map_err(|e| err_internal("Database error", e))
/// ```
pub fn err_internal<E: std::fmt::Display>(context: &str, error: E) -> OutputStream {
    let id = correlation_id();
    tracing::error!(
        correlation_id = %id,
        error = %error,
        context = %context,
        "internal error",
    );
    OutputStream::error(WaferError {
        code: ErrorCode::Internal,
        message: format!("Internal server error (ref: {})", id),
        meta: vec![],
    })
}

/// 500 Internal Server Error for the rare case where there is no underlying
/// cause to log (e.g. an internal invariant violation with a static label).
/// Still logs the context with a correlation ID and returns the sanitized
/// `"Internal server error (ref: <id>)"` message. Most callers should prefer
/// [`err_internal`] which captures the underlying error.
pub fn err_internal_no_cause(context: &str) -> OutputStream {
    let id = correlation_id();
    tracing::error!(
        correlation_id = %id,
        context = %context,
        "internal error (no cause)",
    );
    OutputStream::error(WaferError {
        code: ErrorCode::Internal,
        message: format!("Internal server error (ref: {})", id),
        meta: vec![],
    })
}

/// 8-byte hex correlation ID — short enough to be quotable in support tickets,
/// random enough to grep logs without collisions.
fn correlation_id() -> String {
    let mut buf = [0u8; 8];
    if getrandom::getrandom(&mut buf).is_err() {
        // Fall back to timestamp-derived ID; correlation IDs are diagnostic
        // aids, not security primitives, so a deterministic fallback is fine.
        let nanos = now_millis();
        buf.copy_from_slice(&nanos.to_be_bytes());
    }
    hex_encode(&buf)
}

// ---------------------------------------------------------------------------
// ResponseBuilder — builds an OutputStream with status, headers, cookies, and body/JSON.
// ---------------------------------------------------------------------------

/// Build a response `OutputStream` with custom status, headers, and cookies.
pub struct ResponseBuilder {
    meta: Vec<MetaEntry>,
    cookie_count: usize,
}

impl ResponseBuilder {
    /// Create a new empty response builder.
    pub fn new() -> Self {
        Self {
            meta: Vec::new(),
            cookie_count: 0,
        }
    }

    /// Set an explicit HTTP status code (e.g. 201, 301, 302).
    pub fn status(mut self, status: u16) -> Self {
        self.meta.push(MetaEntry {
            key: META_RESP_STATUS.to_string(),
            value: status.to_string(),
        });
        self
    }

    /// Add a response header.
    pub fn set_header(mut self, key: &str, value: &str) -> Self {
        self.meta.push(MetaEntry {
            key: format!("{}{}", META_RESP_HEADER_PREFIX, key),
            value: value.to_string(),
        });
        self
    }

    /// Append a `Set-Cookie` header.
    pub fn set_cookie(mut self, cookie: &str) -> Self {
        self.meta.push(MetaEntry {
            key: format!("{}{}", META_RESP_COOKIE_PREFIX, self.cookie_count),
            value: cookie.to_string(),
        });
        self.cookie_count += 1;
        self
    }

    /// Serialise `value` to JSON and emit with Content-Type: application/json.
    pub fn json<T: Serialize>(mut self, value: &T) -> OutputStream {
        match serde_json::to_vec(value) {
            Ok(body) => {
                self.meta.push(MetaEntry {
                    key: META_RESP_CONTENT_TYPE.to_string(),
                    value: "application/json".to_string(),
                });
                OutputStream::respond_with_meta(body, self.meta)
            }
            Err(e) => err_internal("response serialization failed", e),
        }
    }

    /// Emit `bytes` with the given content type.
    pub fn body(mut self, bytes: Vec<u8>, content_type: &str) -> OutputStream {
        if !content_type.is_empty() {
            self.meta.push(MetaEntry {
                key: META_RESP_CONTENT_TYPE.to_string(),
                value: content_type.to_string(),
            });
        }
        OutputStream::respond_with_meta(bytes, self.meta)
    }

    /// Emit an empty body (headers / cookies only).
    pub fn empty(self) -> OutputStream {
        OutputStream::respond_with_meta(Vec::new(), self.meta)
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencoding_decode_plus_to_space() {
        assert_eq!(urlencoding_decode("a+b"), "a b");
    }

    #[test]
    fn urlencoding_decode_percent() {
        assert_eq!(urlencoding_decode("a%2Fb"), "a/b");
    }

    #[test]
    fn now_rfc3339_parses() {
        let s = now_rfc3339();
        let _: chrono::DateTime<chrono::Utc> = s.parse().expect("rfc3339 round-trip");
    }

    #[test]
    fn urlencode_space_becomes_plus() {
        assert_eq!(urlencode("a b"), "a+b");
    }

    #[test]
    fn urlencode_special_chars() {
        // Slash and ampersand must be percent-encoded in form values.
        let encoded = urlencode("a/b&c=d");
        assert!(!encoded.contains('/'));
        assert!(!encoded.contains('&'));
        assert!(!encoded.contains('='));
    }

    #[test]
    fn url_path_encode_basic() {
        assert_eq!(url_path_encode("hello"), "hello");
        assert_eq!(url_path_encode("hello world"), "hello%20world");
        assert_eq!(url_path_encode("a+b=c&d"), "a%2Bb%3Dc%26d");
        assert_eq!(url_path_encode("a/b"), "a%2Fb");
        assert_eq!(url_path_encode("café"), "caf%C3%A9");
    }

    #[test]
    fn form_url_encode_uses_plus_for_space() {
        assert_eq!(form_url_encode("hello world"), "hello+world");
        assert_eq!(form_url_encode("a+b=c&d"), "a%2Bb%3Dc%26d");
        assert_eq!(form_url_encode("a/b"), "a%2Fb");
        assert_eq!(form_url_encode("café"), "caf%C3%A9");
        // Round-trip via parse_form_body decodes '+' back to ' '.
        let encoded = form_url_encode("hello world");
        let parsed = parse_form_body(format!("k={encoded}").as_bytes());
        assert_eq!(parsed.get("k"), Some(&"hello world".to_string()));
    }

    #[test]
    fn test_now_rfc3339_format() {
        let ts = now_rfc3339();
        assert!(ts.contains('T'), "RFC 3339 must contain 'T' separator");
        assert!(
            ts.contains('+') || ts.ends_with('Z'),
            "RFC 3339 must have timezone"
        );
    }

    #[test]
    fn test_json_map_from_object() {
        let val = serde_json::json!({"name": "Alice", "age": 30});
        let map = json_map(val);
        assert_eq!(map.get("name").unwrap(), "Alice");
        assert_eq!(map.get("age").unwrap(), 30);
    }

    #[test]
    fn test_json_map_from_non_object() {
        let map = json_map(serde_json::json!("not an object"));
        assert!(map.is_empty());
        let map = json_map(serde_json::json!(42));
        assert!(map.is_empty());
        let map = json_map(serde_json::json!(null));
        assert!(map.is_empty());
    }

    #[test]
    fn test_record_ext_str_field() {
        let mut data = HashMap::new();
        data.insert("name".to_string(), serde_json::json!("Alice"));
        data.insert("count".to_string(), serde_json::json!(42));
        let record = Record {
            id: "1".to_string(),
            data,
        };

        assert_eq!(record.str_field("name"), "Alice");
        assert_eq!(record.str_field("missing"), "");
        assert_eq!(record.str_field("count"), ""); // number is not a string
    }

    #[test]
    fn test_record_ext_i64_field() {
        let mut data = HashMap::new();
        data.insert("count".to_string(), serde_json::json!(42));
        data.insert("name".to_string(), serde_json::json!("Alice"));
        let record = Record {
            id: "1".to_string(),
            data,
        };

        assert_eq!(record.i64_field("count"), 42);
        assert_eq!(record.i64_field("missing"), 0);
        assert_eq!(record.i64_field("name"), 0);
    }

    #[test]
    fn test_record_ext_bool_field() {
        let mut data = HashMap::new();
        data.insert("active".to_string(), serde_json::json!(true));
        data.insert("disabled".to_string(), serde_json::json!(false));
        let record = Record {
            id: "1".to_string(),
            data,
        };

        assert!(record.bool_field("active"));
        assert!(!record.bool_field("disabled"));
        assert!(!record.bool_field("missing"));
    }

    #[test]
    fn test_field_as_string_variants() {
        let mut data = HashMap::new();
        data.insert("str".to_string(), serde_json::json!("hello"));
        data.insert("num".to_string(), serde_json::json!(42));
        data.insert("bool".to_string(), serde_json::json!(true));
        let record = Record {
            id: "1".to_string(),
            data,
        };

        assert_eq!(field_as_string(&record, "str"), "hello");
        assert_eq!(field_as_string(&record, "num"), "42");
        assert_eq!(field_as_string(&record, "bool"), "");
        assert_eq!(field_as_string(&record, "missing"), "");
    }

    #[test]
    fn test_stamp_created() {
        let mut data = HashMap::new();
        stamp_created(&mut data);
        assert!(data.contains_key("created_at"));
        assert!(data.contains_key("updated_at"));

        // Should not overwrite existing values
        let mut data2 = HashMap::new();
        data2.insert("created_at".to_string(), serde_json::json!("custom"));
        stamp_created(&mut data2);
        assert_eq!(data2.get("created_at").unwrap(), "custom");
    }

    #[test]
    fn test_stamp_updated() {
        let mut data = HashMap::new();
        data.insert("updated_at".to_string(), serde_json::json!("old"));
        stamp_updated(&mut data);
        assert_ne!(data.get("updated_at").unwrap(), "old");
    }

    #[test]
    fn test_hex_encode() {
        assert_eq!(hex_encode(&[0x00, 0xff, 0x0a, 0xbc]), "00ff0abc");
        assert_eq!(hex_encode(&[]), "");
    }

    #[test]
    fn test_sha256_hex_deterministic() {
        let hash1 = sha256_hex(b"hello");
        let hash2 = sha256_hex(b"hello");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // 32 bytes = 64 hex chars

        let hash3 = sha256_hex(b"world");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_sha256_known_value() {
        // SHA-256 of empty string
        let hash = sha256_hex(b"");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    /// Helper: drain an OutputStream and extract the client-facing error
    /// message. Panics if the stream did not terminate with an error.
    fn error_message(stream: OutputStream) -> String {
        use wafer_run::TerminalNotResponse;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt");
        match rt.block_on(stream.collect_buffered()) {
            Ok(_) => panic!("expected error terminal, got Complete"),
            Err(TerminalNotResponse::Error(e)) => e.message,
            Err(other) => panic!("expected error terminal, got {other:?}"),
        }
    }

    #[test]
    fn err_internal_sanitizes_underlying_error() {
        // A vivid error message with content that MUST NOT reach the client:
        // SQL fragments, table names, secrets — anything an attacker could
        // use to fingerprint the backend.
        let raw = "DB error: no such table: secret_users_table (sqlite code 1)";
        let stream = err_internal("Database error", raw);
        let msg = error_message(stream);

        // Client message is the sanitized form.
        assert!(
            msg.starts_with("Internal server error (ref: "),
            "expected sanitized prefix, got {msg:?}"
        );
        assert!(msg.ends_with(')'), "expected closing paren, got {msg:?}");

        // Raw error text does NOT leak.
        assert!(
            !msg.contains("DB error"),
            "raw error label leaked into client message: {msg:?}"
        );
        assert!(
            !msg.contains("secret_users_table"),
            "table name leaked into client message: {msg:?}"
        );
        assert!(
            !msg.contains("sqlite"),
            "backend name leaked into client message: {msg:?}"
        );
        assert!(
            !msg.contains("Database error"),
            "context label leaked into client message: {msg:?}"
        );
    }

    #[test]
    fn err_internal_message_contains_ref_id() {
        let stream = err_internal("Database error", "boom");
        let msg = error_message(stream);

        // Pull out the ref id and check it's a hex string of the expected
        // length (8 bytes -> 16 hex chars).
        let id = msg
            .strip_prefix("Internal server error (ref: ")
            .and_then(|s| s.strip_suffix(')'))
            .expect("message shape");
        assert_eq!(
            id.len(),
            16,
            "expected 16-char hex correlation id, got {id:?}"
        );
        assert!(
            id.chars().all(|c| c.is_ascii_hexdigit()),
            "ref id is not hex: {id:?}"
        );
    }

    #[test]
    fn err_internal_ids_are_unique_across_calls() {
        let a = error_message(err_internal("ctx", "e1"));
        let b = error_message(err_internal("ctx", "e2"));
        assert_ne!(
            a, b,
            "correlation IDs should differ between independent calls"
        );
    }

    #[test]
    fn err_internal_no_cause_also_sanitizes() {
        let stream = err_internal_no_cause("Thread setting vanished between read and update");
        let msg = error_message(stream);
        assert!(
            msg.starts_with("Internal server error (ref: "),
            "expected sanitized prefix, got {msg:?}"
        );
        // Even the context label is not echoed.
        assert!(
            !msg.contains("Thread setting"),
            "context label leaked into client message: {msg:?}"
        );
    }
}
