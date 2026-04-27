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

use std::{collections::HashMap, sync::Arc};

mod config;
use clap::{Parser, Subcommand};
use config::{filter_to_declared_keys, load_block_settings, load_wrap_grants};
use solobase_core::builder::{self, SolobaseBuilder};
use solobase_native::{
    collect_app_env_vars, init_tracing, load_dotenv, register_http_listener,
    register_observability_hooks, serve_until_shutdown, InfraConfig,
};
use wafer_core::interfaces::config::service::ConfigService;

// ---------------------------------------------------------------------------
// CLI parser
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "solobase")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Temporary: runs the legacy solobase-cli build pipeline (wasm-pack +
    /// export-assets-equivalent) against the current dir. Used by CI until
    /// Task 7 introduces the unified CLI's `build --target web`. To be
    /// removed when that lands.
    LegacyBuild,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // Parse a subcommand. Bare `solobase` (no args) yields
    // `Cli { command: None }` and falls through to server boot — preserves
    // examples/run-tests.sh behavior. A parse error makes clap exit the
    // process with help text, so any non-empty unknown args fail loudly.
    let cli = Cli::parse();
    if let Some(Commands::LegacyBuild) = cli.command {
        let cwd = std::env::current_dir().expect("cwd");
        let (cfg, repo_root) = match solobase::cli::legacy_config::find_and_load(&cwd) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        };
        // Use Dev profile to match today's CI behavior (faster).
        if let Err(e) = solobase::cli::legacy_build::run(
            &cfg,
            &repo_root,
            solobase::cli::legacy_build::BuildProfile::Dev,
        ) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }

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

    // 4. Collect app config vars from env (non-SOLOBASE_* prefixed, filtered to declared keys)
    let env_vars = filter_to_declared_keys(collect_app_env_vars());

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
        .database(solobase_native::make_sqlite_database_service(
            &infra.db_path,
        ))
        .storage(solobase_native::make_local_storage_service(
            &infra.storage_root,
        ))
        .config(Arc::new(config_service))
        .crypto(solobase_native::make_jwt_crypto_service(jwt_secret))
        .network(solobase_native::make_fetch_network_service())
        .logger(solobase_native::make_tracing_logger())
        .block_settings(features)
        // Hand the SQLite path to the builder so the `native-embedding`
        // feature can open a dedicated connection for `SqliteVecService`.
        // Ignored when the feature is off.
        .sqlite_db_path(&infra.db_path)
        .build()
        .expect("failed to build solobase runtime");

    // 8. Native-only: register http-listener.
    //    solobase dispatches all HTTP traffic through the `site-main` flow
    //    (see crates/solobase/src/flows/site_main.rs).
    register_http_listener(&mut wafer, &infra.listen, "site-main");

    // 9. Register observability hooks
    register_observability_hooks(&mut wafer);

    // 10. Load custom WRAP grants from DB
    let db_grants = load_wrap_grants(&infra.db_path);
    if !db_grants.is_empty() {
        tracing::info!(
            count = db_grants.len(),
            "loaded custom WRAP grants from database"
        );
        wafer.add_wrap_grants(db_grants);
    }

    // 11. Start runtime
    let wafer = wafer.start().await.expect("failed to start WAFER runtime");

    // 12. Inject WRAP grants into storage block
    builder::post_start(&wafer, &storage_block);
    tracing::info!("WAFER runtime started — all blocks resolved");

    // 13. Wait for shutdown signal, then graceful shutdown
    serve_until_shutdown(&wafer).await;
    tracing::info!("solobase shutdown complete");
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
