//! Solobase — Rust binary entry point.
//!
//! All feature blocks are implemented as native Rust structs implementing
//! the Block trait. The WAFER runtime, HTTP server, and embedded frontend
//! are provided by this binary. Infrastructure blocks self-configure from
//! `blocks.json`.

mod blocks;
mod flows;
mod embedded;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
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

    // 3. Load infrastructure block configs from blocks.json
    let blocks_json = std::env::var("BLOCKS_JSON").unwrap_or_else(|_| "blocks.json".into());
    if let Err(e) = wafer.load_blocks_json(&blocks_json) {
        tracing::warn!("could not load {}: {} — using defaults", blocks_json, e);
    }
    tracing::info!("block configs loaded");

    // 4. Register wafer-core infrastructure blocks
    //    (security-headers, cors, rate-limit, readonly-guard, monitoring, auth, iam, web)
    wafer_core::register_all(&mut wafer);
    tracing::info!("wafer-core blocks registered");

    // 5. Register native Rust feature blocks (env-var-driven)
    blocks::register_selected(&mut wafer);
    tracing::info!("native feature blocks registered");

    // 6. Register flow definitions (wafer-core base flows + solobase feature flows)
    let _ = wafer_core::flows::register_flows(&mut wafer);
    flows::register_selected_flows(&mut wafer);
    tracing::info!("flow definitions registered");

    // 7. Register observability hooks
    register_observability_hooks(&mut wafer);

    // 8. Resolve all flows (creates block instances, runs lifecycle init)
    wafer
        .start()
        .expect("failed to resolve and start WAFER runtime");
    tracing::info!("WAFER runtime started — all blocks resolved");

    // 9. Build HTTP router
    let wafer = Arc::new(wafer);
    let app = build_router(wafer.clone());

    // 10. Start axum HTTP server
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8090".into());
    let addr: SocketAddr = bind_addr
        .parse()
        .expect("invalid BIND_ADDR — expected host:port");

    tracing::info!(%addr, "HTTP server listening");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("HTTP server error");

    tracing::info!("solobase shutdown complete");
}

// ---------------------------------------------------------------------------
// Tracing init
// ---------------------------------------------------------------------------

fn init_tracing(log_format: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,wafer=debug,solobase=debug"));

    let is_json = log_format == "json";

    if is_json {
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
    // Log every block execution for debugging / audit.
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

    // Log flow-level summary.
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
// HTTP router
// ---------------------------------------------------------------------------

fn build_router(wafer: Arc<Wafer>) -> Router {
    // Create a single router that dispatches to the main flow.
    // axum strips the /api prefix before passing to handlers, so flow routes
    // stay clean (e.g. /health, /auth/login, /admin/users).
    let api_router = wafer_core::bridge::http::create_router(wafer, "site-main");

    // Embedded frontend (SPA)
    let frontend_router = embedded::frontend_router();

    // Compose: /api/* routes handled by flows, everything else by SPA
    Router::new()
        .nest("/api", api_router)
        .fallback_service(frontend_router)
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
