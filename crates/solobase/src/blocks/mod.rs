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

/// Register blocks, optionally gated by a predicate.
///
/// If `filter` returns `true` for a block name, that block is registered.
/// Pass `|_| true` to register all blocks unconditionally.
#[cfg(not(target_arch = "wasm32"))]
pub fn register_blocks(w: &mut wafer_run::Wafer, filter: impl Fn(&str) -> bool) {
    use std::sync::Arc;

    let blocks: Vec<(&str, Arc<dyn wafer_run::block::Block>)> = vec![
        ("profile", Arc::new(profile::ProfileBlock)),
        ("system", Arc::new(system::SystemBlock)),
        ("userportal", Arc::new(userportal::UserPortalBlock)),
        ("legalpages", Arc::new(legalpages::LegalPagesBlock)),
        ("auth", Arc::new(auth::AuthBlock::new())),
        ("admin", Arc::new(admin::AdminBlock)),
        ("files", Arc::new(files::FilesBlock::new())),
        ("products", Arc::new(products::ProductsBlock::new())),
        ("deployments", Arc::new(deployments::DeploymentsBlock::new())),
    ];

    for (name, block) in blocks {
        if filter(name) {
            w.register_block(&format!("@solobase/{name}"), block);
        }
    }
}

/// Register all blocks unconditionally.
#[cfg(not(target_arch = "wasm32"))]
pub fn register_all(w: &mut wafer_run::Wafer) {
    register_blocks(w, |_| true);
}

/// Register only blocks enabled via `FEATURE_<NAME>` env vars.
/// Missing or non-"false" values mean enabled.
#[cfg(not(target_arch = "wasm32"))]
pub fn register_selected(w: &mut wafer_run::Wafer) {
    register_blocks(w, |name| {
        std::env::var(format!("FEATURE_{}", name.to_uppercase()))
            .map(|v| v != "false")
            .unwrap_or(true)
    });
}
