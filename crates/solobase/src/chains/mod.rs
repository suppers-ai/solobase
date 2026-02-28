//! Chain definitions for Solobase.
//!
//! The wafer-core base chains (http-infra, auth-pipe, admin-pipe) are
//! registered separately by `wafer_core::chains::register_chains`.
//! This module registers the feature-level chains that compose those
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
mod system;
mod userportal;
mod web;

use wafer_run::services::config::ConfigService;
use wafer_run::Wafer;

/// Register all solobase feature chains with the runtime.
pub fn register_all_chains(w: &mut Wafer) {
    // Base pipeline aliases
    register_chain(w, protected::JSON);

    // Feature chains
    register_chain(w, auth::JSON);
    register_chain(w, system::JSON);
    register_chain(w, admin::JSON);
    register_chain(w, monitoring::JSON);
    register_chain(w, settings::JSON);
    register_chain(w, files::JSON);
    register_chain(w, legalpages::JSON);
    register_chain(w, products::JSON);
    register_chain(w, userportal::JSON);
    register_chain(w, profile::JSON);
    register_chain(w, web::JSON);
}

/// Register only chains whose corresponding feature is enabled in config.
///
/// The `protected` chain and `settings` chain are always registered as they
/// provide base infrastructure. Feature chains are gated by `features.<name>`.
pub fn register_selected_chains(w: &mut Wafer, config: &dyn ConfigService) {
    let enabled = |name: &str| -> bool {
        config
            .get(&format!("features.{}", name))
            .map(|v| v != "false")
            .unwrap_or(true) // default: enabled
    };

    // Base pipeline aliases (always registered)
    register_chain(w, protected::JSON);

    // Feature chains (conditionally registered)
    if enabled("auth") {
        register_chain(w, auth::JSON);
    }
    if enabled("system") {
        register_chain(w, system::JSON);
    }
    if enabled("admin") {
        register_chain(w, admin::JSON);
        register_chain(w, settings::JSON);
    }
    if enabled("monitoring") {
        register_chain(w, monitoring::JSON);
    }
    if enabled("files") {
        register_chain(w, files::JSON);
    }
    if enabled("legalpages") {
        register_chain(w, legalpages::JSON);
    }
    if enabled("products") {
        register_chain(w, products::JSON);
    }
    if enabled("userportal") {
        register_chain(w, userportal::JSON);
    }
    if enabled("profile") {
        register_chain(w, profile::JSON);
    }
    if enabled("web") {
        register_chain(w, web::JSON);
    }
}

fn register_chain(w: &mut Wafer, json: &str) {
    let def: wafer_run::ChainDef = serde_json::from_str(json)
        .unwrap_or_else(|e| panic!("invalid chain JSON: {e}\n---\n{json}"));
    w.add_chain_def(&def);
}
