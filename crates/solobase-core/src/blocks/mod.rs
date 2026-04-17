pub mod admin;
pub mod auth;
pub mod crud;
pub mod email;
pub mod errors;
pub mod files;
pub mod helpers;
pub mod legalpages;
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
const SOLOBASE_BLOCKS: &[(&str, BlockId)] = &[
    ("system", BlockId::System),
    ("userportal", BlockId::UserPortal),
    ("legalpages", BlockId::LegalPages),
    ("auth", BlockId::Auth),
    ("admin", BlockId::Admin),
    ("files", BlockId::Files),
    ("products", BlockId::Products),
    ("projects", BlockId::Projects),
    ("messages", BlockId::Messages),
    ("llm", BlockId::Llm),
    ("provider-llm", BlockId::ProviderLlm),
    ("local-llm", BlockId::LocalLlm),
    ("vector", BlockId::Vector),
];

/// Create a block instance for a given BlockId.
fn make_block(id: BlockId) -> Arc<dyn Block> {
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
        BlockId::Llm => Arc::new(llm::LlmBlock),
        BlockId::ProviderLlm => Arc::new(provider_llm::ProviderLlmBlock),
        BlockId::LocalLlm => Arc::new(local_llm::LocalLlmBlock),
        BlockId::Vector => Arc::new(vector::VectorBlock),
        BlockId::Inspector => unreachable!("inspector dispatched via ctx.call_block"),
    }
}

/// Create shared block instances filtered by a predicate.
///
/// Returns a map of BlockId → Arc<dyn Block> for enabled features.
/// The same Arc instances should be registered with the WAFER runtime
/// (for lifecycle hooks) and passed to the `NativeBlockFactory` (for
/// request dispatch), ensuring state is shared.
pub fn create_blocks(filter: impl Fn(&str) -> bool) -> HashMap<BlockId, Arc<dyn Block>> {
    let mut map = HashMap::new();
    for &(name, id) in SOLOBASE_BLOCKS {
        if filter(name) {
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
    let mut infos = Vec::new();
    for &(_name, id) in SOLOBASE_BLOCKS {
        infos.push(make_block(id).info());
    }
    // Email block is always registered (not feature-gated)
    infos.push(Arc::new(email::EmailBlock).info());
    infos
}
