//! HTTP listener registration + graceful-shutdown helpers for native
//! WAFER apps.
//!
//! The shape is deliberately two-phase: `register_http_listener` attaches
//! the `wafer-run/http-listener` block before start; `serve_until_shutdown`
//! awaits a ctrl-c / SIGTERM signal and shuts the runtime down. Splitting
//! them lets the consumer run post-start hooks (e.g., WRAP grant
//! injection) between `wafer.start()` and the shutdown wait.

use std::sync::Arc;

use wafer_run::Wafer;

// Force linker inclusion of wafer-block-http-listener so its
// register_static_block! entry lands in STATIC_BLOCK_REGISTRATIONS.
use wafer_block_http_listener as _;

/// Register the `wafer-run/http-listener` block on `wafer` and configure
/// it to bind `listen_addr` and dispatch through `flow_id`. Must be called
/// before `wafer.start()`.
///
/// `flow_id` is the flow the listener hands requests to (e.g. `"site-main"`
/// for solobase-server, but downstream consumers of this library pick their
/// own flow name).
pub fn register_http_listener(wafer: &mut Wafer, listen_addr: &str, flow_id: &str) {
    // wafer-run/http-listener self-registers via register_static_block! in
    // wafer-block-http-listener. The `use wafer_block_http_listener as _`
    // above ensures the linker includes its .o file. We only need to set
    // the block config here.
    wafer.add_block_config(
        "wafer-run/http-listener",
        serde_json::json!({ "flow": flow_id, "listen": listen_addr }),
    );
}

/// Await a graceful-shutdown signal (ctrl-c or SIGTERM on Unix), then call
/// `wafer.shutdown().await`. Returns after the shutdown completes.
pub async fn serve_until_shutdown(wafer: &Arc<Wafer>) {
    shutdown_signal().await;
    wafer.shutdown().await;
}

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
