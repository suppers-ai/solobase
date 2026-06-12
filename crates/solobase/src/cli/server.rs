//! Server-boot body lifted from the previous `main.rs`.
//!
//! `run()` is invoked by the sealed × native flow (and by the bare-`solobase`
//! shortcut path in `main.rs`). It owns the SQLite seeding, WAFER builder,
//! HTTP listener registration, and the `serve_until_shutdown` loop.

use std::{collections::HashMap, path::Path, sync::Arc};

use anyhow::{anyhow, Context};
use solobase_core::builder::{self, SolobaseBuilder};
use solobase_native::{
    collect_app_env_vars, init_tracing, load_dotenv, register_http_listener,
    register_observability_hooks, serve_until_shutdown, InfraConfig,
};
use wafer_core::interfaces::config::service::ConfigService;

use crate::cli::server_config::{filter_to_declared_keys, load_block_settings, load_wrap_grants};

/// Boot the native server end-to-end. The body mirrors the previous
/// `main()` exactly; the signature is `pub async fn run()` so the new
/// dispatcher can `await` it as the sealed × native flow.
///
/// `run_migrations` mirrors `solobase serve --run-migrations`. When `true`
/// the boot path stamps `SOLOBASE_RUN_MIGRATIONS=1` into the config
/// snapshot directly (so [`migration_helper::apply_if_blessed`] sees it),
/// instead of the prior `std::env::set_var` smuggle. Rust 2024 makes
/// process-env mutation `unsafe`, and the smuggle leaked into any child
/// process the boot path might spawn — neither was the right channel.
pub async fn run(repo_root: &Path, run_migrations: bool) -> anyhow::Result<()> {
    // 1. Load .env file (before reading any env vars). Anchored to
    // `repo_root` so the boot path doesn't depend on the process cwd —
    // mutating cwd globally would leak into anything else this binary
    // (or a future caller) spawns.
    load_dotenv(repo_root);

    // 2. Initialize tracing / logging
    let log_format = std::env::var("SOLOBASE_LOG_FORMAT").unwrap_or_else(|_| "text".into());
    init_tracing(&log_format).context("initialize tracing subscriber")?;
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
    let vars = seed_and_load_variables(&infra.db_path, &env_vars)?;
    tracing::info!(vars = vars.len(), "variables loaded from database");

    // 6. Extract JWT secret and feature config from variables. An empty
    // JWT secret would silently fail-open every token verification; bail
    // explicitly so the operator sees the misconfiguration at boot.
    let jwt_secret = vars
        .get(solobase_core::blocks::auth::JWT_SECRET_KEY)
        .cloned()
        .ok_or_else(|| {
            anyhow!(
                "missing required variable `{}` — auto-generation should seed this; \
                 the variables table is unreadable or corrupted",
                solobase_core::blocks::auth::JWT_SECRET_KEY
            )
        })?;
    if jwt_secret.is_empty() {
        return Err(anyhow!(
            "variable `{}` is set but empty — refusing to boot with an empty JWT secret",
            solobase_core::blocks::auth::JWT_SECRET_KEY
        ));
    }
    let features = load_block_settings(&infra.db_path);

    // 7. Build WAFER runtime via SolobaseBuilder
    let config_service = wafer_core::service_blocks::config::EnvConfigService::new();
    for (key, value) in &vars {
        config_service.set(key, value);
    }
    // Fan-out block_settings into the config snapshot so consumer blocks
    // (e.g. userportal) can read enablement state via `ctx.config_get`
    // without re-querying the `block_settings` SQLite table per request.
    config_service.set(
        solobase_core::features::BLOCK_SETTINGS_CONFIG_KEY,
        &features.to_config_json(),
    );
    if run_migrations {
        config_service.set(solobase_core::migration_helper::RUN_MIGRATIONS_KEY, "1");
    }

    // Build the parallel snapshot map fed to `Wafer::set_config_snapshot`.
    // `EnvConfigService` is the async (`wafer-run/config`) read surface;
    // the snapshot is the synchronous `ctx.config_get` surface. Both must
    // carry the same data so `migration_helper::apply_if_blessed` (which
    // reads `BLOCK_SETTINGS_CONFIG_KEY` + `SOLOBASE_RUN_MIGRATIONS` via
    // `config_get`) sees the boot values without a per-call D1 hop. See
    // `docs/superpowers/specs/2026-05-14-config-snapshot-and-migration-gate-design.md`.
    let mut snapshot: HashMap<String, String> = vars.clone();
    snapshot.insert(
        solobase_core::features::BLOCK_SETTINGS_CONFIG_KEY.to_string(),
        features.to_config_json(),
    );
    if run_migrations {
        snapshot.insert(
            solobase_core::migration_helper::RUN_MIGRATIONS_KEY.to_string(),
            "1".to_string(),
        );
    }

    let (mut wafer, storage_block) = SolobaseBuilder::new()
        .database(solobase_native::make_sqlite_database_service(
            &infra.db_path,
        )?)
        .storage(solobase_native::make_local_storage_service(
            &infra.storage_root,
        )?)
        .config(Arc::new(config_service))
        .config_source(Arc::new(wafer_run::StaticConfigSource::new(vars.clone())))
        .crypto(solobase_native::make_jwt_crypto_service(jwt_secret)?)
        .network(solobase_native::make_fetch_network_service())
        .logger(solobase_native::make_tracing_logger())
        .block_settings(features)
        // Hand the SQLite path to the builder so the `native-embedding`
        // feature can open a dedicated connection for `SqliteVecService`.
        // Ignored when the feature is off.
        .sqlite_db_path(&infra.db_path)
        .build()
        .context("build solobase runtime")?;

    // 7b. Wire the env-var snapshot into `RuntimeContext.config` so blocks
    //     can read embedder-provided keys via `ctx.config_get` synchronously.
    //     Mirrors the cloudflare embedder; both surfaces carry identical
    //     data (see snapshot construction above).
    wafer.set_config_snapshot(snapshot);

    // 8. Native-only: register http-listener.
    //    solobase dispatches all HTTP traffic through the `site-main` flow
    //    (see crates/solobase-core/src/flows/site_main.rs).
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

    // 11. Start runtime. We init the admin block first so its migrations
    //     (which create suppers_ai__admin__block_settings + the variables
    //     table) finish before any other block's Init tries to write to
    //     block_settings via migration_helper. Without this, HashMap key-
    //     iteration order could put another block first, hit a hard
    //     'no such table' error (since solobase #182 made write_state
    //     propagate strictly), and the cascade would skip auth's bootstrap
    //     — manifesting as a login 401 on the freshly-booted server in
    //     CI E2E.
    //
    //     Mirrors the CF runner's seal → init_block(admin) → init_all_blocks
    //     sequence (solobase #186); the runtime API for this ordering hook
    //     landed in wafer-run #143.
    let wafer = wafer
        .start_with_priority(&[solobase_core::blocks::admin::ADMIN_BLOCK_ID])
        .await
        .context("start WAFER runtime")?;

    // 12. Inject WRAP grants into storage block
    builder::post_start(&wafer, &storage_block);
    tracing::info!("WAFER runtime started — all blocks resolved");

    // 13. Wait for shutdown signal, then graceful shutdown
    serve_until_shutdown(&wafer)
        .await
        .context("await shutdown signal")?;
    tracing::info!("solobase shutdown complete");

    Ok(())
}

// ---------------------------------------------------------------------------
// SQLite variable seeding and loading
// ---------------------------------------------------------------------------

/// Canonical variables table name, sourced from the admin block so the
/// boot loader and the `/b/admin/settings/variables` UI always read and
/// write the same place. Earlier versions used a bare `variables` table
/// here, which drifted from the admin block's prefixed `CollectionSchema`
/// and silently divided the config into two stores. The constant lives
/// in `solobase-core::blocks::admin` so there's no second source of truth.
const VARIABLES_TABLE: &str = solobase_core::blocks::admin::VARIABLES_TABLE;

/// Ensure the variables table exists, seed from env vars, and return all variables.
fn seed_and_load_variables(
    db_path: &str,
    env_vars: &[(String, String)],
) -> anyhow::Result<HashMap<String, String>> {
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(db_path).parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create database directory {}", parent.display()))?;
    }

    let conn =
        rusqlite::Connection::open(db_path).with_context(|| format!("open SQLite at {db_path}"))?;

    // Create variables table if it doesn't exist.
    //
    // Boot runs before WAFER, so the admin block's lifecycle hasn't created
    // the table yet — we have to pre-create it via raw SQL. Schema mirrors
    // the admin block's `CollectionSchema`: the user-visible columns are
    // declared there, and `ensure_table` adds the `id`/`created_at`/
    // `updated_at` columns the WAFER DB client expects. Pre-creating here
    // with the union of both is harmless (the columns line up; later
    // `ensure_table` calls see they exist and skip).
    let create_sql = format!(
        "CREATE TABLE IF NOT EXISTS \"{VARIABLES_TABLE}\" (
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
        CREATE UNIQUE INDEX IF NOT EXISTS \"idx_{VARIABLES_TABLE}_key\" \
         ON \"{VARIABLES_TABLE}\" (key);"
    );
    conn.execute_batch(&create_sql)
        .context("create variables table")?;

    // Seed from env vars (INSERT OR IGNORE — existing DB values take priority)
    {
        let insert_sql = format!(
            "INSERT OR IGNORE INTO \"{VARIABLES_TABLE}\" \
             (id, key, value, sensitive, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))"
        );
        let mut stmt = conn
            .prepare(&insert_sql)
            .context("prepare seed-variables statement")?;

        for (key, value) in env_vars {
            let id = format!("var_{}", uuid::Uuid::new_v4());
            let sensitive = i32::from(key.ends_with("_SECRET") || key.ends_with("_KEY"));
            if let Err(e) = stmt.execute(rusqlite::params![id, key, value, sensitive]) {
                tracing::warn!(key = %key, error = %e, "failed to seed variable");
            }
        }
    }

    // Auto-generate secrets for config vars marked with auto_generate
    seed_auto_generated(&conn)?;

    // Load all variables
    let mut vars = HashMap::new();
    let select_sql = format!("SELECT key, value FROM \"{VARIABLES_TABLE}\"");
    let mut stmt = conn
        .prepare(&select_sql)
        .context("prepare SELECT variables statement")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .context("query variables")?;

    for row in rows {
        let (key, value) = row.context("read variables row")?;
        if key.is_empty() {
            // Empty key means a corrupt row — surface as warning rather than
            // silently dropping (DB corruption is a real failure case).
            tracing::warn!("variables table contains a row with an empty key");
            continue;
        }
        vars.insert(key, value);
    }

    Ok(vars)
}

/// Auto-generate values for config vars marked with `auto_generate: true`.
///
/// Reads all block config var declarations, finds those needing auto-generation,
/// and generates random values for any that don't already exist in the variables table.
fn seed_auto_generated(conn: &rusqlite::Connection) -> anyhow::Result<()> {
    let block_infos = solobase_core::blocks::all_block_infos();
    let all_vars = solobase_core::config_vars::collect_all_config_vars(&block_infos);

    let insert_sql = format!(
        "INSERT OR IGNORE INTO \"{VARIABLES_TABLE}\" \
         (id, key, name, description, value, warning, sensitive, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), datetime('now'))"
    );
    let mut stmt = conn
        .prepare(&insert_sql)
        .context("prepare auto-generate statement")?;

    let mut seed =
        |key: &str, name: &str, description: &str, warning: &str| -> anyhow::Result<()> {
            let mut bytes = [0u8; 32];
            getrandom::getrandom(&mut bytes).context("generate random secret")?;
            let secret: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            let id = format!("var_{}", uuid::Uuid::new_v4());
            let affected = stmt
                .execute(rusqlite::params![
                    id,
                    key,
                    name,
                    description,
                    secret,
                    warning,
                    1_i32
                ])
                .unwrap_or(0);
            if affected > 0 {
                tracing::warn!(key = %key, "auto-generated secret (not found in variables table)");
            }
            Ok(())
        };

    for var in &all_vars {
        if !var.auto_generate {
            continue;
        }
        seed(&var.key, &var.name, &var.description, &var.warning)?;
    }

    // JWT_SECRET is not declared as an `auto_generate: true` ConfigVar by
    // the auth block (the block's mod.rs:124-130 comment notes this as a
    // wafer-run config-keys gap). Seed it here so the strict empty-check
    // in `run()` doesn't trip on a fresh DB. Hardcoded because the const
    // is `pub` in solobase-core::blocks::auth, but the auto-gen pipeline
    // is owned by the CLI crate and shouldn't grow a cross-crate scan.
    seed(
        solobase_core::blocks::auth::JWT_SECRET_KEY,
        "JWT signing secret",
        "256-bit secret used to sign access + refresh JWTs.",
        "Rotating this secret invalidates every issued session.",
    )?;

    Ok(())
}
