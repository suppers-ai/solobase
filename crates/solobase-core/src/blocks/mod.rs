pub mod admin;
pub mod auth;
pub mod crud;
pub mod email;
pub mod errors;
#[cfg(feature = "native-embedding")]
pub mod fastembed;
pub mod files;
pub mod helpers;
pub mod legalpages;
#[cfg(feature = "llm")]
pub mod llm;
pub mod messages;
pub mod network;
pub mod products;
pub mod rate_limit;
pub mod router;
pub mod storage;
pub mod system;
#[cfg(target_arch = "wasm32")]
pub mod transformers_embed;
pub mod userportal;
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

    #[cfg_attr(not(feature = "llm"), allow(unused_mut))]
    let mut infos: Vec<wafer_run::block::BlockInfo> = vec![
        admin::AdminBlock::new().info(),
        auth::AuthBlock::new().info(),
        email::EmailBlock::new().info(),
        files::FilesBlock::new().info(),
        legalpages::LegalPagesBlock::new().info(),
        messages::MessagesBlock::new().info(),
        products::ProductsBlock::new().info(),
        system::SystemBlock::new().info(),
        userportal::UserPortalBlock::new().info(),
        vector::VectorBlock::new().info(),
    ];

    #[cfg(feature = "native-embedding")]
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
    wafer.register_block("suppers-ai/auth", Arc::new(auth::AuthBlock::new()))?;
    wafer.register_block("suppers-ai/email", Arc::new(email::EmailBlock::new()))?;
    wafer.register_block("suppers-ai/files", Arc::new(files::FilesBlock::new()))?;
    wafer.register_block(
        "suppers-ai/legalpages",
        Arc::new(legalpages::LegalPagesBlock::new()),
    )?;
    wafer.register_block(
        "suppers-ai/messages",
        Arc::new(messages::MessagesBlock::new()),
    )?;
    wafer.register_block(
        "suppers-ai/products",
        Arc::new(products::ProductsBlock::new()),
    )?;
    wafer.register_block("suppers-ai/system", Arc::new(system::SystemBlock::new()))?;
    wafer.register_block(
        "suppers-ai/userportal",
        Arc::new(userportal::UserPortalBlock::new()),
    )?;
    wafer.register_block("suppers-ai/vector", Arc::new(vector::VectorBlock::new()))?;

    Ok(())
}
