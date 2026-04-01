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
    Projects,
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
///
/// Block SSR pages are served under `/b/{block_name}/...`. API paths use the
/// same prefix. The block's `handle()` decides whether to return HTML or JSON.
pub const ROUTES: &[Route] = &[
    // System & static assets
    Route {
        prefix: "/health",
        requires_admin: false,
        block_id: BlockId::System,
    },
    Route {
        prefix: "/nav",
        requires_admin: false,
        block_id: BlockId::System,
    },
    Route {
        prefix: "/debug/",
        requires_admin: true,
        block_id: BlockId::System,
    },
    Route {
        prefix: "/static/",
        requires_admin: false,
        block_id: BlockId::System,
    },
    // Auth (SSR pages + API under /b/auth/)
    Route {
        prefix: "/b/auth/",
        requires_admin: false,
        block_id: BlockId::Auth,
    },
    Route {
        prefix: "/auth/",
        requires_admin: false,
        block_id: BlockId::Auth,
    },
    Route {
        prefix: "/internal/oauth/",
        requires_admin: false,
        block_id: BlockId::Auth,
    },
    // Admin (SSR pages + API under /b/admin/)
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
    Route {
        prefix: "/admin/settings",
        requires_admin: true,
        block_id: BlockId::Admin,
    },
    Route {
        prefix: "/settings",
        requires_admin: true,
        block_id: BlockId::Admin,
    },
    Route {
        prefix: "/admin/storage/",
        requires_admin: true,
        block_id: BlockId::Files,
    },
    Route {
        prefix: "/admin/b/cloudstorage/",
        requires_admin: true,
        block_id: BlockId::Files,
    },
    Route {
        prefix: "/admin/legalpages/",
        requires_admin: true,
        block_id: BlockId::LegalPages,
    },
    Route {
        prefix: "/admin/b/products",
        requires_admin: true,
        block_id: BlockId::Products,
    },
    Route {
        prefix: "/admin/b/projects",
        requires_admin: true,
        block_id: BlockId::Projects,
    },
    Route {
        prefix: "/admin/",
        requires_admin: true,
        block_id: BlockId::Admin,
    },
    // Feature blocks (SSR + API)
    Route {
        prefix: "/storage/",
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
    Route {
        prefix: "/profile",
        requires_admin: false,
        block_id: BlockId::Profile,
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
        BlockId::Auth => "auth",
        BlockId::Admin => "admin",
        BlockId::Files => "files",
        BlockId::LegalPages => "legalpages",
        BlockId::Products => "products",
        BlockId::Projects => "projects",
        BlockId::UserPortal => "userportal",
        BlockId::Profile => "profile",
    }
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
        if route.requires_admin
            && !msg
                .get_meta("auth.user_roles")
                .split(',')
                .any(|r| r.trim() == "admin")
        {
            return crate::ui::forbidden_response(msg);
        }

        // Dispatch to block
        let block = match factory.create(route.block_id) {
            Some(b) => b,
            None => return wafer_run::helpers::err_internal(msg, "block not available"),
        };
        return block.handle(ctx, msg).await;
    }

    crate::ui::not_found_response(msg)
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
            ("/health", BlockId::System),
            ("/nav", BlockId::System),
            ("/debug/info", BlockId::System),
            ("/static/app.css", BlockId::System),
            // SSR pages under /b/{name}/
            ("/b/auth/login", BlockId::Auth),
            ("/b/auth/signup", BlockId::Auth),
            ("/b/admin/", BlockId::Admin),
            ("/b/admin/users", BlockId::Admin),
            ("/b/admin", BlockId::Admin),
            // Legacy API paths (still routed)
            ("/auth/login", BlockId::Auth),
            ("/auth/signup", BlockId::Auth),
            ("/internal/oauth/callback", BlockId::Auth),
            ("/admin/settings", BlockId::Admin),
            ("/settings", BlockId::Admin),
            ("/admin/storage/buckets", BlockId::Files),
            ("/admin/legalpages/documents", BlockId::LegalPages),
            ("/admin/b/products", BlockId::Products),
            ("/admin/b/projects", BlockId::Projects),
            ("/admin/users", BlockId::Admin),
            ("/storage/upload", BlockId::Files),
            ("/b/products", BlockId::Products),
            ("/b/legalpages", BlockId::LegalPages),
            ("/b/projects", BlockId::Projects),
            ("/b/userportal", BlockId::UserPortal),
            ("/profile", BlockId::Profile),
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
        let unmatched = vec!["/unknown", "/foo/bar", "/"];
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
            let is_admin_route = route.prefix.starts_with("/admin/")
                || route.prefix == "/settings"
                || route.prefix.starts_with("/b/admin");
            if is_admin_route {
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
            "/nav",
            "/static/",
            "/b/auth/",
            "/auth/",
            "/internal/oauth/",
            "/storage/",
            "/b/products",
            "/b/legalpages",
            "/b/projects",
            "/b/userportal",
            "/b/cloudstorage/",
            "/profile",
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
    fn more_specific_admin_routes_come_before_general() {
        // /admin/storage/ should match Files, not Admin (which is /admin/)
        let route = ROUTES
            .iter()
            .find(|r| "/admin/storage/buckets".starts_with(r.prefix))
            .unwrap();
        assert_eq!(route.block_id, BlockId::Files);

        // /admin/legalpages/ should match LegalPages, not Admin
        let route = ROUTES
            .iter()
            .find(|r| "/admin/legalpages/documents".starts_with(r.prefix))
            .unwrap();
        assert_eq!(route.block_id, BlockId::LegalPages);
    }
}
