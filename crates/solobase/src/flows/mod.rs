//! Flow definitions for Solobase.
//!
//! With the `@solobase/router` block, all API routing is handled by
//! `solobase-core`'s shared pipeline. The only flow needed is `site-main`,
//! which dispatches API paths to the router and serves the SPA for everything
//! else. The wafer-core base flows (@wafer/infra) provide middleware.
//!
//! Legacy per-feature flows (auth, admin, files, etc.) are kept for backwards
//! compatibility with the `blocks.json` configuration mode.

mod admin;
mod auth;
mod deployments;
mod files;
mod legalpages;
mod products;
mod profile;
mod protected;
mod settings;
mod site_main;
mod system;
mod userportal;

use wafer_run::Wafer;

/// Register only the site-main flow (used with @solobase/router).
///
/// This is the preferred mode when using app.json configuration. All API
/// routing goes through the shared solobase-core pipeline.
pub fn register_site_main(w: &mut Wafer) {
    register_flow(w, site_main::JSON);
}

// ---------------------------------------------------------------------------
// Legacy flow registration (blocks.json mode — kept for backwards compat)
// ---------------------------------------------------------------------------

/// Feature flows with their gate name (None = always registered).
const FEATURE_FLOWS: &[(&str, Option<&str>)] = &[
    (protected::JSON, None),        // base pipeline alias — always
    (auth::JSON, Some("auth")),
    (system::JSON, Some("system")),
    (admin::JSON, Some("admin")),
    (settings::JSON, Some("admin")), // settings gated with admin
    (files::JSON, Some("files")),
    (legalpages::JSON, Some("legalpages")),
    (products::JSON, Some("products")),
    (deployments::JSON, Some("deployments")),
    (userportal::JSON, Some("userportal")),
    (profile::JSON, Some("profile")),
    (site_main::JSON, None),        // top-level dispatch — always
];

/// Register flows, optionally gated by a predicate (legacy blocks.json mode).
pub fn register_flows(w: &mut Wafer, filter: impl Fn(&str) -> bool) {
    for &(json, gate) in FEATURE_FLOWS {
        if gate.is_none_or(&filter) {
            register_flow(w, json);
        }
    }
}

/// Register all solobase feature flows with the runtime.
pub fn register_all_flows(w: &mut Wafer) {
    register_flows(w, |_| true);
}

/// Register only flows whose corresponding feature is enabled via env vars.
pub fn register_selected_flows(w: &mut Wafer) {
    register_flows(w, |name| {
        std::env::var(format!("FEATURE_{}", name.to_uppercase()))
            .map(|v| v != "false")
            .unwrap_or(true)
    });
}

fn register_flow(w: &mut Wafer, json: &str) {
    let def: wafer_run::FlowDef = serde_json::from_str(json)
        .unwrap_or_else(|e| panic!("invalid flow JSON: {e}\n---\n{json}"));
    w.add_flow_def(&def);
}
