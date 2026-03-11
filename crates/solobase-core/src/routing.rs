//! Shared routing table — maps URL path prefixes to solobase blocks.
//!
//! Both Cloudflare and native adapters use this same routing logic.
//! Block instantiation is provided by the caller via a factory function,
//! keeping this crate free of solobase block dependencies.

use std::sync::Arc;

use wafer_run::block::Block;
use wafer_run::context::Context;
use wafer_run::types::*;

use crate::features::FeatureConfig;

/// Block identifier for the routing table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockId {
    System,
    Auth,
    Admin,
    Files,
    LegalPages,
    Products,
    Deployments,
    UserPortal,
    Profile,
}

/// A single route entry.
pub struct Route {
    pub prefix: &'static str,
    pub requires_admin: bool,
    pub block_id: BlockId,
}

/// The shared routing table. Order matters — more specific prefixes before general ones.
pub const ROUTES: &[Route] = &[
    // System
    Route { prefix: "/health",                  requires_admin: false, block_id: BlockId::System },
    Route { prefix: "/nav",                     requires_admin: false, block_id: BlockId::System },
    Route { prefix: "/debug/",                  requires_admin: false, block_id: BlockId::System },
    // Auth
    Route { prefix: "/auth/",                   requires_admin: false, block_id: BlockId::Auth },
    Route { prefix: "/internal/oauth/",         requires_admin: false, block_id: BlockId::Auth },
    // Admin sub-routes (more specific before general)
    Route { prefix: "/admin/settings/",         requires_admin: true,  block_id: BlockId::Admin },
    Route { prefix: "/settings/",               requires_admin: true,  block_id: BlockId::Admin },
    Route { prefix: "/admin/storage/",          requires_admin: true,  block_id: BlockId::Files },
    Route { prefix: "/admin/b/cloudstorage/",   requires_admin: true,  block_id: BlockId::Files },
    Route { prefix: "/admin/legalpages/",       requires_admin: true,  block_id: BlockId::LegalPages },
    Route { prefix: "/admin/b/products",        requires_admin: true,  block_id: BlockId::Products },
    Route { prefix: "/admin/b/deployments",     requires_admin: true,  block_id: BlockId::Deployments },
    Route { prefix: "/admin/",                  requires_admin: true,  block_id: BlockId::Admin },
    // Non-admin feature routes
    Route { prefix: "/storage/",                requires_admin: false, block_id: BlockId::Files },
    Route { prefix: "/b/cloudstorage/",         requires_admin: false, block_id: BlockId::Files },
    Route { prefix: "/b/products",              requires_admin: false, block_id: BlockId::Products },
    Route { prefix: "/b/legalpages",            requires_admin: false, block_id: BlockId::LegalPages },
    Route { prefix: "/b/deployments",           requires_admin: false, block_id: BlockId::Deployments },
    Route { prefix: "/b/userportal",            requires_admin: false, block_id: BlockId::UserPortal },
    Route { prefix: "/profile",                 requires_admin: false, block_id: BlockId::Profile },
];

/// Check if a block's feature is enabled.
fn is_block_enabled(block_id: BlockId, features: &dyn FeatureConfig) -> bool {
    match block_id {
        BlockId::System | BlockId::Profile => true, // always on
        BlockId::Auth        => features.auth_enabled(),
        BlockId::Admin       => features.admin_enabled(),
        BlockId::Files       => features.files_enabled(),
        BlockId::Products    => features.products_enabled(),
        BlockId::Deployments => features.deployments_enabled(),
        BlockId::LegalPages  => features.legalpages_enabled(),
        BlockId::UserPortal  => features.userportal_enabled(),
    }
}

/// Block factory — the caller provides this to create block instances.
///
/// This keeps solobase-core decoupled from the actual block implementations.
/// Implementations may return fresh instances (CF) or shared `Arc` clones (native).
pub trait BlockFactory: Send + Sync {
    fn create(&self, block_id: BlockId) -> Arc<dyn Block>;
}

/// Route a message to the appropriate solobase block based on request path.
///
/// Checks feature flags and admin role. Uses the provided `factory` to
/// instantiate the matched block.
pub async fn route_to_block(
    ctx: &dyn Context,
    msg: &mut Message,
    features: &dyn FeatureConfig,
    factory: &dyn BlockFactory,
) -> Result_ {
    let path = msg.path().to_string();

    for route in ROUTES {
        let matches = path == route.prefix || path.starts_with(route.prefix);
        if !matches {
            continue;
        }

        // Feature gate
        if !is_block_enabled(route.block_id, features) {
            return wafer_run::helpers::err_not_found(msg, "endpoint not found");
        }

        // Admin gate
        if route.requires_admin && !msg.is_admin() {
            return wafer_run::helpers::err_forbidden(msg, "admin access required");
        }

        // Dispatch to block
        let block = factory.create(route.block_id);
        return block.handle(ctx, msg).await;
    }

    wafer_run::helpers::err_not_found(msg, "endpoint not found")
}
