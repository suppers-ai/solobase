//! Server-boot body lifted from the previous `main.rs`.
//!
//! `run()` is invoked by the sealed × native flow (and by the bare-`solobase`
//! shortcut path in `main.rs`). It constructs the database service, seeds the
//! admin variables / block_settings tables pre-wafer through the shared
//! `solobase_core` seeders, builds the WAFER runtime, registers the HTTP
//! listener, and runs the `serve_until_shutdown` loop.

use std::{collections::HashMap, path::Path, sync::Arc};

use anyhow::{anyhow, Context};
use solobase_core::builder::{self, SolobaseBuilder};
use solobase_native::{
    collect_app_env_vars, init_tracing, load_dotenv, register_http_listener,
    register_observability_hooks, serve_until_shutdown, InfraConfig,
};
use wafer_core::interfaces::config::service::ConfigService;

use crate::cli::server_config::{filter_to_declared_keys, load_wrap_grants};

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

    // 5. Construct the platform database service up front. Native seeds the
    //    variables / block_settings tables BEFORE the wafer exists — it needs
    //    the JWT secret to construct its immutable crypto service, and
    //    `start_with_priority` (which binds the HTTP socket in the atomic
    //    Start pass) leaves no public hook to seed mid-lifecycle the way the
    //    stateless targets do via `solobase_core::builder::boot`. The same
    //    `Arc` is handed to the builder below, so seeding and the runtime
    //    share one connection/pool.
    let database = solobase_native::make_database_service(
        &infra.db_type,
        &infra.db_path,
        infra.db_url.as_deref(),
    )
    .await
    .context("construct database service")?;

    // Create the admin variables / block_settings tables pre-wafer by running
    // admin's migration-file SQL through the service (migration-file-runner
    // exception). Reuses the embedded `.sql` constants admin's gated `Init`
    // re-asserts later — single schema source, no hand-rolled CREATE TABLE.
    solobase_core::migration_helper::apply_ddl_via_service(
        &database,
        solobase_core::blocks::admin::migrations::ddl_files(&infra.db_type),
    )
    .await
    .map_err(|e| anyhow!("create admin tables pre-wafer: {e}"))?;

    // Seed env/auto-gen/JWT variables + run the #222 block-settings hash-gate,
    // all through the shared `solobase_core` seeders over the service.
    let vars = solobase_core::boot::seed_and_load_variables(&database, &env_vars)
        .await
        .map_err(|e| anyhow!("seed and load variables: {e}"))?;
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
    let features = solobase_core::features::load_and_seed_block_settings(&database).await;

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

    // Dispatch on the infra config: `SOLOBASE_STORAGE_TYPE` (local|s3) selects
    // the platform storage service. An unsupported value, or a type whose
    // cargo feature is off, is a hard boot error — the boot path no longer
    // logs `storage = s3` while silently running local disk. (The database
    // service was already constructed above and reused here.)
    let storage = solobase_native::make_storage_service(&infra.storage_type, &infra.storage_root)
        .await
        .context("construct storage service")?;

    let (mut wafer, storage_block) = SolobaseBuilder::new()
        .database(database)
        .storage(storage)
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
