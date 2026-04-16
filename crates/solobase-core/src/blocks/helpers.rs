use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use wafer_core::clients::database::Record;
use wafer_run::meta::{
    META_RESP_CONTENT_TYPE, META_RESP_COOKIE_PREFIX, META_RESP_HEADER_PREFIX, META_RESP_STATUS,
};
use wafer_run::types::{ErrorCode, MetaEntry, WaferError};
use wafer_run::OutputStream;

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
        write!(s, "{:02x}", b).unwrap();
    }
    s
}

/// Check if the current user has admin role from the message metadata.
pub fn is_admin(msg: &wafer_run::types::Message) -> bool {
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
        Err(e) => OutputStream::error(WaferError {
            code: ErrorCode::Internal,
            message: format!("serialize failed: {}", e),
            meta: vec![],
        }),
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
pub fn err_internal(message: &str) -> OutputStream {
    OutputStream::error(WaferError {
        code: ErrorCode::Internal,
        message: message.to_string(),
        meta: vec![],
    })
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
            Err(e) => err_internal(&format!("serialize failed: {}", e)),
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
}
