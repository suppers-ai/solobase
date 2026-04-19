//! DB schema for the `suppers_ai__llm__providers` collection.
//!
//! Owns the table declaration plus row ↔ [`ProviderConfig`] conversion
//! helpers.
//!
//! Secrets are NOT stored in this table. Providers reference an entry in
//! `suppers_ai__admin__variables` via [`ProviderConfig::key_var`]; the
//! feature block resolves `key_var` → plaintext `api_key` at runtime
//! before calling `ProviderLlmService::configure`. This matches the
//! project-wide convention for sensitive config (single storage location,
//! admin-gated, masked in API responses) and avoids a `*_encrypted` column
//! name that would not actually be encrypted.

use std::collections::HashMap;

use wafer_core::clients::database::Record;
use wafer_run::types::CollectionSchema;

use super::providers::config::{ProviderConfig, ProviderProtocol};

pub const PROVIDERS_COLLECTION: &str = "suppers_ai__llm__providers";

/// Table declaration for `suppers_ai__llm__providers`.
pub fn providers_schema() -> CollectionSchema {
    CollectionSchema::new(PROVIDERS_COLLECTION)
        .field_unique("name", "string")
        .field("protocol", "string")
        .field("endpoint", "string")
        .field_optional("key_var", "string")
        .field_default("models", "json", "[]")
        .field_default("enabled", "int", "1")
        .index(&["enabled"])
}

/// Encode a [`ProviderConfig`] as a row payload for insert/update.
///
/// `cfg.api_key` is intentionally dropped — it is a runtime-resolved value
/// and must not be persisted. The secret lives in
/// `suppers_ai__admin__variables` and is referenced by `cfg.key_var`.
pub fn config_to_row(cfg: &ProviderConfig) -> HashMap<String, serde_json::Value> {
    let mut row = HashMap::new();
    row.insert(
        "name".to_string(),
        serde_json::Value::String(cfg.name.clone()),
    );
    row.insert(
        "protocol".to_string(),
        serde_json::Value::String(cfg.protocol.as_str().to_string()),
    );
    row.insert(
        "endpoint".to_string(),
        serde_json::Value::String(cfg.endpoint.clone()),
    );
    if let Some(var) = &cfg.key_var {
        row.insert(
            "key_var".to_string(),
            serde_json::Value::String(var.clone()),
        );
    }
    row.insert(
        "models".to_string(),
        serde_json::Value::Array(
            cfg.models
                .iter()
                .map(|m| serde_json::Value::String(m.clone()))
                .collect(),
        ),
    );
    row.insert(
        "enabled".to_string(),
        serde_json::Value::Number(serde_json::Number::from(if cfg.enabled { 1 } else { 0 })),
    );
    row
}

/// Decode a database [`Record`] into a [`ProviderConfig`].
///
/// The returned `api_key` is always `None` — callers resolve it from
/// `key_var` at call time via the config client.
pub fn row_to_config(record: &Record) -> Result<ProviderConfig, String> {
    let name = record
        .data
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing `name`".to_string())?
        .to_string();

    let protocol_str = record
        .data
        .get("protocol")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing `protocol`".to_string())?;
    let protocol = ProviderProtocol::parse(protocol_str)
        .ok_or_else(|| format!("invalid `protocol`: {protocol_str}"))?;

    let endpoint = record
        .data
        .get("endpoint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing `endpoint`".to_string())?
        .to_string();

    let key_var = record
        .data
        .get("key_var")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let models = match record.data.get("models") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        Some(serde_json::Value::String(s)) if !s.is_empty() => {
            serde_json::from_str::<Vec<String>>(s)
                .map_err(|e| format!("invalid `models` json: {e}"))?
        }
        _ => Vec::new(),
    };

    let enabled = match record.data.get("enabled") {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        Some(serde_json::Value::String(s)) => matches!(s.as_str(), "1" | "true"),
        None => true,
        _ => true,
    };

    Ok(ProviderConfig {
        name,
        protocol,
        endpoint,
        api_key: None,
        key_var,
        models,
        enabled,
    })
}

#[cfg(test)]
mod tests {
    use wafer_core::clients::database::Record;

    use super::*;

    #[test]
    fn schema_declares_collection_name() {
        let s = providers_schema();
        assert_eq!(s.name, PROVIDERS_COLLECTION);
    }

    #[test]
    fn schema_declares_all_expected_fields() {
        let s = providers_schema();
        let by_name: HashMap<&str, &wafer_run::types::FieldSchema> =
            s.fields.iter().map(|f| (f.name.as_str(), f)).collect();

        let name = by_name.get("name").expect("name field");
        assert_eq!(name.field_type, "string");
        assert!(name.unique, "`name` must be unique");

        let protocol = by_name.get("protocol").expect("protocol field");
        assert_eq!(protocol.field_type, "string");

        let endpoint = by_name.get("endpoint").expect("endpoint field");
        assert_eq!(endpoint.field_type, "string");

        assert!(
            !by_name.contains_key("api_key_encrypted"),
            "api_key_encrypted must not exist — secrets live in suppers_ai__admin__variables"
        );
        assert!(
            !by_name.contains_key("api_key"),
            "api_key must not exist — providers reference secrets via key_var only"
        );

        let key_var = by_name.get("key_var").expect("key_var field");
        assert_eq!(key_var.field_type, "string");
        assert!(key_var.optional, "`key_var` must be optional");

        let models = by_name.get("models").expect("models field");
        assert_eq!(models.field_type, "json");
        assert_eq!(models.default_value, "[]");

        let enabled = by_name.get("enabled").expect("enabled field");
        assert_eq!(enabled.field_type, "int");
        assert_eq!(enabled.default_value, "1");
    }

    #[test]
    fn schema_has_enabled_index() {
        let s = providers_schema();
        assert!(
            s.indexes.iter().any(|ix| ix.fields == vec!["enabled"]),
            "expected an index on [enabled]; got {:?}",
            s.indexes
        );
    }

    #[test]
    fn config_to_row_encodes_minimal_config() {
        let cfg = ProviderConfig::new(
            "openai-main",
            ProviderProtocol::OpenAi,
            "https://api.openai.com/v1",
        );
        let row = config_to_row(&cfg);
        assert_eq!(
            row.get("name").and_then(|v| v.as_str()),
            Some("openai-main")
        );
        assert_eq!(
            row.get("protocol").and_then(|v| v.as_str()),
            Some("open_ai")
        );
        assert_eq!(
            row.get("endpoint").and_then(|v| v.as_str()),
            Some("https://api.openai.com/v1")
        );
        assert!(!row.contains_key("api_key_encrypted"));
        assert!(!row.contains_key("key_var"));
        assert_eq!(row.get("models"), Some(&serde_json::json!([])));
        assert_eq!(row.get("enabled").and_then(|v| v.as_i64()), Some(1));
    }

    #[test]
    fn config_to_row_never_persists_api_key() {
        // Even if a runtime ProviderConfig holds a resolved plaintext
        // api_key, it must not be written to the providers table.
        let cfg = ProviderConfig::new(
            "local-llama",
            ProviderProtocol::OpenAiCompatible,
            "http://localhost:11434/v1",
        )
        .with_api_key("resolved-plaintext-secret")
        .with_key_var("SUPPERS_AI__LLM__OPENAI_KEY")
        .with_models(vec!["llama3".into(), "mistral".into()]);
        let row = config_to_row(&cfg);
        assert!(
            !row.contains_key("api_key"),
            "api_key must never be persisted"
        );
        assert!(
            !row.contains_key("api_key_encrypted"),
            "api_key_encrypted must not exist"
        );
        assert_eq!(
            row.get("key_var").and_then(|v| v.as_str()),
            Some("SUPPERS_AI__LLM__OPENAI_KEY")
        );
        assert_eq!(
            row.get("models"),
            Some(&serde_json::json!(["llama3", "mistral"]))
        );
    }

    #[test]
    fn roundtrip_drops_api_key_on_load() {
        // The runtime api_key (if set) is lost through the DB roundtrip —
        // callers must re-resolve via key_var after loading.
        let cfg = ProviderConfig::new(
            "anthropic-main",
            ProviderProtocol::Anthropic,
            "https://api.anthropic.com/v1",
        )
        .with_api_key("runtime-only")
        .with_key_var("SUPPERS_AI__LLM__ANTHROPIC_KEY")
        .with_models(vec!["claude-opus-4-7".into()]);
        let row = config_to_row(&cfg);
        let record = Record {
            id: "abc123".into(),
            data: row,
        };
        let decoded = row_to_config(&record).expect("decode");
        assert_eq!(decoded.name, cfg.name);
        assert_eq!(decoded.protocol, cfg.protocol);
        assert_eq!(decoded.endpoint, cfg.endpoint);
        assert_eq!(decoded.key_var, cfg.key_var);
        assert_eq!(decoded.models, cfg.models);
        assert_eq!(decoded.enabled, cfg.enabled);
        assert!(
            decoded.api_key.is_none(),
            "api_key must be None after roundtrip — resolve via key_var at call time"
        );
    }

    #[test]
    fn row_to_config_rejects_invalid_protocol() {
        let record = Record {
            id: "r1".into(),
            data: {
                let mut d = HashMap::new();
                d.insert("name".into(), serde_json::json!("x"));
                d.insert("protocol".into(), serde_json::json!("openai")); // non-canonical
                d.insert("endpoint".into(), serde_json::json!("https://x"));
                d
            },
        };
        let err = row_to_config(&record).expect_err("must reject alias");
        assert!(err.contains("invalid `protocol`"), "got: {err}");
    }

    #[test]
    fn row_to_config_handles_missing_optionals() {
        let record = Record {
            id: "r1".into(),
            data: {
                let mut d = HashMap::new();
                d.insert("name".into(), serde_json::json!("x"));
                d.insert("protocol".into(), serde_json::json!("open_ai"));
                d.insert("endpoint".into(), serde_json::json!("https://x"));
                // no api_key_encrypted, no key_var, no models, no enabled
                d
            },
        };
        let cfg = row_to_config(&record).expect("decode");
        assert_eq!(cfg.name, "x");
        assert!(cfg.api_key.is_none());
        assert!(cfg.key_var.is_none());
        assert!(cfg.models.is_empty());
        assert!(cfg.enabled, "enabled should default to true when missing");
    }

    #[test]
    fn row_to_config_accepts_models_as_json_string() {
        // Defensive: some DB backends may serialize json columns as strings.
        let record = Record {
            id: "r1".into(),
            data: {
                let mut d = HashMap::new();
                d.insert("name".into(), serde_json::json!("x"));
                d.insert("protocol".into(), serde_json::json!("open_ai"));
                d.insert("endpoint".into(), serde_json::json!("https://x"));
                d.insert("models".into(), serde_json::json!(r#"["a","b"]"#));
                d
            },
        };
        let cfg = row_to_config(&record).expect("decode");
        assert_eq!(cfg.models, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn row_to_config_enabled_from_int_zero_is_false() {
        let record = Record {
            id: "r1".into(),
            data: {
                let mut d = HashMap::new();
                d.insert("name".into(), serde_json::json!("x"));
                d.insert("protocol".into(), serde_json::json!("open_ai"));
                d.insert("endpoint".into(), serde_json::json!("https://x"));
                d.insert("enabled".into(), serde_json::json!(0));
                d
            },
        };
        let cfg = row_to_config(&record).expect("decode");
        assert!(!cfg.enabled);
    }
}
