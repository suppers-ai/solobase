pub mod admin;
pub mod auth;
pub mod crud;
pub mod email;
pub mod errors;
#[cfg(feature = "native-embedding")]
pub mod fastembed;
#[cfg(target_arch = "wasm32")]
pub mod transformers_embed;
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
