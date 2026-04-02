use sha2::{Digest, Sha256};
use std::collections::HashMap;
use wafer_core::clients::database::Record;

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
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
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
