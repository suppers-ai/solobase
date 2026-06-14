//! Target-agnostic boot orchestration + variable seeding.
//!
//! Before this module existed, each platform (native CLI, Cloudflare Worker,
//! browser WASM) carried its own copy of:
//! - the auto-generated-secret seeder,
//! - the env-var → variables-table seeder,
//! - and the seal → admin-init → seed → init_all_blocks → post_start
//!   lifecycle dance,
//!
//! with documented drift between the copies (the audit's Top-10 #9). The
//! seeding now lives here once, written against [`DatabaseService`] so all
//! three targets share it — native constructs its `SQLiteDatabaseService`
//! pre-wafer, the browser hands in its `BrowserDatabaseService`, and the
//! Cloudflare runner its D1 service.
//!
//! The block-settings hash-gated seed lives next to its pure planner in
//! [`crate::features::load_and_seed_block_settings`].

use std::{collections::HashMap, sync::Arc};

use wafer_block::db::ListOptions;
use wafer_core::interfaces::database::service::DatabaseService;

use crate::blocks::admin::VARIABLES_TABLE;

/// Seed `INSERT OR IGNORE`-style: every variable row carries a synthesized
/// `id`, the canonical `block` column ([`crate::config_vars::screaming_block`]),
/// and `created_at` / `updated_at`. The `value`/`name`/`description`/`warning`/
/// `sensitive` columns vary by call site.
fn build_variable_row(
    key: &str,
    value: &str,
    name: &str,
    description: &str,
    warning: &str,
    sensitive: bool,
    block: &str,
) -> HashMap<String, serde_json::Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let id = format!("var_{}", uuid::Uuid::new_v4());
    let mut data: HashMap<String, serde_json::Value> = HashMap::new();
    data.insert("id".into(), serde_json::Value::String(id));
    data.insert("key".into(), serde_json::Value::String(key.to_string()));
    data.insert("value".into(), serde_json::Value::String(value.to_string()));
    data.insert("name".into(), serde_json::Value::String(name.to_string()));
    data.insert(
        "description".into(),
        serde_json::Value::String(description.to_string()),
    );
    data.insert(
        "warning".into(),
        serde_json::Value::String(warning.to_string()),
    );
    data.insert(
        "sensitive".into(),
        serde_json::Value::Number(serde_json::Number::from(i64::from(sensitive))),
    );
    if !block.is_empty() {
        data.insert("block".into(), serde_json::Value::String(block.to_string()));
    }
    data.insert("created_at".into(), serde_json::Value::String(now.clone()));
    data.insert("updated_at".into(), serde_json::Value::String(now));
    data
}

/// `INSERT OR IGNORE` semantics over [`DatabaseService`]: insert a variable
/// row for `key` only when no row with that `key` already exists.
///
/// Public so platform code can seed its own non-declared defaults (e.g. the
/// browser's bootstrap-admin credentials and WebLLM script var) through the
/// same `DatabaseService` path — no bridge raw SQL, no hardcoded table
/// literal. The `block` column is derived from the key via
/// [`crate::config_vars::key_block_prefix`], matching migration 002.
///
/// Returns `Ok(true)` when a row was inserted, `Ok(false)` when one already
/// existed. A pre-existing row always wins — seeding never clobbers a stored
/// value.
pub async fn seed_variable_if_absent(
    db: &Arc<dyn DatabaseService>,
    key: &str,
    value: &str,
    name: &str,
    description: &str,
    sensitive: bool,
) -> Result<bool, String> {
    let block = crate::config_vars::key_block_prefix(key);
    let data = build_variable_row(key, value, name, description, "", sensitive, &block);
    insert_if_absent(db, key, data).await
}

/// `INSERT OR IGNORE` semantics over [`DatabaseService`]: insert `data` into
/// `VARIABLES_TABLE` only when no row with `data["key"]` already exists.
///
/// `db.create` has no native `OR IGNORE`, so we check existence first via a
/// `key`-filtered list. A pre-existing row (env override, prior boot, admin-UI
/// edit) always wins — seeding never clobbers a stored value.
///
/// Returns `Ok(true)` when a row was inserted, `Ok(false)` when one already
/// existed. Errors bubble up so the caller can decide whether a failed seed is
/// fatal (a missing JWT secret) or merely logged (best-effort secrets).
async fn insert_if_absent(
    db: &Arc<dyn DatabaseService>,
    key: &str,
    data: HashMap<String, serde_json::Value>,
) -> Result<bool, String> {
    let exists_opts = ListOptions {
        filters: vec![wafer_block::db::Filter {
            field: "key".to_string(),
            operator: wafer_block::db::FilterOp::Equal,
            value: serde_json::Value::String(key.to_string()),
        }],
        limit: 1,
        offset: 0,
        skip_count: true,
        ..Default::default()
    };
    // `list` tolerates a missing table (returns empty), so on a fresh DB this
    // is a clean "does not exist" rather than an error.
    let listed = db
        .list(VARIABLES_TABLE, &exists_opts)
        .await
        .map_err(|e| format!("list {VARIABLES_TABLE} for key `{key}`: {e}"))?;
    if !listed.records.is_empty() {
        return Ok(false);
    }
    db.create(VARIABLES_TABLE, data)
        .await
        .map_err(|e| format!("insert variable `{key}`: {e}"))?;
    Ok(true)
}

/// Auto-generate random 32-byte hex secrets for every [`wafer_block::ConfigVar`]
/// declared with `.auto_generate()` that lacks a row in the admin variables
/// table. Shared by all three targets.
///
/// Idempotent: a key that already has a row is left untouched. Per-key failures
/// are logged and tolerated — operators retain the manual seed fallback.
///
/// Ordering contract: this MUST run after the admin block's `lifecycle(Init)`
/// (so migration 002's `block` column exists) and BEFORE
/// [`wafer_run::Wafer::init_all_blocks`] on the targets that seed post-admin
/// (Cloudflare, browser). Native seeds pre-wafer, so it ensures the table
/// itself first (see [`crate::blocks::admin::migrations::apply_via_service`]).
pub async fn seed_auto_generated(db: &Arc<dyn DatabaseService>) {
    let block_infos = crate::blocks::all_block_infos();
    for info in &block_infos {
        let block_col = crate::config_vars::screaming_block(&info.name);
        for var in &info.config_keys {
            if !var.auto_generate {
                continue;
            }
            match seed_one_secret(db, &block_col, var).await {
                Ok(true) => tracing::warn!(
                    key = %var.key,
                    block = %info.name,
                    "auto-generated secret seeded (no row existed)"
                ),
                Ok(false) => {}
                Err(e) => tracing::warn!(
                    key = %var.key,
                    block = %info.name,
                    error = %e,
                    "seed_auto_generated failed"
                ),
            }
        }
    }
}

/// Generate one 32-byte hex secret and insert it for `var` when absent.
async fn seed_one_secret(
    db: &Arc<dyn DatabaseService>,
    block_col: &str,
    var: &wafer_block::ConfigVar,
) -> Result<bool, String> {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).map_err(|e| format!("getrandom: {e}"))?;
    let secret: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let data = build_variable_row(
        &var.key,
        &secret,
        &var.name,
        &var.description,
        &var.warning,
        true,
        block_col,
    );
    insert_if_absent(db, &var.key, data).await
}

/// Seed `env_vars` into the admin variables table (`INSERT OR IGNORE`),
/// auto-generate any `auto_generate` secrets, and return the full key→value
/// map currently stored.
///
/// `env_vars` is empty for the browser and Cloudflare targets (their config
/// lives in the platform store, not process env) and carries the
/// declared-key-filtered process environment on native.
///
/// PRECONDITION: the `VARIABLES_TABLE` must already exist — either because the
/// admin block's `lifecycle(Init)` has run (browser, Cloudflare), or because
/// the caller ensured it pre-wafer (native). `db.create` does not lazily
/// create tables.
pub async fn seed_and_load_variables(
    db: &Arc<dyn DatabaseService>,
    env_vars: &[(String, String)],
) -> Result<HashMap<String, String>, String> {
    // 1. Seed env-provided values (existing rows win).
    for (key, value) in env_vars {
        let sensitive = key.ends_with("_SECRET") || key.ends_with("_KEY");
        let block = crate::config_vars::key_block_prefix(key);
        let data = build_variable_row(key, value, "", "", "", sensitive, &block);
        if let Err(e) = insert_if_absent(db, key, data).await {
            tracing::warn!(key = %key, error = %e, "failed to seed env variable");
        }
    }

    // 2. Auto-generate declared secrets (incl. the auth JWT secret).
    seed_auto_generated(db).await;
    seed_jwt_secret(db).await;

    // 3. Load the full set back.
    load_all_variables(db).await
}

/// JWT_SECRET is not declared as an `auto_generate: true` `ConfigVar` by the
/// auth block (a wafer-run config-keys gap noted in the auth block module), so
/// the auto-gen loop above never seeds it. Seed it here so the strict
/// empty-secret boot check (native `server.rs`) can't trip on a fresh DB and
/// the browser/CF crypto can pick up a real key. Idempotent.
async fn seed_jwt_secret(db: &Arc<dyn DatabaseService>) {
    let key = crate::blocks::auth::JWT_SECRET_KEY;
    let block = crate::config_vars::key_block_prefix(key);
    let mut bytes = [0u8; 32];
    if let Err(e) = getrandom::getrandom(&mut bytes) {
        tracing::warn!(error = %e, "getrandom failed for JWT secret");
        return;
    }
    let secret: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let data = build_variable_row(
        key,
        &secret,
        "JWT signing secret",
        "256-bit secret used to sign access + refresh JWTs.",
        "Rotating this secret invalidates every issued session.",
        true,
        &block,
    );
    match insert_if_absent(db, key, data).await {
        Ok(true) => tracing::warn!(key = %key, "auto-generated JWT secret (not found in variables table)"),
        Ok(false) => {}
        Err(e) => tracing::warn!(key = %key, error = %e, "failed to seed JWT secret"),
    }
}

/// Read every row of the admin variables table into a key→value map. Rows with
/// an empty `key` are skipped (and warned) as corruption rather than silently
/// dropped.
pub async fn load_all_variables(
    db: &Arc<dyn DatabaseService>,
) -> Result<HashMap<String, String>, String> {
    let opts = ListOptions {
        offset: 0,
        limit: 100_000,
        skip_count: true,
        ..Default::default()
    };
    let listed = db
        .list(VARIABLES_TABLE, &opts)
        .await
        .map_err(|e| format!("load variables from {VARIABLES_TABLE}: {e}"))?;
    let mut vars = HashMap::new();
    for record in listed.records {
        let key = record
            .data
            .get("key")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if key.is_empty() {
            tracing::warn!("variables table contains a row with an empty key");
            continue;
        }
        let value = record
            .data
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        vars.insert(key.to_string(), value.to_string());
    }
    Ok(vars)
}
