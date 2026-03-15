//! Solobase — Rust binary entry point.
//!
//! All feature blocks are implemented as native Rust structs implementing
//! the Block trait. The WAFER runtime handles HTTP serving and block lifecycle.
//! Routing is handled by `solobase-core`'s shared pipeline via the
//! `suppers-ai/router` block, which replaces individual per-feature flow files.

use std::sync::Arc;

use solobase::app_config::AppConfig;
use solobase::blocks;
use solobase::blocks::router::{NativeBlockFactory, SolobaseRouterBlock};
use solobase::flows;

use tracing_subscriber::{fmt, EnvFilter};
use wafer_run::Wafer;

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // 1. Initialize tracing / logging
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "text".into());
    init_tracing(&log_format);
    tracing::info!("solobase starting (Rust/WAFER runtime)");

    // 2. Create WAFER runtime
    let mut wafer = Wafer::new();

    // 3. Load solobase.json config
    let app_config = load_config(&mut wafer);
    tracing::info!("block configs loaded");

    // 4. Register blocks explicitly (no register_all — runtime is minimal)
    //    Infrastructure blocks:
    wafer_block_auth_validator::register(&mut wafer);
    wafer_block_cors::register(&mut wafer);
    wafer_block_iam_guard::register(&mut wafer);
    wafer_block_inspector::register(&mut wafer);
    wafer_block_monitoring::register(&mut wafer);
    wafer_block_ip_rate_limit::register(&mut wafer);
    wafer_block_readonly_guard::register(&mut wafer);
    wafer_block_router::register(&mut wafer);
    wafer_block_security_headers::register(&mut wafer);
    wafer_block_web::register(&mut wafer);
    wafer_block_config::register(&mut wafer);
    wafer_block_logger::register(&mut wafer);
    #[cfg(feature = "server")]
    {
        wafer_block_crypto::register(&mut wafer);
        wafer_block_network::register(&mut wafer);
        wafer_block_http_listener::register(&mut wafer);
    }
    //    Database + storage blocks:
    wafer_block_sqlite::register(&mut wafer);
    wafer_block_local_storage::register(&mut wafer);
    tracing::info!("blocks registered");

    // 5. Create shared block instances and register the solobase router
    let enabled = app_config.enabled_features();
    let shared_blocks = blocks::create_blocks(|name| enabled.contains(&name));

    // Register blocks with runtime for lifecycle hooks
    blocks::register_shared_blocks(&mut wafer, &shared_blocks);

    // Build the router block with shared factory
    let jwt_secret = app_config.jwt_secret.clone().unwrap_or_default();
    let features: Arc<dyn solobase_core::FeatureConfig> =
        Arc::new(app_config.feature_config());
    let factory = NativeBlockFactory::new(shared_blocks);
    let router = SolobaseRouterBlock::new(jwt_secret, features, factory);
    wafer.register_block("suppers-ai/router", Arc::new(router));
    tracing::info!("native feature blocks registered");

    // 6. Register flow definitions
    flows::register_site_main(&mut wafer);
    tracing::info!("flow definitions registered");

    // 7. Register observability hooks
    register_observability_hooks(&mut wafer);

    // 8. Start WAFER runtime (resolves flows, runs lifecycle init, binds listeners)
    let wafer = wafer
        .start()
        .await
        .expect("failed to resolve and start WAFER runtime");
    tracing::info!("WAFER runtime started — all blocks resolved");

    // 9. Wait for shutdown signal
    shutdown_signal().await;

    // 10. Graceful shutdown
    wafer.shutdown().await;
    tracing::info!("solobase shutdown complete");
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
// Config loading
// ---------------------------------------------------------------------------

fn load_config(wafer: &mut Wafer) -> AppConfig {
    let app_json = std::env::var("SOLOBASE_CONFIG").unwrap_or_else(|_| "solobase.json".into());

    let cfg = AppConfig::load(&app_json).unwrap_or_else(|e| {
        tracing::error!("failed to load {app_json}: {e}");
        std::process::exit(1);
    });

    let name = cfg.app.as_deref().unwrap_or("solobase");
    tracing::info!(app = name, version = cfg.version, config = %app_json, "loaded app config");

    // Expand app config into block configs and aliases
    let (block_configs, aliases) = cfg.to_blocks_json();
    for (name, config) in block_configs {
        wafer.add_block_config(name, config);
    }
    for (alias, target) in aliases {
        wafer.add_alias(alias, target);
    }
    cfg
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
