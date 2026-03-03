use std::sync::Arc;
use wafer_run::services::config::ConfigService;
use wafer_run::Wafer;

pub(crate) mod helpers;
mod admin;
mod auth;
mod files;
mod legalpages;
mod monitoring;
mod products;
mod profile;
mod system;
mod userportal;
mod web;

/// Register all feature blocks unconditionally.
pub fn register_all(w: &mut Wafer) {
    w.register_block("profile-feature", Arc::new(profile::ProfileBlock));
    w.register_block("system-feature", Arc::new(system::SystemBlock));
    w.register_block(
        "userportal-feature",
        Arc::new(userportal::UserPortalBlock),
    );
    w.register_block("web-feature", Arc::new(web::WebBlock::new()));
    w.register_block(
        "monitoring-feature",
        Arc::new(monitoring::MonitoringBlock::new()),
    );
    w.register_block(
        "legalpages-feature",
        Arc::new(legalpages::LegalPagesBlock),
    );
    w.register_block("auth-feature", Arc::new(auth::AuthBlock));
    w.register_block("admin-feature", Arc::new(admin::AdminBlock));
    w.register_block("files-feature", Arc::new(files::FilesBlock));
    w.register_block("products-feature", Arc::new(products::ProductsBlock));
}

/// Register only the feature blocks enabled in config.
///
/// Reads `features.<name>` keys from config. If a key is missing or "true",
/// the block is registered. If explicitly "false", it is skipped.
pub fn register_selected(w: &mut Wafer, config: &dyn ConfigService) {
    let enabled = |name: &str| -> bool {
        config
            .get(&format!("features.{}", name))
            .map(|v| v != "false")
            .unwrap_or(true) // default: enabled
    };

    if enabled("profile") {
        w.register_block("profile-feature", Arc::new(profile::ProfileBlock));
    }
    if enabled("system") {
        w.register_block("system-feature", Arc::new(system::SystemBlock));
    }
    if enabled("userportal") {
        w.register_block(
            "userportal-feature",
            Arc::new(userportal::UserPortalBlock),
        );
    }
    if enabled("web") {
        w.register_block("web-feature", Arc::new(web::WebBlock::new()));
    }
    if enabled("monitoring") {
        w.register_block(
            "monitoring-feature",
            Arc::new(monitoring::MonitoringBlock::new()),
        );
    }
    if enabled("legalpages") {
        w.register_block(
            "legalpages-feature",
            Arc::new(legalpages::LegalPagesBlock),
        );
    }
    if enabled("auth") {
        w.register_block("auth-feature", Arc::new(auth::AuthBlock));
    }
    if enabled("admin") {
        w.register_block("admin-feature", Arc::new(admin::AdminBlock));
    }
    if enabled("files") {
        w.register_block("files-feature", Arc::new(files::FilesBlock));
    }
    if enabled("products") {
        w.register_block("products-feature", Arc::new(products::ProductsBlock));
    }
}
