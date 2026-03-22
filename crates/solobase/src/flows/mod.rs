//! Flow definitions for Solobase.
//!
//! All API routing is handled by the `suppers-ai/router` block, which delegates
//! to `solobase-core`'s shared pipeline. The only flow needed is `site-main`,
//! which dispatches API paths to the router and serves the SPA for everything
//! else. The wafer-core base flows (wafer-run/infra) provide middleware.

pub mod site_main;

use wafer_run::Wafer;

/// Register the site-main flow (used with suppers-ai/router).
pub fn register_site_main(w: &mut Wafer) {
    // Inject default routes into the router block config
    w.add_block_config(
        "wafer-run/router",
        serde_json::json!({ "routes": site_main::default_routes() }),
    );

    w.add_flow_json(site_main::JSON)
        .unwrap_or_else(|e| panic!("invalid flow JSON: {e}\n---\n{}", site_main::JSON));
}
