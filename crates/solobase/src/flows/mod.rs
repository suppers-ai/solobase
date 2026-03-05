//! Flow definitions for Solobase.
//!
//! The wafer-core base flows (@wafer/infra, @wafer/auth-pipe, @wafer/admin-pipe)
//! are registered separately by `wafer_core::flows::register_flows`.
//! This module registers the feature-level flows that compose those
//! base pipelines with solobase's native Rust feature blocks.

mod admin;
mod auth;
mod files;
mod legalpages;
mod monitoring;
mod products;
mod profile;
mod protected;
mod settings;
mod site_main;
mod system;
mod userportal;

use wafer_run::Wafer;

/// Register all solobase feature flows with the runtime.
pub fn register_all_flows(w: &mut Wafer) {
    // Base pipeline aliases
    register_flow(w, protected::JSON);

    // Feature flows
    register_flow(w, auth::JSON);
    register_flow(w, system::JSON);
    register_flow(w, admin::JSON);
    register_flow(w, monitoring::JSON);
    register_flow(w, settings::JSON);
    register_flow(w, files::JSON);
    register_flow(w, legalpages::JSON);
    register_flow(w, products::JSON);
    register_flow(w, userportal::JSON);
    register_flow(w, profile::JSON);

    // Top-level dispatch (must be registered last — references all feature flows)
    register_flow(w, site_main::JSON);
}

/// Register only flows whose corresponding feature is enabled via env vars.
///
/// The `protected` flow, `settings` flow, and `site-main` flow are always
/// registered. Feature flows are gated by `FEATURE_<NAME>`.
pub fn register_selected_flows(w: &mut Wafer) {
    let enabled = |name: &str| -> bool {
        std::env::var(format!("FEATURE_{}", name.to_uppercase()))
            .map(|v| v != "false")
            .unwrap_or(true) // default: enabled
    };

    // Base pipeline aliases (always registered)
    register_flow(w, protected::JSON);

    // Feature flows (conditionally registered)
    if enabled("auth") {
        register_flow(w, auth::JSON);
    }
    if enabled("system") {
        register_flow(w, system::JSON);
    }
    if enabled("admin") {
        register_flow(w, admin::JSON);
        register_flow(w, settings::JSON);
    }
    if enabled("monitoring") {
        register_flow(w, monitoring::JSON);
    }
    if enabled("files") {
        register_flow(w, files::JSON);
    }
    if enabled("legalpages") {
        register_flow(w, legalpages::JSON);
    }
    if enabled("products") {
        register_flow(w, products::JSON);
    }
    if enabled("userportal") {
        register_flow(w, userportal::JSON);
    }
    if enabled("profile") {
        register_flow(w, profile::JSON);
    }

    // Top-level dispatch (always registered)
    register_flow(w, site_main::JSON);
}

fn register_flow(w: &mut Wafer, json: &str) {
    let def: wafer_run::FlowDef = serde_json::from_str(json)
        .unwrap_or_else(|e| panic!("invalid flow JSON: {e}\n---\n{json}"));
    w.add_flow_def(&def);
}
