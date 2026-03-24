pub mod errors;
pub mod rate_limit;
pub mod helpers;
pub mod admin;
pub mod auth;
pub mod deployments;
pub mod email;
pub mod files;
pub mod legalpages;
pub mod products;
pub mod profile;
pub mod router;
pub mod system;
pub mod userportal;

use std::collections::HashMap;
use std::sync::Arc;

use crate::routing::BlockId;
use wafer_run::block::Block;

/// Mapping from feature name (used in solobase.json) to BlockId.
const FEATURE_BLOCKS: &[(&str, BlockId)] = &[
    ("profile",     BlockId::Profile),
    ("system",      BlockId::System),
    ("userportal",  BlockId::UserPortal),
    ("legalpages",  BlockId::LegalPages),
    ("auth",        BlockId::Auth),
    ("admin",       BlockId::Admin),
    ("files",       BlockId::Files),
    ("products",    BlockId::Products),
    ("deployments", BlockId::Deployments),
];

/// Create a block instance for a given BlockId.
fn make_block(id: BlockId) -> Arc<dyn Block> {
    match id {
        BlockId::Profile     => Arc::new(profile::ProfileBlock),
        BlockId::System      => Arc::new(system::SystemBlock),
        BlockId::UserPortal  => Arc::new(userportal::UserPortalBlock),
        BlockId::LegalPages  => Arc::new(legalpages::LegalPagesBlock),
        BlockId::Auth        => Arc::new(auth::AuthBlock::new()),
        BlockId::Admin       => Arc::new(admin::AdminBlock),
        BlockId::Files       => Arc::new(files::FilesBlock::new()),
        BlockId::Products    => Arc::new(products::ProductsBlock::new()),
        BlockId::Deployments => Arc::new(deployments::DeploymentsBlock::new()),
    }
}

/// Create shared block instances filtered by a predicate.
///
/// Returns a map of BlockId → Arc<dyn Block> for enabled features.
/// The same Arc instances should be registered with the WAFER runtime
/// (for lifecycle hooks) and passed to the `NativeBlockFactory` (for
/// request dispatch), ensuring state is shared.
#[cfg(not(target_arch = "wasm32"))]
pub fn create_blocks(filter: impl Fn(&str) -> bool) -> HashMap<BlockId, Arc<dyn Block>> {
    let mut map = HashMap::new();
    for &(name, id) in FEATURE_BLOCKS {
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
#[cfg(not(target_arch = "wasm32"))]
pub fn register_shared_blocks(
    w: &mut wafer_run::Wafer,
    blocks: &HashMap<BlockId, Arc<dyn Block>>,
) {
    for (&id, block) in blocks {
        let name = block_id_to_name(id);
        w.register_block(format!("suppers-ai/{name}"), block.clone());
    }
}

fn block_id_to_name(id: BlockId) -> &'static str {
    match id {
        BlockId::Profile     => "profile",
        BlockId::System      => "system",
        BlockId::UserPortal  => "userportal",
        BlockId::LegalPages  => "legalpages",
        BlockId::Auth        => "auth",
        BlockId::Admin       => "admin",
        BlockId::Files       => "files",
        BlockId::Products    => "products",
        BlockId::Deployments => "deployments",
    }
}
