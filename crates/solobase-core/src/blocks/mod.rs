pub mod admin;
pub mod auth;
pub mod auth_ui;
pub mod crud;
pub mod email;
pub mod errors;
// `native-embedding` always implies `block-fastembed` (see Cargo.toml), so
// the native build still gets this module. wafer-site / wasm32 builds with
// neither feature drop the ONNX-runtime dep entirely.
#[cfg(feature = "block-fastembed")]
pub mod fastembed;
#[cfg(feature = "block-files")]
pub mod files;
pub mod helpers;
#[cfg(feature = "block-legalpages")]
pub mod legalpages;
// The LLM feature block compiles on every target that enables `block-llm`,
// including wasm32. `LlmBlock` holds `Arc<dyn ProviderAdmin>` (the
// provider-management seam), not the concrete reqwest/tokio
// `ProviderLlmService`, so the block module no longer drags the native HTTP
// stack. The `llm` feature is now just "native provider backend": it gates
// `providers::ProviderLlmService` (reqwest/stream + tokio) and is implied by
// nothing the block module itself needs. Browser/CF builds enable `block-llm`
// without `llm` and supply their own backend via
// `SolobaseBuilder::llm_service` (e.g. `BrowserLlmService` in solobase-web);
// the block holds a `NoopProviderAdmin` there.
#[cfg(feature = "block-llm")]
pub mod llm;
#[cfg(feature = "block-messages")]
pub mod messages;
pub mod network;
#[cfg(feature = "block-products")]
pub mod products;
pub mod rate_limit;
pub mod router;
pub mod storage;
pub mod system;
#[cfg(target_arch = "wasm32")]
pub mod transformers_embed;
#[cfg(feature = "block-userportal")]
pub mod userportal;
#[cfg(feature = "block-vector")]
pub mod vector;

/// Return `BlockInfo` for every solobase block.
///
/// This is the single canonical source of truth for both native and wasm32.
/// Previously, native used `linkme`/`STATIC_BLOCK_REGISTRATIONS` iteration
/// and wasm32 had a separate manual list — see audit finding #13.
///
/// The two lists had diverged: the native linkme sweep also picked up
/// `wafer-run/*` framework blocks (cors, inspector, etc.) that were never
/// relevant to `collect_all_config_vars`, and the wasm32 list included the
/// framework `AuthBlock` whose config vars are declared via
/// `shared_config_vars()` → `auth_config_vars()` rather than
/// `BlockInfo::config_keys`, making it redundant there too. This function
/// enumerates only the solobase feature blocks, consistently on both targets.
///
/// Used by `collect_all_config_vars()` to discover declared config
/// variables before block registration runs.
pub fn all_block_infos() -> Vec<wafer_run::BlockInfo> {
    use wafer_run::Block as _;

    // `unused_mut` fires when every optional feature is off and no later
    // `.push(...)` exists to mutate the vec.
    #[allow(unused_mut)]
    let mut infos: Vec<wafer_run::BlockInfo> = vec![
        admin::AdminBlock::new().info(),
        auth_ui::AuthUiBlock::default().info(),
        email::EmailBlock::new().info(),
        system::SystemBlock::new().info(),
    ];

    #[cfg(feature = "block-files")]
    infos.push(files::FilesBlock::new().info());
    #[cfg(feature = "block-legalpages")]
    infos.push(legalpages::LegalPagesBlock::new().info());
    #[cfg(feature = "block-messages")]
    infos.push(messages::MessagesBlock::new().info());
    #[cfg(feature = "block-products")]
    infos.push(products::ProductsBlock::new().info());
    #[cfg(feature = "block-userportal")]
    infos.push(userportal::UserPortalBlock::new().info());
    #[cfg(feature = "block-vector")]
    infos.push(vector::VectorBlock::new().info());

    // fastembed is native-only: it requires ONNX Runtime which is not
    // available on wasm32.
    #[cfg(feature = "block-fastembed")]
    infos.push(fastembed::FastembedBlock::new().info());

    // LlmBlock cannot self-register because its constructor takes
    // Arc<dyn ProviderAdmin>. A no-op provider-admin handle is enough here
    // since info() is declarative and never drives the provider surface.
    #[cfg(feature = "block-llm")]
    {
        use std::sync::Arc;
        let provider_admin = Arc::new(llm::provider_admin::NoopProviderAdmin);
        infos.push(llm::LlmBlock::new(provider_admin).info());
    }

    infos
}

/// Register the LLM feature block with the WAFER runtime.
///
/// LlmBlock cannot self-register via `register_static_block!` because its
/// constructor takes `Arc<dyn ProviderAdmin>`. Call this after the LLM
/// service router is registered in `SolobaseBuilder::build()`.
///
/// `provider_admin` is the provider-management seam: the concrete
/// `ProviderLlmService` on native (`feature = "llm"`) or a `NoopProviderAdmin`
/// on wasm32 (where the browser configures providers inside its own
/// `BrowserLlmService`).
#[cfg(feature = "block-llm")]
pub fn register_llm(
    w: &mut wafer_run::Wafer,
    provider_admin: std::sync::Arc<dyn llm::provider_admin::ProviderAdmin>,
) -> Result<(), wafer_run::RuntimeError> {
    w.register_block(
        "suppers-ai/llm".to_string(),
        std::sync::Arc::new(llm::LlmBlock::new(provider_admin)),
    )
}

/// Register the framework `suppers-ai/auth` block — wafer-core's `AuthBlock`
/// wrapping solobase's `AuthServiceImpl`.
///
/// Cannot self-register via `register_static_block!` because the framework
/// `AuthBlock::new` takes `Arc<dyn AuthService>`. Call this from
/// `SolobaseBuilder::build` (native) or `register_all_static_blocks` (wasm32)
/// to install both the block and the service.
///
/// The `AuthServiceImpl`'s context cell starts empty here; it gets populated
/// when the runtime fires the framework AuthBlock's `lifecycle(Init)` event,
/// which calls into `AuthService::init` and stashes `ctx.clone_arc()` for
/// later `require_*` dispatches.
pub fn register_auth(wafer: &mut wafer_run::Wafer) -> Result<(), wafer_run::RuntimeError> {
    use std::sync::Arc;
    let state = auth::service::BlockState::new();
    let svc = Arc::new(auth::service::AuthServiceImpl::new(state));
    wafer_core::service_blocks::auth::register_with(wafer, svc)
}

/// Register every solobase feature block on wasm32 builds.
///
/// On native, each block self-registers via `register_static_block!` (gated
/// `cfg(not(target_arch = "wasm32"))` because linkme's distributed_slice
/// only emits on ELF/Mach-O/PE — see `wafer_run::builder::WaferBuilder::build`).
/// On wasm32 that path is a no-op, so the runtime starts with zero
/// `suppers-ai/*` blocks and the SolobaseRouter dispatches into a void —
/// every feature route returns `block 'suppers-ai/<name>' not found`.
///
/// This helper mirrors the linkme manifest so wasm builds get the same
/// block set. Keep this list in sync with the `register_static_block!`
/// invocations across `crate::blocks::*` and with the `all_block_infos`
/// wasm32 fallback above.
///
/// Excludes `suppers-ai/fastembed` (native-only, requires
/// `feature = "native-embedding"`). `suppers-ai/llm` IS registered here when
/// `block-llm` is on: the block now holds `Arc<dyn ProviderAdmin>`, so the
/// wasm32 build installs it with a `NoopProviderAdmin` (the browser's
/// `BrowserLlmService` on the shared router serves chat; provider CRUD /
/// discovery are admin-only and degrade to no-ops).
#[cfg(target_arch = "wasm32")]
pub fn register_all_static_blocks(
    wafer: &mut wafer_run::Wafer,
) -> Result<(), wafer_run::RuntimeError> {
    use std::sync::Arc;

    wafer.register_block("suppers-ai/admin", Arc::new(admin::AdminBlock::new()))?;
    // Framework `suppers-ai/auth` is registered unconditionally by
    // `SolobaseBuilder::build` (after this fn returns) — don't duplicate
    // here, the second register_block would fail with "block already
    // registered" and abort the wasm boot.
    wafer.register_block(
        "suppers-ai/auth-ui",
        Arc::new(auth_ui::AuthUiBlock::default()),
    )?;
    wafer.register_block("suppers-ai/email", Arc::new(email::EmailBlock::new()))?;
    wafer.register_block("suppers-ai/system", Arc::new(system::SystemBlock::new()))?;

    #[cfg(feature = "block-files")]
    wafer.register_block("suppers-ai/files", Arc::new(files::FilesBlock::new()))?;
    #[cfg(feature = "block-legalpages")]
    wafer.register_block(
        "suppers-ai/legalpages",
        Arc::new(legalpages::LegalPagesBlock::new()),
    )?;
    #[cfg(feature = "block-messages")]
    wafer.register_block(
        "suppers-ai/messages",
        Arc::new(messages::MessagesBlock::new()),
    )?;
    #[cfg(feature = "block-products")]
    wafer.register_block(
        "suppers-ai/products",
        Arc::new(products::ProductsBlock::new()),
    )?;
    #[cfg(feature = "block-userportal")]
    wafer.register_block(
        "suppers-ai/userportal",
        Arc::new(userportal::UserPortalBlock::new()),
    )?;
    #[cfg(feature = "block-vector")]
    wafer.register_block("suppers-ai/vector", Arc::new(vector::VectorBlock::new()))?;

    // The LLM block runs on wasm32 against a browser-supplied `LlmService`
    // (registered on the router via `SolobaseBuilder::llm_service`). Provider
    // CRUD / discovery have no browser surface, so a `NoopProviderAdmin`
    // stands in for the native HTTP `ProviderLlmService`.
    #[cfg(feature = "block-llm")]
    register_llm(wafer, Arc::new(llm::provider_admin::NoopProviderAdmin))?;

    Ok(())
}
