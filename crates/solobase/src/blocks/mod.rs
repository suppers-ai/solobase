use std::sync::Arc;
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

/// Register all feature blocks unconditionally.
pub fn register_all(w: &mut Wafer) {
    w.register_block("@solobase/profile", Arc::new(profile::ProfileBlock));
    w.register_block("@solobase/system", Arc::new(system::SystemBlock));
    w.register_block("@solobase/userportal", Arc::new(userportal::UserPortalBlock));
    w.register_block("@solobase/monitoring", Arc::new(monitoring::MonitoringBlock::new()));
    w.register_block("@solobase/legalpages", Arc::new(legalpages::LegalPagesBlock));
    w.register_block("@solobase/auth", Arc::new(auth::AuthBlock));
    w.register_block("@solobase/admin", Arc::new(admin::AdminBlock));
    w.register_block("@solobase/files", Arc::new(files::FilesBlock));
    w.register_block("@solobase/products", Arc::new(products::ProductsBlock));
}

/// Register only the feature blocks enabled via env vars.
///
/// Reads `FEATURE_<NAME>` env vars. If a var is missing or not "false",
/// the block is registered. If explicitly "false", it is skipped.
pub fn register_selected(w: &mut Wafer) {
    let enabled = |name: &str| -> bool {
        std::env::var(format!("FEATURE_{}", name.to_uppercase()))
            .map(|v| v != "false")
            .unwrap_or(true) // default: enabled
    };

    if enabled("profile") {
        w.register_block("@solobase/profile", Arc::new(profile::ProfileBlock));
    }
    if enabled("system") {
        w.register_block("@solobase/system", Arc::new(system::SystemBlock));
    }
    if enabled("userportal") {
        w.register_block("@solobase/userportal", Arc::new(userportal::UserPortalBlock));
    }
    if enabled("monitoring") {
        w.register_block("@solobase/monitoring", Arc::new(monitoring::MonitoringBlock::new()));
    }
    if enabled("legalpages") {
        w.register_block("@solobase/legalpages", Arc::new(legalpages::LegalPagesBlock));
    }
    if enabled("auth") {
        w.register_block("@solobase/auth", Arc::new(auth::AuthBlock));
    }
    if enabled("admin") {
        w.register_block("@solobase/admin", Arc::new(admin::AdminBlock));
    }
    if enabled("files") {
        w.register_block("@solobase/files", Arc::new(files::FilesBlock));
    }
    if enabled("products") {
        w.register_block("@solobase/products", Arc::new(products::ProductsBlock));
    }
}
