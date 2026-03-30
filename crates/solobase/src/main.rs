//! Solobase — Rust binary entry point.
//!
//! All config comes from environment variables (auto-loaded from `.env`).
//! Infrastructure uses `SOLOBASE_*` prefix. App config is unprefixed and
//! seeded into the `variables` table (single source of truth).
//!
//! Startup:
//! 1. Load `.env` file (auto-detect or SOLOBASE_ENV_FILE)
//! 2. Read SOLOBASE_* env vars for infrastructure config
//! 3. Seed unprefixed env vars into variables table (INSERT OR IGNORE)
//! 4. Read variables table → JWT secret, feature flags, app config
//! 5. Start WAFER runtime

use std::collections::HashMap;
use std::sync::Arc;

use solobase::app_config::{InfraConfig, FeatureSnapshot};
use solobase::blocks;
use solobase::blocks::router::{NativeBlockFactory, SolobaseRouterBlock};
use solobase::flows;

use tracing_subscriber::{fmt, EnvFilter};
use wafer_run::Wafer;

/// Known sensitive variable keys — marked as sensitive=1 in the variables table
/// so the settings API masks their values.
const SENSITIVE_VARS: &[&str] = &[
    "JWT_SECRET",
    "STRIPE_SECRET_KEY",
    "STRIPE_WEBHOOK_SECRET",
    "MAILGUN_API_KEY",
    "PRODUCTS_WEBHOOK_SECRET",
    "CONTROL_PLANE_SECRET",
];

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // 1. Load .env file (before reading any env vars)
    load_dotenv();

    // 2. Initialize tracing / logging
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "text".into());
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
    let jwt_secret = vars.get("JWT_SECRET").cloned().unwrap_or_default();
    let features = FeatureSnapshot::from_vars(&vars);

    // 7. Create WAFER runtime
    let mut wafer = Wafer::new();

    // 8. Register non-service block configs (http-listener, web)
    let (block_configs, aliases) = infra.to_blocks_json();
    for (name, config) in block_configs {
        wafer.add_block_config(name, config);
    }
    for (alias, target) in aliases {
        wafer.add_alias(alias, target);
    }

    // 9. Register unified service blocks (database, storage, config, crypto, network, logger)
    {
        use wafer_core::interfaces::config::service::ConfigService;

        // Database — open SQLite (already opened above for variable seeding, but the
        // service needs its own connection for the block runtime)
        let db_service = Arc::new(
            wafer_block_sqlite::service::SQLiteDatabaseService::open(&infra.db_path)
                .expect("failed to open SQLite database for block runtime"),
        );
        wafer_core::service_blocks::database::register_with(&mut wafer, db_service);
        wafer.add_alias("db", "wafer-run/database");

        // Storage — local filesystem
        let storage_service = Arc::new(
            wafer_block_local_storage::service::LocalStorageService::new(&infra.storage_root)
                .expect("failed to create local storage service"),
        );
        wafer_core::service_blocks::storage::register_with(&mut wafer, storage_service);
        wafer.add_alias("storage", "wafer-run/storage");

        // Config — env vars with variables table overrides
        let config_service = wafer_block_config::service::EnvConfigService::new();
        for (key, value) in &vars {
            config_service.set(key, value);
        }
        wafer_core::service_blocks::config::register_with(&mut wafer, Arc::new(config_service));

        // Crypto — Argon2 password hashing + JWT
        let crypto_service = Arc::new(
            wafer_block_crypto::service::Argon2JwtCryptoService::new(jwt_secret.clone()),
        );
        wafer_core::service_blocks::crypto::register_with(&mut wafer, crypto_service);

        // Network — async HTTP client
        let network_service = Arc::new(wafer_block_network::service::HttpNetworkService::new());
        wafer_core::service_blocks::network::register_with(&mut wafer, network_service);

        // Logger — tracing
        let logger_service = Arc::new(wafer_block_logger::service::TracingLogger);
        wafer_core::service_blocks::logger::register_with(&mut wafer, logger_service);
    }

    // 10. Register middleware and other infrastructure blocks
    wafer_block_auth_validator::register(&mut wafer);
    wafer_block_cors::register(&mut wafer);
    wafer_block_iam_guard::register(&mut wafer);
    wafer_block_inspector::register(&mut wafer);
    wafer_block_readonly_guard::register(&mut wafer);
    wafer_block_router::register(&mut wafer);
    wafer_block_security_headers::register(&mut wafer);
    wafer_block_web::register(&mut wafer);
    #[cfg(feature = "server")]
    {
        wafer_block_http_listener::register(&mut wafer);
    }
    tracing::info!("infrastructure blocks registered");

    // 12. Create feature blocks based on variables-derived feature config
    let shared_blocks = blocks::create_blocks(|name| features.is_enabled(name));
    blocks::register_shared_blocks(&mut wafer, &shared_blocks);

    // 13. Build the solobase router
    let feature_config: Arc<dyn solobase_core::FeatureConfig> = Arc::new(features);
    let factory = NativeBlockFactory::new(shared_blocks);
    let router = SolobaseRouterBlock::new(jwt_secret, feature_config, factory);
    wafer.register_block("suppers-ai/router", Arc::new(router));
    tracing::info!("feature blocks registered");

    // 14. Register flow definitions
    flows::register_site_main(&mut wafer).unwrap_or_else(|e| {
        tracing::error!("failed to register site-main flow: {e}");
        std::process::exit(1);
    });

    // 15. Register observability hooks
    register_observability_hooks(&mut wafer);

    // 16. Start WAFER runtime
    let wafer = wafer
        .start()
        .await
        .expect("failed to resolve and start WAFER runtime");
    tracing::info!("WAFER runtime started — all blocks resolved");

    // 17. Wait for shutdown signal
    shutdown_signal().await;

    // 18. Graceful shutdown
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
            Ok(_) => {},
            Err(e) => {
                eprintln!("warning: failed to load env file '{path}': {e}");
            }
        }
        return;
    }

    // Auto-detect .env in current directory (standard behavior)
    let _ = dotenvy::dotenv();
}

/// Collect all non-SOLOBASE_* env vars that were loaded from the .env file.
///
/// We can't distinguish .env vars from pre-existing env vars, so we collect
/// ALL non-prefixed env vars and let INSERT OR IGNORE handle dedup in the DB.
/// Only well-known app config keys are seeded to avoid polluting the variables
/// table with system vars like PATH, HOME, etc.
fn collect_app_env_vars() -> Vec<(String, String)> {
    /// Known app config keys that should be seeded into the variables table.
    const APP_CONFIG_KEYS: &[&str] = &[
        "APP_NAME", "JWT_SECRET", "ALLOW_SIGNUP", "ENABLE_OAUTH",
        "PRIMARY_COLOR", "POST_LOGIN_REDIRECT", "FRONTEND_URL",
        "AUTH_ALLOWED_EMAIL_DOMAINS", "ADMIN_EMAIL",
        // Secrets
        "STRIPE_SECRET_KEY", "STRIPE_WEBHOOK_SECRET", "STRIPE_API_URL",
        "MAILGUN_API_KEY", "MAILGUN_DOMAIN", "MAILGUN_FROM",
        // Webhooks
        "PRODUCTS_WEBHOOK_URL", "PRODUCTS_WEBHOOK_SECRET",
        // Platform (cloud-only)
        "CONTROL_PLANE_URL", "CONTROL_PLANE_SECRET",
        // Feature flags
        "FEATURE_AUTH", "FEATURE_ADMIN", "FEATURE_FILES",
        "FEATURE_PRODUCTS", "FEATURE_PROJECTS", "FEATURE_LEGALPAGES",
        "FEATURE_USERPORTAL",
    ];

    let mut vars = Vec::new();
    for key in APP_CONFIG_KEYS {
        if let Ok(value) = std::env::var(key) {
            vars.push((key.to_string(), value));
        }
    }
    vars
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
            tracing::error!("failed to create database directory {}: {e}", parent.display());
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
        CREATE UNIQUE INDEX IF NOT EXISTS idx_variables_key ON variables (key);"
    ).unwrap_or_else(|e| {
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
            let sensitive = if SENSITIVE_VARS.contains(&key.as_str()) { 1 } else { 0 };
            if let Err(e) = stmt.execute(rusqlite::params![id, key, value, sensitive]) {
                tracing::warn!(key = %key, error = %e, "failed to seed variable");
            }
        }
    }

    // Generate JWT_SECRET if not present
    ensure_jwt_secret(&conn);

    // Load all variables
    let mut vars = HashMap::new();
    let mut stmt = conn.prepare("SELECT key, value FROM variables")
        .expect("failed to prepare SELECT variables");
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
        ))
    }).expect("failed to query variables");

    for row in rows {
        if let Ok((key, value)) = row {
            if !key.is_empty() {
                vars.insert(key, value);
            }
        }
    }

    vars
}

/// Generate and store a JWT_SECRET if one doesn't exist in the variables table.
fn ensure_jwt_secret(conn: &rusqlite::Connection) {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("failed to generate random JWT secret");
    let secret: String = bytes.iter().map(|b| format!("{b:02x}")).collect();

    let id = format!("var_{}", uuid::Uuid::new_v4());
    let affected = conn.execute(
        "INSERT OR IGNORE INTO variables (id, key, name, description, value, warning, sensitive, created_at, updated_at)
         VALUES (?1, 'JWT_SECRET', 'JWT Secret', 'Secret key used to sign authentication tokens',
                 ?2, 'Changing this will invalidate all existing user sessions', 1,
                 datetime('now'), datetime('now'))",
        rusqlite::params![id, secret],
    ).expect("failed to ensure JWT_SECRET");

    if affected > 0 {
        tracing::warn!("no JWT_SECRET found — generated and stored in variables table");
    }
}

// ---------------------------------------------------------------------------
// Tracing init
// ---------------------------------------------------------------------------

fn init_tracing(log_format: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,wafer=debug,solobase=debug"));

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

// ---------------------------------------------------------------------------
// Observability hooks
// ---------------------------------------------------------------------------

fn register_observability_hooks(wafer: &mut Wafer) {
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
