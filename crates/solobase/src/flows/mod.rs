//! Flow definitions for Solobase.
//!
//! All API routing is handled by the `@solobase/router` block, which delegates
//! to `solobase-core`'s shared pipeline. The only flow needed is `site-main`,
//! which dispatches API paths to the router and serves the SPA for everything
//! else. The wafer-core base flows (@wafer/infra) provide middleware.

mod site_main;

use wafer_run::Wafer;

/// Register the site-main flow (used with @solobase/router).
pub fn register_site_main(w: &mut Wafer) {
    let def: wafer_run::FlowDef = serde_json::from_str(site_main::JSON)
        .unwrap_or_else(|e| panic!("invalid flow JSON: {e}\n---\n{}", site_main::JSON));
    w.add_flow_def(&def);
}
