//! Flow definitions for Solobase.
//!
//! The wafer-core base flows (@wafer/infra, @wafer/auth-pipe, @wafer/admin-pipe)
//! are registered separately by `wafer_core::flows::register_flows`.
//! This module registers the feature-level flows that compose those
//! base pipelines with solobase's native Rust feature blocks.

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

/// Register flows, optionally gated by a predicate.
///
/// Flows with `gate = None` are always registered. Flows with a gate name
/// are only registered if `filter(gate_name)` returns `true`.
pub fn register_flows(w: &mut Wafer, filter: impl Fn(&str) -> bool) {
    for &(json, gate) in FEATURE_FLOWS {
        if gate.map_or(true, |name| filter(name)) {
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
