//! Shared routing table — maps URL path prefixes to solobase blocks.
//!
//! Both Cloudflare and native adapters use this same routing logic.
//! Block instantiation is provided by the caller via a factory function,
//! keeping this crate free of solobase block dependencies.

use std::sync::Arc;

use wafer_run::{block::Block, context::Context, types::*, InputStream, OutputStream};

use crate::{blocks::helpers, features::FeatureConfig};

/// Block identifier for the routing table.
///
/// Most variants map to an HTTP route prefix in [`ROUTES`]; some (e.g. the
/// embedding blocks) are pure service blocks with no HTTP surface — they
/// still have a `BlockId` so they flow through the same `create_blocks` /
/// `register_shared_blocks` pipeline as feature blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockId {
    System,
    Inspector,
    Auth,
    Admin,
    Files,
    LegalPages,
    Products,
    Projects,
    UserPortal,
    Messages,
    Llm,
    ProviderLlm,
    LocalLlm,
    Vector,
    /// Native ONNX embedding service (no HTTP routes). Feature-gated
    /// behind `native-embedding` — see `blocks::fastembed`.
    Fastembed,
}

/// A single route entry.
pub struct Route {
    pub prefix: &'static str,
    pub requires_admin: bool,
    pub block_id: BlockId,
}

/// Access tier for a runtime-added [`ExtraRoute`].
///
/// Checked by [`route_to_block`] before dispatching to the target block.
///
/// Built-in [`Route`]s still use the `requires_admin: bool` field —
/// migrating those to `RouteAccess` would be a wider refactor with no
/// behavioural change, so the two systems coexist: built-ins as booleans,
/// extras as declarative tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteAccess {
    /// No auth check. Anyone can hit this route.
    Public,
    /// `msg.user_id()` must be non-empty, or the request is rejected with 403.
    Authenticated,
    /// User must have the `admin` role (per [`helpers::is_admin`]) or 403.
    Admin,
}

/// A runtime-added route registered by a downstream project via
/// `SolobaseBuilder::add_route`.
///
/// Dispatches by block name string (not [`BlockId`]) since projects supply
/// these at build time and cannot extend the closed `BlockId` enum.
///
/// # Priority
///
/// Built-in [`ROUTES`] always win. An extra route with the same prefix as a
/// built-in is ignored. To disable a built-in route, disable its feature
/// flag — do not try to override it.
#[derive(Debug, Clone)]
pub struct ExtraRoute {
    pub prefix: String,
    pub access: RouteAccess,
    pub block_name: String,
}

/// The shared routing table. Order matters — more specific prefixes before general ones.
///
/// All block routes live under `/b/{block_name}/...`. SSR pages and JSON API
/// share the same prefix — blocks distinguish by HTTP method and path.
/// System endpoints (`/health`, `/nav`, `/static/`, `/debug/`) are the only
/// routes outside `/b/`.
pub const ROUTES: &[Route] = &[
    // System & static assets
    Route {
        prefix: "/health",
        requires_admin: false,
        block_id: BlockId::System,
    },
    Route {
        prefix: "/b/static/",
        requires_admin: false,
        block_id: BlockId::System,
    },
    // Inspector — runtime debugging UI (admin only)
    Route {
        prefix: "/b/inspector",
        requires_admin: true,
        block_id: BlockId::Inspector,
    },
    // Auth — SSR pages + API under /b/auth/
    Route {
        prefix: "/b/auth/",
        requires_admin: false,
        block_id: BlockId::Auth,
    },
    // Admin — SSR pages + API under /b/admin/
    Route {
        prefix: "/b/admin/",
        requires_admin: true,
        block_id: BlockId::Admin,
    },
    Route {
        prefix: "/b/admin",
        requires_admin: true,
        block_id: BlockId::Admin,
    },
    // Feature blocks — SSR + API under /b/{block}/
    Route {
        prefix: "/b/storage/",
        requires_admin: false,
        block_id: BlockId::Files,
    },
    Route {
        prefix: "/b/cloudstorage/",
        requires_admin: false,
        block_id: BlockId::Files,
    },
    Route {
        prefix: "/b/products",
        requires_admin: false,
        block_id: BlockId::Products,
    },
    Route {
        prefix: "/b/legalpages",
        requires_admin: false,
        block_id: BlockId::LegalPages,
    },
    Route {
        prefix: "/b/projects",
        requires_admin: false,
        block_id: BlockId::Projects,
    },
    Route {
        prefix: "/b/userportal",
        requires_admin: false,
        block_id: BlockId::UserPortal,
    },
    // Messages — generic thread/message system
    // Route is open; block enforces admin for UI pages, authenticated for API
    Route {
        prefix: "/b/messages",
        requires_admin: false,
        block_id: BlockId::Messages,
    },
    // LLM — chat orchestrator
    // Route is open; block enforces admin for UI pages, authenticated for API
    Route {
        prefix: "/b/llm",
        requires_admin: false,
        block_id: BlockId::Llm,
    },
    // Provider LLM — remote API providers
    // Route is open; block enforces admin for UI/config, authenticated for chat API
    Route {
        prefix: "/b/provider-llm",
        requires_admin: false,
        block_id: BlockId::ProviderLlm,
    },
    // Local LLM — WebLLM browser inference
    Route {
        prefix: "/b/local-llm",
        requires_admin: false,
        block_id: BlockId::LocalLlm,
    },
    // Vector — similarity search, hybrid retrieval, RAG ingestion.
    //
    // Each endpoint from `VectorBlock::info().endpoints` is registered as a
    // separate entry so the inspector's routes document reflects the
    // granularity the block exposes. The prefix matcher uses `starts_with`,
    // so entries are ordered most-specific-first:
    //   - `DELETE /b/vector/api/indexes/{name}` must be listed BEFORE the
    //     generic `DELETE /b/vector/api/{index}/{id}` entry so the specific
    //     "delete index" route wins over the generic "delete vector" route.
    // All entries dispatch to `BlockId::Vector`; per-method path-param
    // matching happens inside the block's `pages::route` dispatcher.
    Route {
        prefix: "/b/vector/api/indexes/{name}",
        requires_admin: false,
        block_id: BlockId::Vector,
    },
    Route {
        prefix: "/b/vector/api/indexes",
        requires_admin: false,
        block_id: BlockId::Vector,
    },
    Route {
        prefix: "/b/vector/api/upsert",
        requires_admin: false,
        block_id: BlockId::Vector,
    },
    Route {
        prefix: "/b/vector/api/query",
        requires_admin: false,
        block_id: BlockId::Vector,
    },
    Route {
        prefix: "/b/vector/api/ingest",
        requires_admin: false,
        block_id: BlockId::Vector,
    },
    Route {
        prefix: "/b/vector/api/embed",
        requires_admin: false,
        block_id: BlockId::Vector,
    },
    Route {
        prefix: "/b/vector/api/stats",
        requires_admin: false,
        block_id: BlockId::Vector,
    },
    // Generic `DELETE /b/vector/api/{index}/{id}` — MUST come after the
    // more specific `/b/vector/api/indexes/{name}` entry above so that
    // path-prefix ordering routes index-deletes to the correct handler.
    Route {
        prefix: "/b/vector/api/{index}/{id}",
        requires_admin: false,
        block_id: BlockId::Vector,
    },
    // SSR pages and any other /b/vector/* paths.
    Route {
        prefix: "/b/vector/",
        requires_admin: false,
        block_id: BlockId::Vector,
    },
];

/// Check if a block's feature is enabled.
fn is_block_enabled(block_id: BlockId, features: &dyn FeatureConfig) -> bool {
    let full_name = format!("suppers-ai/{}", block_id_short_name(block_id));
    features.is_block_enabled(&full_name)
}

/// Block factory — the caller provides this to create block instances.
///
/// This keeps solobase-core decoupled from the actual block implementations.
/// Implementations may return fresh instances (CF) or shared `Arc` clones (native).
pub trait BlockFactory: wafer_run::MaybeSend + wafer_run::MaybeSync {
    fn create(&self, block_id: BlockId) -> Option<Arc<dyn Block>>;
    /// Return `BlockInfo` for all registered blocks (used for discovery documents).
    fn all_block_infos(&self) -> Vec<wafer_run::BlockInfo>;
}

/// Generate the routing table as JSON config (same format as wafer-run/router).
/// Used to expose routes to the inspector.
pub fn routes_config() -> serde_json::Value {
    let routes: Vec<serde_json::Value> = ROUTES
        .iter()
        .map(|r| {
            let block_name = format!("suppers-ai/{}", block_id_short_name(r.block_id));
            let path = format!("{}**", r.prefix);
            serde_json::json!({ "path": path, "block": block_name })
        })
        .collect();
    serde_json::json!({ "routes": routes })
}

fn block_id_short_name(id: BlockId) -> &'static str {
    match id {
        BlockId::System => "system",
        BlockId::Inspector => "inspector",
        BlockId::Auth => "auth",
        BlockId::Admin => "admin",
        BlockId::Files => "files",
        BlockId::LegalPages => "legalpages",
        BlockId::Products => "products",
        BlockId::Projects => "projects",
        BlockId::UserPortal => "userportal",
        BlockId::Messages => "messages",
        BlockId::Llm => "llm",
        BlockId::ProviderLlm => "provider-llm",
        BlockId::LocalLlm => "local-llm",
        BlockId::Vector => "vector",
        BlockId::Fastembed => "fastembed",
    }
}

/// Route a message to the appropriate solobase block based on request path.
///
/// Checks feature flags and admin role. Uses the provided `factory` to
/// instantiate the matched block.
pub async fn route_to_block(
    ctx: &dyn Context,
    msg: Message,
    input: InputStream,
    features: &dyn FeatureConfig,
    _factory: &dyn BlockFactory,
    extra_routes: &[ExtraRoute],
) -> OutputStream {
    let path = msg.path().to_string();

    for route in ROUTES {
        let matches = path == route.prefix || path.starts_with(route.prefix);
        if !matches {
            continue;
        }

        // Feature gate
        if !is_block_enabled(route.block_id, features) {
            return crate::blocks::helpers::err_not_found("endpoint not found");
        }

        // Admin gate
        if route.requires_admin && !helpers::is_admin(&msg) {
            return crate::ui::forbidden_response(&msg);
        }

        // Dispatch to block via call_block so WRAP sees the correct caller identity
        if route.block_id == BlockId::Inspector {
            return ctx.call_block("wafer-run/inspector", msg, input).await;
        }
        let block_name = format!("suppers-ai/{}", block_id_short_name(route.block_id));
        return ctx.call_block(&block_name, msg, input).await;
    }

    // Fall back to project-registered extra routes. Built-ins above win on
    // prefix collision — this loop only runs when no built-in matched.
    for route in extra_routes {
        let matches = path == route.prefix || path.starts_with(&route.prefix);
        if !matches {
            continue;
        }

        match route.access {
            RouteAccess::Public => {}
            RouteAccess::Authenticated => {
                if msg.user_id().is_empty() {
                    return crate::ui::forbidden_response(&msg);
                }
            }
            RouteAccess::Admin => {
                if !helpers::is_admin(&msg) {
                    return crate::ui::forbidden_response(&msg);
                }
            }
        }

        return ctx.call_block(&route.block_name, msg, input).await;
    }

    crate::ui::not_found_response(&msg)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that the routing table covers expected prefixes and block assignments.
    #[test]
    fn route_table_maps_expected_paths() {
        let cases = vec![
            // System endpoints
            ("/health", BlockId::System),
            ("/b/static/app.css", BlockId::System),
            // Inspector
            ("/b/inspector", BlockId::Inspector),
            ("/b/inspector/blocks", BlockId::Inspector),
            // All block routes under /b/
            ("/b/auth/login", BlockId::Auth),
            ("/b/auth/signup", BlockId::Auth),
            ("/b/auth/api/me", BlockId::Auth),
            ("/b/admin/", BlockId::Admin),
            ("/b/admin/users", BlockId::Admin),
            ("/b/admin", BlockId::Admin),
            ("/b/storage/buckets", BlockId::Files),
            ("/b/cloudstorage/shares", BlockId::Files),
            ("/b/products", BlockId::Products),
            ("/b/legalpages", BlockId::LegalPages),
            ("/b/projects", BlockId::Projects),
            ("/b/userportal", BlockId::UserPortal),
        ];

        for (path, expected_block) in cases {
            let matched = ROUTES
                .iter()
                .find(|r| path == r.prefix || path.starts_with(r.prefix));
            assert!(matched.is_some(), "path {path} should match a route");
            assert_eq!(
                matched.unwrap().block_id,
                expected_block,
                "path {path} should route to {expected_block:?}"
            );
        }
    }

    #[test]
    fn unmatched_paths_have_no_route() {
        // Legacy paths no longer match — all block routes are under /b/
        let unmatched = vec![
            "/unknown",
            "/foo/bar",
            "/",
            "/auth/login",
            "/admin/settings",
            "/storage/buckets",
            "/settings",
            "/profile",
            "/nav",
            "/debug/time",
        ];
        for path in unmatched {
            let matched = ROUTES
                .iter()
                .find(|r| path == r.prefix || path.starts_with(r.prefix));
            assert!(matched.is_none(), "path {path} should NOT match any route");
        }
    }

    #[test]
    fn admin_routes_require_admin() {
        for route in ROUTES {
            if route.prefix.starts_with("/b/admin") {
                assert!(
                    route.requires_admin,
                    "route {} should require admin",
                    route.prefix
                );
            }
        }
    }

    #[test]
    fn non_admin_routes_dont_require_admin() {
        let non_admin_prefixes = vec![
            "/health",
            "/static/",
            "/b/auth/",
            "/b/storage/",
            "/b/products",
            "/b/legalpages",
            "/b/projects",
            "/b/userportal",
            "/b/cloudstorage/",
        ];
        for route in ROUTES {
            if non_admin_prefixes
                .iter()
                .any(|p| route.prefix == *p || route.prefix.starts_with(p))
            {
                assert!(
                    !route.requires_admin,
                    "route {} should NOT require admin",
                    route.prefix
                );
            }
        }
    }

    struct AllEnabled;
    impl FeatureConfig for AllEnabled {
        fn is_block_enabled(&self, _: &str) -> bool {
            true
        }
    }

    struct NoneEnabled;
    impl FeatureConfig for NoneEnabled {
        fn is_block_enabled(&self, _: &str) -> bool {
            false
        }
    }

    #[test]
    fn feature_gating_all_enabled() {
        let all = AllEnabled;
        assert!(is_block_enabled(BlockId::Auth, &all));
        assert!(is_block_enabled(BlockId::Admin, &all));
        assert!(is_block_enabled(BlockId::Files, &all));
        assert!(is_block_enabled(BlockId::Products, &all));
        assert!(is_block_enabled(BlockId::Projects, &all));
        assert!(is_block_enabled(BlockId::LegalPages, &all));
        assert!(is_block_enabled(BlockId::UserPortal, &all));
    }

    #[test]
    fn feature_gating_all_disabled() {
        let none = NoneEnabled;
        assert!(!is_block_enabled(BlockId::Auth, &none));
        assert!(!is_block_enabled(BlockId::Admin, &none));
        assert!(!is_block_enabled(BlockId::Files, &none));
        assert!(!is_block_enabled(BlockId::Products, &none));
        assert!(!is_block_enabled(BlockId::Projects, &none));
        assert!(!is_block_enabled(BlockId::LegalPages, &none));
        assert!(!is_block_enabled(BlockId::UserPortal, &none));
    }

    #[test]
    fn all_block_routes_are_under_b_prefix() {
        for route in ROUTES {
            let is_system = matches!(route.block_id, BlockId::System);
            if !is_system {
                assert!(
                    route.prefix.starts_with("/b/"),
                    "block route {} should start with /b/",
                    route.prefix
                );
            }
        }
    }
}
