pub mod errors;
pub mod rate_limit;
pub mod helpers;
pub mod admin;
pub mod auth;
pub mod deployments;
pub mod files;
pub mod legalpages;
pub mod products;
pub mod profile;
pub mod system;
pub mod userportal;

/// Register all feature blocks unconditionally.
#[cfg(not(target_arch = "wasm32"))]
pub fn register_all(w: &mut wafer_run::Wafer) {
    use std::sync::Arc;
    w.register_block("@solobase/profile", Arc::new(profile::ProfileBlock));
    w.register_block("@solobase/system", Arc::new(system::SystemBlock));
    w.register_block("@solobase/userportal", Arc::new(userportal::UserPortalBlock));
    w.register_block("@solobase/legalpages", Arc::new(legalpages::LegalPagesBlock));
    w.register_block("@solobase/auth", Arc::new(auth::AuthBlock::new()));
    w.register_block("@solobase/admin", Arc::new(admin::AdminBlock));
    w.register_block("@solobase/files", Arc::new(files::FilesBlock::new()));
    w.register_block("@solobase/products", Arc::new(products::ProductsBlock::new()));
    w.register_block("@solobase/deployments", Arc::new(deployments::DeploymentsBlock::new()));
}

/// Register only the feature blocks enabled via env vars.
///
/// Reads `FEATURE_<NAME>` env vars. If a var is missing or not "false",
/// the block is registered. If explicitly "false", it is skipped.
#[cfg(not(target_arch = "wasm32"))]
pub fn register_selected(w: &mut wafer_run::Wafer) {
    use std::sync::Arc;
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
    if enabled("legalpages") {
        w.register_block("@solobase/legalpages", Arc::new(legalpages::LegalPagesBlock));
    }
    if enabled("auth") {
        w.register_block("@solobase/auth", Arc::new(auth::AuthBlock::new()));
    }
    if enabled("admin") {
        w.register_block("@solobase/admin", Arc::new(admin::AdminBlock));
    }
    if enabled("files") {
        w.register_block("@solobase/files", Arc::new(files::FilesBlock::new()));
    }
    if enabled("products") {
        w.register_block("@solobase/products", Arc::new(products::ProductsBlock::new()));
    }
    if enabled("deployments") {
        w.register_block("@solobase/deployments", Arc::new(deployments::DeploymentsBlock::new()));
    }
}
