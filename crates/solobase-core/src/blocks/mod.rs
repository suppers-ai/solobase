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
pub mod llm_backend;
pub mod local_llm;
pub mod messages;
pub mod network;
pub mod products;
pub mod projects;
pub mod provider_llm;
pub mod rate_limit;
pub mod router;
pub mod storage;
pub mod system;
pub mod userportal;
pub mod vector;

use std::{collections::HashMap, sync::Arc};

use wafer_run::block::Block;

use crate::routing::BlockId;

/// Mapping from short block name to BlockId for registration.
///
/// The `Fastembed` and `Vector` entries are only present when the
/// `native-embedding` feature is enabled. Fastembed pulls in ONNX runtime
/// via `wafer-block-fastembed`, and Vector declares
/// `requires=["wafer-run/vector"]` which can only be satisfied by the
/// feature-gated runtime registration in `solobase::builder`. Registering
/// either without the feature would fail dependency resolution at startup.
fn solobase_blocks() -> Vec<(&'static str, BlockId)> {
    #[cfg_attr(
        not(any(feature = "native-embedding", feature = "llm")),
        allow(unused_mut)
    )]
    let mut v = vec![
        ("system", BlockId::System),
        ("userportal", BlockId::UserPortal),
        ("legalpages", BlockId::LegalPages),
        ("auth", BlockId::Auth),
        ("admin", BlockId::Admin),
        ("files", BlockId::Files),
        ("products", BlockId::Products),
        ("projects", BlockId::Projects),
        ("messages", BlockId::Messages),
        ("provider-llm", BlockId::ProviderLlm),
        ("local-llm", BlockId::LocalLlm),
    ];
    #[cfg(feature = "llm")]
    {
        v.push(("llm", BlockId::Llm));
    }
    #[cfg(feature = "native-embedding")]
    {
        v.push(("vector", BlockId::Vector));
        v.push(("fastembed", BlockId::Fastembed));
    }
    v
}

/// Create a block instance for a given BlockId.
///
/// Returns `None` only for `BlockId::Inspector`, which is served by the
/// runtime's built-in inspector block rather than one of ours. A missing
/// feature gate (`Fastembed` or `Vector` without `native-embedding`) is a
/// caller bug — those variants can't be constructed unless the feature is
/// on, because `solobase_blocks()` is the only place that produces them.
fn make_block(
    id: BlockId,
    #[cfg(feature = "llm")] provider_llm_svc: &Arc<llm::providers::ProviderLlmService>,
) -> Arc<dyn Block> {
    match id {
        BlockId::System => Arc::new(system::SystemBlock),
        BlockId::UserPortal => Arc::new(userportal::UserPortalBlock),
        BlockId::LegalPages => Arc::new(legalpages::LegalPagesBlock),
        BlockId::Auth => Arc::new(auth::AuthBlock::new()),
        BlockId::Admin => Arc::new(admin::AdminBlock),
        BlockId::Files => Arc::new(files::FilesBlock::new()),
        BlockId::Products => Arc::new(products::ProductsBlock::new()),
        BlockId::Projects => Arc::new(projects::ProjectsBlock::new()),
        BlockId::Messages => Arc::new(messages::MessagesBlock),
        #[cfg(feature = "llm")]
        BlockId::Llm => Arc::new(llm::LlmBlock::new(provider_llm_svc.clone())),
        #[cfg(not(feature = "llm"))]
        BlockId::Llm => unreachable!(
            "BlockId::Llm requires the `llm` feature \
             — solobase_blocks() only emits it when that feature is on"
        ),
        BlockId::ProviderLlm => Arc::new(provider_llm::ProviderLlmBlock),
        BlockId::LocalLlm => Arc::new(local_llm::LocalLlmBlock),
        #[cfg(feature = "native-embedding")]
        BlockId::Vector => Arc::new(vector::VectorBlock),
        #[cfg(not(feature = "native-embedding"))]
        BlockId::Vector => unreachable!(
            "BlockId::Vector requires the `native-embedding` feature \
             — solobase_blocks() only emits it when that feature is on"
        ),
        #[cfg(feature = "native-embedding")]
        BlockId::Fastembed => Arc::new(fastembed::FastembedBlock::new()),
        #[cfg(not(feature = "native-embedding"))]
        BlockId::Fastembed => unreachable!(
            "BlockId::Fastembed requires the `native-embedding` feature \
             — solobase_blocks() only emits it when that feature is on"
        ),
        BlockId::Inspector => unreachable!("inspector dispatched via ctx.call_block"),
    }
}

/// Create shared block instances filtered by a predicate.
///
/// Returns a map of BlockId → Arc<dyn Block> for enabled features.
/// The same Arc instances should be registered with the WAFER runtime
/// (for lifecycle hooks) and passed to the `NativeBlockFactory` (for
/// request dispatch), ensuring state is shared.
pub fn create_blocks(
    filter: impl Fn(&str) -> bool,
    #[cfg(feature = "llm")] provider_llm_svc: &Arc<llm::providers::ProviderLlmService>,
) -> HashMap<BlockId, Arc<dyn Block>> {
    let mut map = HashMap::new();
    for (name, id) in solobase_blocks() {
        if filter(name) {
            #[cfg(feature = "llm")]
            map.insert(id, make_block(id, provider_llm_svc));
            #[cfg(not(feature = "llm"))]
            map.insert(id, make_block(id));
        }
    }
    map
}

/// Register pre-created block instances with the WAFER runtime.
///
/// This registers the blocks for lifecycle hooks (Init, Shutdown) and
/// for `ctx.call_block("suppers-ai/...", ...)` calls.
pub fn register_shared_blocks(
    w: &mut wafer_run::Wafer,
    blocks: &HashMap<BlockId, Arc<dyn Block>>,
) -> Result<(), wafer_run::RuntimeError> {
    for (&id, block) in blocks {
        let name = block_id_to_name(id);
        w.register_block(format!("suppers-ai/{name}"), block.clone())?;
    }
    Ok(())
}

fn block_id_to_name(id: BlockId) -> &'static str {
    match id {
        BlockId::System => "system",
        BlockId::UserPortal => "userportal",
        BlockId::LegalPages => "legalpages",
        BlockId::Auth => "auth",
        BlockId::Admin => "admin",
        BlockId::Files => "files",
        BlockId::Products => "products",
        BlockId::Projects => "projects",
        BlockId::Messages => "messages",
        BlockId::Llm => "llm",
        BlockId::ProviderLlm => "provider-llm",
        BlockId::LocalLlm => "local-llm",
        BlockId::Vector => "vector",
        BlockId::Fastembed => "fastembed",
        BlockId::Inspector => "inspector",
    }
}

/// Return `BlockInfo` for all solobase feature blocks (plus the email service block).
///
/// Creates temporary block instances to call `info()`. This is cheap —
/// block structs hold no resources until `lifecycle(Init)` is called.
/// Used by `collect_all_config_vars()` to discover declared config variables
/// before block registration.
pub fn all_block_infos() -> Vec<wafer_run::block::BlockInfo> {
    // `info()` is declarative — no runtime state needed. A throwaway
    // `ProviderLlmService` is fine here; the real one is constructed by
    // `SolobaseBuilder::build()`.
    #[cfg(feature = "llm")]
    let throwaway_llm_svc = Arc::new(llm::providers::ProviderLlmService::new());
    let mut infos = Vec::new();
    for (_name, id) in solobase_blocks() {
        #[cfg(feature = "llm")]
        infos.push(make_block(id, &throwaway_llm_svc).info());
        #[cfg(not(feature = "llm"))]
        infos.push(make_block(id).info());
    }
    // Email block is always registered (not feature-gated)
    infos.push(Arc::new(email::EmailBlock).info());
    infos
}
