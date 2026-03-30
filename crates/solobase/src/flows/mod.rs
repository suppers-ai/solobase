//! Flow definitions for Solobase.
//!
//! All API routing is handled by the `suppers-ai/router` block, which delegates
//! to `solobase-core`'s shared pipeline. The only flow needed is `site-main`,
//! which dispatches API paths to the router and serves the SPA for everything
//! else. The wafer-core base flows (wafer-run/infra) provide middleware.

pub mod site_main;

use wafer_run::Wafer;

/// Register the site-main flow (used with suppers-ai/router).
///
/// # Panics
/// This function is called during startup. If the embedded flow JSON is invalid
/// (which would be a build-time bug), it returns an error rather than panicking.
pub fn register_site_main(w: &mut Wafer) -> Result<(), String> {
    // Inject default routes into the router block config
    w.add_block_config(
        "wafer-run/router",
        serde_json::json!({ "routes": site_main::default_routes() }),
    );

    // Configure the web block to serve from the "site" storage bucket as an SPA
    w.add_block_config(
        "wafer-run/web",
        serde_json::json!({ "web_root": "site", "web_spa": "true", "web_index": "index.html" }),
    );

    w.add_flow_json(site_main::JSON)
        .map_err(|e| format!("invalid flow JSON: {e}"))
}
