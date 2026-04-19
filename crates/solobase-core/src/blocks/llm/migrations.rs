//! One-shot migration from the legacy `suppers_ai__provider_llm__providers`
//! table to the new `suppers_ai__llm__providers` table.
//!
//! Runs on `LlmBlock::lifecycle(Init)`. Idempotent — when the legacy table
//! is gone (normal post-migration state), step 1 short-circuits cleanly and
//! the helper returns `Ok(())`.
//!
//! Mapping (see docstrings on [`map_legacy_row`] for the full table):
//!   - `provider_type` → `protocol` (openai → open_ai, anthropic → anthropic,
//!     anything else → open_ai_compatible)
//!   - `models` (JSON string) → `models` (Vec<String>; `[]` on parse error)
//!   - `enabled` (int/bool/string) coerced to the `ProviderConfig::enabled` bool
//!   - `key_var` derived from the legacy provider_type:
//!       * openai    → `Some("SUPPERS_AI__PROVIDER_LLM__OPENAI_KEY")`
//!       * anthropic → `Some("SUPPERS_AI__PROVIDER_LLM__ANTHROPIC_KEY")`
//!       * otherwise → `None` (local OpenAI-compatible servers typically
//!         don't need auth)
//!
//! Secrets (`SUPPERS_AI__PROVIDER_LLM__*_KEY`) remain in
//! `suppers_ai__admin__variables`. Task 25 deletes the legacy `ConfigVar`
//! declarations; the stored values stay so admins can rename them at leisure.

use std::collections::HashMap;

use wafer_core::clients::database::{self as db, ListOptions, Record};
use wafer_run::{context::Context, types::ErrorCode};

use super::{
    providers::{
        config::{ProviderConfig, ProviderProtocol},
        ProviderLlmService,
    },
    schema::{config_to_row, PROVIDERS_COLLECTION},
};

/// The legacy providers collection. Owned by the provider-llm block (still
/// present on-disk on upgrade paths) — we read once, migrate, and drop.
pub(super) const LEGACY_COLLECTION: &str = "suppers_ai__provider_llm__providers";

const LEGACY_OPENAI_KEY_VAR: &str = "SUPPERS_AI__PROVIDER_LLM__OPENAI_KEY";
const LEGACY_ANTHROPIC_KEY_VAR: &str = "SUPPERS_AI__PROVIDER_LLM__ANTHROPIC_KEY";

/// Map a legacy provider row's `data` HashMap into a new-schema
/// [`ProviderConfig`]. Pure function — no DB access — so it's testable
/// without any runtime scaffolding.
pub(super) fn map_legacy_row(data: &HashMap<String, serde_json::Value>) -> Option<ProviderConfig> {
    let name = data.get("name").and_then(|v| v.as_str())?.to_string();
    if name.is_empty() {
        return None;
    }

    // Legacy `provider_type` column — "openai" / "anthropic" /
    // "openai-compatible" / arbitrary.
    let provider_type = data
        .get("provider_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let (protocol, key_var) = match provider_type {
        "openai" => (
            ProviderProtocol::OpenAi,
            Some(LEGACY_OPENAI_KEY_VAR.to_string()),
        ),
        "anthropic" => (
            ProviderProtocol::Anthropic,
            Some(LEGACY_ANTHROPIC_KEY_VAR.to_string()),
        ),
        // Includes "openai-compatible" (the legacy hyphenated token) and any
        // other value — local servers are typically unauthenticated, so no
        // `key_var` is derived.
        _ => (ProviderProtocol::OpenAiCompatible, None),
    };

    let endpoint = data
        .get("endpoint")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("https://api.openai.com/v1")
        .to_string();

    let models = match data.get("models") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        // Legacy stored `models` as a JSON string in a `text` column.
        Some(serde_json::Value::String(s)) if !s.is_empty() => {
            serde_json::from_str::<Vec<String>>(s).unwrap_or_default()
        }
        _ => Vec::new(),
    };

    let enabled = match data.get("enabled") {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        Some(serde_json::Value::String(s)) => matches!(s.as_str(), "1" | "true"),
        None => true,
        _ => true,
    };

    Some(ProviderConfig {
        name,
        protocol,
        endpoint,
        api_key: None,
        key_var,
        models,
        enabled,
    })
}

/// Run the one-shot migration. Safe to call on every boot:
///   - legacy table absent → `Ok(())`, nothing to do
///   - legacy table present → copy each row into the new table, then drop it
///
/// Partial-failure safe: a single row that fails to insert is logged (not
/// fatal). The legacy table is only dropped after every row was attempted.
/// After a successful migration, the `ProviderLlmService` is refreshed from
/// the new table so chat works immediately without a restart.
pub(super) async fn migrate_legacy_providers(
    ctx: &dyn Context,
    provider_svc: &ProviderLlmService,
) -> Result<(), String> {
    // Step 1: read the legacy table. NotFound (table doesn't exist) = already
    // migrated — return cleanly. All other errors bubble up to the caller.
    let legacy = match db::list(ctx, LEGACY_COLLECTION, &ListOptions::default()).await {
        Ok(r) => r,
        Err(e) if e.code == ErrorCode::NotFound => {
            // Already migrated or never needed — common path on every subsequent
            // boot. No log: this is the expected steady state.
            return Ok(());
        }
        Err(e) => {
            tracing::warn!("legacy providers table read failed (skipping migration): {e}");
            return Ok(());
        }
    };

    if legacy.records.is_empty() {
        // Empty legacy table — nothing to copy, but we still drop it so the
        // next boot takes the short-circuit path above.
        drop_legacy_table(ctx).await;
        return Ok(());
    }

    tracing::info!(
        "migrating {} legacy provider row(s) from {LEGACY_COLLECTION} → {PROVIDERS_COLLECTION}",
        legacy.records.len()
    );

    let mut migrated = 0usize;
    let mut skipped = 0usize;
    for record in &legacy.records {
        match migrate_one(ctx, record).await {
            MigrateOutcome::Inserted => migrated += 1,
            MigrateOutcome::Skipped => skipped += 1,
            MigrateOutcome::Failed => {
                // Already logged inside migrate_one; just count implicitly as
                // "skipped" for the summary.
                skipped += 1;
            }
        }
    }

    tracing::info!("legacy provider migration done: inserted={migrated} skipped={skipped}");

    // Step 3: drop the legacy table so subsequent boots skip migration.
    drop_legacy_table(ctx).await;

    // Step 4: refresh the in-memory ProviderLlmService from the new table so
    // chat routes pick up the migrated providers without a restart.
    reload_service(ctx, provider_svc).await;

    Ok(())
}

enum MigrateOutcome {
    Inserted,
    Skipped,
    Failed,
}

async fn migrate_one(ctx: &dyn Context, record: &Record) -> MigrateOutcome {
    let Some(cfg) = map_legacy_row(&record.data) else {
        tracing::warn!(
            "skipping unmappable legacy provider row id={}: missing/empty name",
            record.id
        );
        return MigrateOutcome::Skipped;
    };

    let row = config_to_row(&cfg);
    match db::create(ctx, PROVIDERS_COLLECTION, row).await {
        Ok(_) => MigrateOutcome::Inserted,
        Err(e) if e.code == ErrorCode::AlreadyExists => {
            // Admin has already recreated this provider in the new table
            // (the `name` column is unique). Leave it alone.
            tracing::info!(
                "skipping legacy provider '{}': already exists in new table",
                cfg.name
            );
            MigrateOutcome::Skipped
        }
        Err(e) => {
            tracing::warn!(
                "failed to migrate legacy provider '{}' (id={}): {e}",
                cfg.name,
                record.id
            );
            MigrateOutcome::Failed
        }
    }
}

/// Drop the legacy table. No `drop_collection` exists on the database client,
/// so we issue a raw `DROP TABLE IF EXISTS` — destructive but intended: this
/// is a one-shot cleanup. Errors are warnings, not fatal: the next boot will
/// just try again and still find nothing to migrate (empty / NotFound).
async fn drop_legacy_table(ctx: &dyn Context) {
    let stmt = format!("DROP TABLE IF EXISTS {LEGACY_COLLECTION}");
    if let Err(e) = db::exec_raw(ctx, &stmt, &[]).await {
        tracing::warn!("failed to drop legacy table {LEGACY_COLLECTION}: {e}");
    }
}

/// Reload enabled providers from the new table and push into the service.
/// Mirrors `routes::reload_provider_service` but is intentionally duplicated
/// here to keep the migration self-contained (no cross-module coupling).
async fn reload_service(ctx: &dyn Context, provider_svc: &ProviderLlmService) {
    let opts = ListOptions {
        limit: 200,
        ..Default::default()
    };
    let records = match db::list(ctx, PROVIDERS_COLLECTION, &opts).await {
        Ok(r) => r.records,
        Err(e) => {
            tracing::warn!("post-migration reload list failed: {e}");
            return;
        }
    };
    let mut configs = Vec::with_capacity(records.len());
    for rec in &records {
        match super::schema::row_to_config(rec) {
            Ok(cfg) if cfg.enabled => configs.push(cfg),
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("post-migration: skipping malformed row {}: {e}", rec.id);
            }
        }
    }
    provider_svc.configure(configs);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_from_json(v: serde_json::Value) -> HashMap<String, serde_json::Value> {
        v.as_object()
            .expect("test row must be a JSON object")
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    #[test]
    fn maps_legacy_openai_row() {
        let row = row_from_json(serde_json::json!({
            "name": "OpenAI",
            "provider_type": "openai",
            "endpoint": "https://api.openai.com/v1",
            "models": r#"["gpt-4o","gpt-4o-mini"]"#,
            "enabled": 1,
        }));
        let cfg = map_legacy_row(&row).expect("maps");
        assert_eq!(cfg.name, "OpenAI");
        assert_eq!(cfg.protocol, ProviderProtocol::OpenAi);
        assert_eq!(cfg.endpoint, "https://api.openai.com/v1");
        assert_eq!(
            cfg.key_var.as_deref(),
            Some("SUPPERS_AI__PROVIDER_LLM__OPENAI_KEY")
        );
        assert_eq!(cfg.models, vec!["gpt-4o".to_string(), "gpt-4o-mini".into()]);
        assert!(cfg.enabled);
        assert!(cfg.api_key.is_none(), "api_key must never be populated");
    }

    #[test]
    fn maps_legacy_anthropic_row() {
        let row = row_from_json(serde_json::json!({
            "name": "Anthropic",
            "provider_type": "anthropic",
            "endpoint": "https://api.anthropic.com/v1",
            "models": r#"["claude-sonnet-4-20250514"]"#,
            "enabled": 0,
        }));
        let cfg = map_legacy_row(&row).expect("maps");
        assert_eq!(cfg.protocol, ProviderProtocol::Anthropic);
        assert_eq!(
            cfg.key_var.as_deref(),
            Some("SUPPERS_AI__PROVIDER_LLM__ANTHROPIC_KEY")
        );
        assert_eq!(cfg.models, vec!["claude-sonnet-4-20250514".to_string()]);
        assert!(!cfg.enabled, "enabled=0 must decode to false");
    }

    #[test]
    fn maps_unknown_provider_type_to_open_ai_compatible_no_key_var() {
        // Includes the legacy hyphenated "openai-compatible" token and any
        // other free-form value (e.g. a local server that someone typed
        // "ollama" into the dropdown).
        for provider_type in ["openai-compatible", "ollama", "whatever", ""] {
            let row = row_from_json(serde_json::json!({
                "name": "Local",
                "provider_type": provider_type,
                "endpoint": "http://localhost:11434/v1",
                "models": "[]",
                "enabled": 1,
            }));
            let cfg = map_legacy_row(&row)
                .unwrap_or_else(|| panic!("expected to map provider_type={provider_type:?}"));
            assert_eq!(
                cfg.protocol,
                ProviderProtocol::OpenAiCompatible,
                "provider_type={provider_type:?} should map to open_ai_compatible"
            );
            assert!(
                cfg.key_var.is_none(),
                "unknown/local provider_type must not derive a key_var"
            );
            assert!(cfg.models.is_empty());
        }
    }

    #[test]
    fn falls_back_to_default_endpoint_when_missing() {
        let row = row_from_json(serde_json::json!({
            "name": "X",
            "provider_type": "openai",
        }));
        let cfg = map_legacy_row(&row).expect("maps");
        assert_eq!(cfg.endpoint, "https://api.openai.com/v1");
    }

    #[test]
    fn treats_malformed_models_json_as_empty() {
        let row = row_from_json(serde_json::json!({
            "name": "X",
            "provider_type": "openai",
            "endpoint": "https://api.openai.com/v1",
            "models": "this is not json",
        }));
        let cfg = map_legacy_row(&row).expect("maps");
        assert!(cfg.models.is_empty(), "bad JSON should fall back to []");
    }

    #[test]
    fn rejects_row_without_name() {
        let row = row_from_json(serde_json::json!({
            "provider_type": "openai",
            "endpoint": "https://x",
        }));
        assert!(map_legacy_row(&row).is_none());
    }

    #[test]
    fn rejects_row_with_empty_name() {
        let row = row_from_json(serde_json::json!({
            "name": "",
            "provider_type": "openai",
        }));
        assert!(map_legacy_row(&row).is_none());
    }
}
