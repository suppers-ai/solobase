//! Solobase — Rust binary entry point.
//!
//! All config comes from environment variables (auto-loaded from `.env`).
//! Infrastructure uses `SOLOBASE_*` prefix (no double-underscore).
//! App config uses convention-based prefixes and is seeded into the
//! `variables` table (single source of truth):
//! - `SOLOBASE_SHARED__*` — shared platform variables
//! - `{ORG}__{BLOCK}__*` — block-scoped variables (e.g., `SUPPERS_AI__AUTH__*`)
//!
//! Sensitive detection uses naming convention: keys ending in `_SECRET` or
//! `_KEY` are marked sensitive=1 in the DB. No hardcoded lists.
//!
//! Startup:
//! 1. Load `.env` file (auto-detect or SOLOBASE_ENV_FILE)
//! 2. Read SOLOBASE_* env vars for infrastructure config
//! 3. Seed matching env vars into variables table (INSERT OR IGNORE)
//! 4. Read variables table → JWT secret, feature flags, app config
//! 5. Start WAFER runtime

use std::collections::HashMap;
use std::sync::Arc;

mod app_config;
use app_config::{load_block_settings, InfraConfig};

use solobase::builder::{self, SolobaseBuilder};
use tracing_subscriber::{fmt, EnvFilter};
use wafer_core::interfaces::config::service::ConfigService;
use wafer_run::Wafer;

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // 1. Load .env file (before reading any env vars)
    load_dotenv();

    // 2. Initialize tracing / logging
    let log_format = std::env::var("SOLOBASE_LOG_FORMAT").unwrap_or_else(|_| "text".into());
    init_tracing(&log_format);
    tracing::info!("solobase starting (Rust/WAFER runtime)");

    // 3. Read infrastructure config from SOLOBASE_* env vars
    let infra = InfraConfig::from_env();
    tracing::info!(
        listen = %infra.listen,
        db = %infra.db_type,
        db_path = %infra.db_path,
        storage = %infra.storage_type,
        "infrastructure config loaded"
    );

    // 4. Collect app config vars from env (non-SOLOBASE_* prefixed)
    let env_vars = collect_app_env_vars();

    // 5. Open SQLite directly, seed variables, read config
    let vars = seed_and_load_variables(&infra.db_path, &env_vars);
    tracing::info!(vars = vars.len(), "variables loaded from database");

    // 6. Extract JWT secret and feature config from variables
    let jwt_secret = vars
        .get("SUPPERS_AI__AUTH__JWT_SECRET")
        .cloned()
        .unwrap_or_default();
    let features = load_block_settings(&infra.db_path);

    // 7. Build WAFER runtime via SolobaseBuilder
    let config_service = wafer_block_config::service::EnvConfigService::new();
    for (key, value) in &vars {
        config_service.set(key, value);
    }

    let (mut wafer, storage_block) = SolobaseBuilder::new()
        .database(Arc::new(
            wafer_block_sqlite::service::SQLiteDatabaseService::open(&infra.db_path)
                .expect("failed to open SQLite database"),
        ))
        .storage(Arc::new(
            wafer_block_local_storage::service::LocalStorageService::new(&infra.storage_root)
                .expect("failed to create local storage service"),
        ))
        .config(Arc::new(config_service))
        .crypto(Arc::new(
            wafer_block_crypto::service::Argon2JwtCryptoService::new(jwt_secret),
        ))
        .network(Arc::new(wafer_block_network::service::HttpNetworkService::new()))
        .logger(Arc::new(wafer_block_logger::service::TracingLogger))
        .block_settings(features)
        .build()
        .expect("failed to build solobase runtime");

    // 8. Native-only: register http-listener
    wafer_block_http_listener::register(&mut wafer).expect("register http-listener");
    wafer.add_block_config(
        "wafer-run/http-listener",
        serde_json::json!({ "flow": "site-main", "listen": infra.listen }),
    );

    // 9. Register observability hooks
    register_observability_hooks(&mut wafer);

    // 10. Load custom WRAP grants from DB
    let db_grants = app_config::load_wrap_grants(&infra.db_path);
    if !db_grants.is_empty() {
        tracing::info!(count = db_grants.len(), "loaded custom WRAP grants from database");
        wafer.add_wrap_grants(db_grants);
    }

    // 11. Start runtime
    let wafer = wafer
        .start()
        .await
        .expect("failed to start WAFER runtime");

    // 12. Inject WRAP grants into storage block
    builder::post_start(&wafer, &storage_block);
    tracing::info!("WAFER runtime started — all blocks resolved");

    // 13. Wait for shutdown signal
    shutdown_signal().await;

    // 14. Graceful shutdown
    wafer.shutdown().await;
    tracing::info!("solobase shutdown complete");
}

// ---------------------------------------------------------------------------
// .env loading
// ---------------------------------------------------------------------------

/// Auto-detect `.env` file or use `SOLOBASE_ENV_FILE` override.
fn load_dotenv() {
    // Check for explicit path override first
    if let Ok(path) = std::env::var("SOLOBASE_ENV_FILE") {
        match dotenvy::from_filename(&path) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("warning: failed to load env file '{path}': {e}");
            }
        }
        return;
    }

    // Auto-detect .env in current directory (standard behavior)
    let _ = dotenvy::dotenv();
}

/// Collect app config env vars that match declared config variable keys.
///
/// Only seeds env vars that are actually declared in either:
/// - `shared_config_vars()` — platform shared variables
/// - Block `config_keys` — block-scoped variables
///
/// This prevents random env vars with `__` from polluting the variables table.
fn collect_app_env_vars() -> Vec<(String, String)> {
    // Collect all known config var keys from declarations
    let block_infos = solobase_core::blocks::all_block_infos();
    let all_vars = solobase_core::config_vars::collect_all_config_vars(&block_infos);

    let known_keys: std::collections::HashSet<String> =
        all_vars.iter().map(|v| v.key.clone()).collect();

    std::env::vars()
        .filter(|(key, _)| known_keys.contains(key))
        .collect()
}

// ---------------------------------------------------------------------------
// SQLite variable seeding and loading
// ---------------------------------------------------------------------------

/// Ensure the variables table exists, seed from env vars, and return all variables.
fn seed_and_load_variables(
    db_path: &str,
    env_vars: &[(String, String)],
) -> HashMap<String, String> {
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(db_path).parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| {
            tracing::error!(
                "failed to create database directory {}: {e}",
                parent.display()
            );
            std::process::exit(1);
        });
    }

    let conn = rusqlite::Connection::open(db_path).unwrap_or_else(|e| {
        tracing::error!("failed to open SQLite at {db_path}: {e}");
        std::process::exit(1);
    });

    // Create variables table if it doesn't exist
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS variables (
            id TEXT PRIMARY KEY,
            key TEXT NOT NULL UNIQUE,
            name TEXT DEFAULT '',
            description TEXT DEFAULT '',
            value TEXT DEFAULT '',
            warning TEXT DEFAULT '',
            sensitive INTEGER DEFAULT 0,
            updated_by TEXT DEFAULT '',
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_variables_key ON variables (key);",
    )
    .unwrap_or_else(|e| {
        tracing::error!("failed to create variables table: {e}");
        std::process::exit(1);
    });

    // Seed from env vars (INSERT OR IGNORE — existing DB values take priority)
    {
        let mut stmt = conn.prepare(
            "INSERT OR IGNORE INTO variables (id, key, value, sensitive, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))"
        ).expect("failed to prepare seed statement");

        for (key, value) in env_vars {
            let id = format!("var_{}", uuid::Uuid::new_v4());
            let sensitive = if key.ends_with("_SECRET") || key.ends_with("_KEY") {
                1
            } else {
                0
            };
            if let Err(e) = stmt.execute(rusqlite::params![id, key, value, sensitive]) {
                tracing::warn!(key = %key, error = %e, "failed to seed variable");
            }
        }
    }

    // Auto-generate secrets for config vars marked with auto_generate
    seed_auto_generated(&conn);

    // Load all variables
    let mut vars = HashMap::new();
    let mut stmt = conn
        .prepare("SELECT key, value FROM variables")
        .expect("failed to prepare SELECT variables");
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .expect("failed to query variables");

    for (key, value) in rows.flatten() {
        if !key.is_empty() {
            vars.insert(key, value);
        }
    }

    vars
}

/// Auto-generate values for config vars marked with `auto_generate: true`.
///
/// Reads all block config var declarations, finds those needing auto-generation,
/// and generates random values for any that don't already exist in the variables table.
fn seed_auto_generated(conn: &rusqlite::Connection) {
    let block_infos = solobase_core::blocks::all_block_infos();
    let all_vars = solobase_core::config_vars::collect_all_config_vars(&block_infos);

    let mut stmt = conn.prepare(
        "INSERT OR IGNORE INTO variables (id, key, name, description, value, warning, sensitive, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), datetime('now'))"
    ).expect("failed to prepare auto-generate statement");

    for var in &all_vars {
        if !var.auto_generate {
            continue;
        }

        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).expect("failed to generate random secret");
        let secret: String = bytes.iter().map(|b| format!("{b:02x}")).collect();

        let id = format!("var_{}", uuid::Uuid::new_v4());
        let sensitive: i32 = if var.is_sensitive() { 1 } else { 0 };

        let affected = stmt
            .execute(rusqlite::params![
                id,
                var.key,
                var.name,
                var.description,
                secret,
                var.warning,
                sensitive
            ])
            .unwrap_or(0);

        if affected > 0 {
            tracing::warn!(key = %var.key, "auto-generated secret (not found in variables table)");
        }
    }
}

// ---------------------------------------------------------------------------
// Tracing init
// ---------------------------------------------------------------------------

fn init_tracing(log_format: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,wafer=debug,solobase=debug"));

    #[cfg(feature = "otel")]
    {
        if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok() {
            init_tracing_with_otel(log_format, filter);
            return;
        }
    }

    if log_format == "json" {
        fmt()
            .json()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(false)
            .init();
    } else {
        fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(false)
            .init();
    }
}

#[cfg(feature = "otel")]
fn init_tracing_with_otel(log_format: &str, filter: EnvFilter) {
    use opentelemetry::trace::TracerProvider;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::Layer;

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("failed to create OTLP span exporter");

    let service_name = std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "solobase".into());
    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", service_name),
        ]))
        .build();

    let tracer = provider.tracer("solobase");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let fmt_layer: Box<dyn Layer<_> + Send + Sync> = if log_format == "json" {
        Box::new(fmt::layer().json().with_target(true).with_thread_ids(false))
    } else {
        Box::new(fmt::layer().with_target(true).with_thread_ids(false))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    tracing::info!("OpenTelemetry tracing enabled");
}

// ---------------------------------------------------------------------------
// Observability hooks
// ---------------------------------------------------------------------------

fn register_observability_hooks(wafer: &mut Wafer) {
    wafer.hooks.on_flow_start(|flow_id, _msg| {
        tracing::info_span!("flow", flow = %flow_id).in_scope(|| {});
    });

    wafer.hooks.on_block_end(|obs_ctx, result, duration| {
        let status = match result.action {
            wafer_run::Action::Error => "ERROR",
            wafer_run::Action::Respond => "RESPOND",
            wafer_run::Action::Continue => "CONTINUE",
            wafer_run::Action::Drop => "DROP",
        };
        tracing::debug!(
            flow   = %obs_ctx.flow_id,
            block  = %obs_ctx.block_name,
            trace  = %obs_ctx.trace_id,
            status = status,
            ms     = duration.as_millis() as u64,
            "block executed"
        );
    });

    wafer.hooks.on_flow_end(|flow_id, result, duration| {
        let status = match result.action {
            wafer_run::Action::Error => "ERROR",
            _ => "OK",
        };
        tracing::info!(
            flow   = %flow_id,
            status = status,
            ms     = duration.as_millis() as u64,
            "flow completed"
        );
    });
}

// ---------------------------------------------------------------------------
// Graceful shutdown
// ---------------------------------------------------------------------------

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("received Ctrl+C — shutting down"),
        _ = terminate => tracing::info!("received SIGTERM — shutting down"),
    }
}
