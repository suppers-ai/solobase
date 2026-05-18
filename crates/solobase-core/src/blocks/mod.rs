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
// `feature = "llm"` is the existing flag that pulls in the `ProviderLlmService`
// (reqwest/stream + tokio). It implies `block-llm` so turning `llm` on alone
// still registers the block. Turning `block-llm` on without `llm` is allowed
// for callers that supply their own LLM backend via `SolobaseBuilder::llm_service`
// (e.g. `BrowserLlmService` in solobase-web) — but currently the block module
// itself only compiles when `llm` is on because `LlmBlock::new` takes
// `Arc<ProviderLlmService>`. Future work could split the provider service out
// to let `block-llm` stand alone; for now they travel together.
#[cfg(feature = "llm")]
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
/// Sources:
/// - Auto-registered (zero-arg) blocks via `linkme` — same instances
///   Wafer::new() will register at runtime.
/// - LlmBlock — constructed with a throwaway ProviderLlmService since its
///   info() is declarative and doesn't depend on the real service. Can't
///   auto-register because its constructor takes Arc<ProviderLlmService>.
///
/// Used by `collect_all_config_vars()` to discover declared config
/// variables before block registration runs.
#[cfg(not(target_arch = "wasm32"))]
pub fn all_block_infos() -> Vec<wafer_run::block::BlockInfo> {
    #[cfg_attr(not(feature = "llm"), allow(unused_mut))]
    let mut infos: Vec<_> = wafer_run::STATIC_BLOCK_REGISTRATIONS
        .iter()
        .map(|reg| (reg.factory)().info())
        .collect();

    #[cfg(feature = "llm")]
    {
        use wafer_run::block::Block as _;
        let throwaway = std::sync::Arc::new(llm::providers::ProviderLlmService::new());
        infos.push(llm::LlmBlock::new(throwaway).info());
    }

    infos
}

/// wasm32 fallback: linkme is not supported on wasm32 (wafer-run gates
/// `StaticBlockRegistration` behind `cfg(not(target_arch = "wasm32"))`), so
/// enumerate blocks manually. Same content the linkme iteration would
/// produce on native, plus LlmBlock under `feature = "llm"`.
#[cfg(target_arch = "wasm32")]
pub fn all_block_infos() -> Vec<wafer_run::block::BlockInfo> {
    use wafer_run::block::Block as _;

    // `unused_mut` fires when every optional feature is off, because no later
    // `.push(...)` exists to mutate the vec.
    #[allow(unused_mut)]
    let mut infos: Vec<wafer_run::block::BlockInfo> = vec![
        admin::AdminBlock::new().info(),
        // Framework AuthBlock wrapping AuthServiceImpl — not self-registered
        // because the constructor takes `Arc<dyn AuthService>`. Mirrors the
        // `register_auth` helper below so wasm config-var collection sees
        // the same block info native gets.
        {
            use std::sync::Arc;
            let state = auth::service::BlockState::new();
            let svc = Arc::new(auth::service::AuthServiceImpl::new(state));
            wafer_core::service_blocks::auth::AuthBlock::new(svc).info()
        },
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

    #[cfg(feature = "block-fastembed")]
    infos.push(fastembed::FastembedBlock::new().info());

    #[cfg(feature = "llm")]
    {
        use std::sync::Arc;
        let throwaway = Arc::new(llm::providers::ProviderLlmService::new());
        infos.push(llm::LlmBlock::new(throwaway).info());
    }

    infos
}

/// Register the LLM feature block with the WAFER runtime.
///
/// LlmBlock cannot self-register via `register_static_block!` because its
/// constructor takes `Arc<ProviderLlmService>`. Call this after the LLM
/// service router is registered in `SolobaseBuilder::build()`.
#[cfg(feature = "llm")]
pub fn register_llm(
    w: &mut wafer_run::Wafer,
    provider_llm_svc: std::sync::Arc<llm::providers::ProviderLlmService>,
) -> Result<(), wafer_run::RuntimeError> {
    w.register_block(
        "suppers-ai/llm".to_string(),
        std::sync::Arc::new(llm::LlmBlock::new(provider_llm_svc)),
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
/// Excludes `suppers-ai/llm` (constructed in `SolobaseBuilder::build` with
/// `Arc<ProviderLlmService>`) and `suppers-ai/fastembed` (native-only,
/// requires `feature = "native-embedding"`).
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

    Ok(())
}
