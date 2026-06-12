//! Shared routing table — maps URL path prefixes to solobase blocks.
//!
//! Both Cloudflare and native adapters use this same routing logic.
//! All solobase blocks are registered in the Wafer registry at boot; routing
//! dispatches via `ctx.call_block` without any factory indirection.

use wafer_run::{context::Context, InputStream, OutputStream, Message};

use crate::{blocks::helpers, features::FeatureConfig};

/// A single route entry.
///
/// `block` is the solobase block name (`{org}/{block}`) used for feature-gating
/// and the inspector's [`routes_config`] view. `dispatch_to` is the Wafer block
/// name passed to `ctx.call_block`; it equals `block` for every route except the
/// inspector, which is feature-gated/displayed as `suppers-ai/inspector` but
/// dispatches to the `wafer-run/inspector` runtime block.
pub struct Route {
    pub prefix: &'static str,
    pub access: RouteAccess,
    pub block: &'static str,
    pub dispatch_to: &'static str,
}

impl Route {
    /// A route whose dispatch target equals its block name (the common case).
    const fn new(prefix: &'static str, access: RouteAccess, block: &'static str) -> Route {
        Route {
            prefix,
            access,
            block,
            dispatch_to: block,
        }
    }

    /// A route whose `ctx.call_block` target differs from its block name. Used
    /// only by the inspector, which dispatches to the `wafer-run/inspector`
    /// runtime block while remaining feature-gated as `suppers-ai/inspector`.
    const fn proxy(
        prefix: &'static str,
        access: RouteAccess,
        block: &'static str,
        dispatch_to: &'static str,
    ) -> Route {
        Route {
            prefix,
            access,
            block,
            dispatch_to,
        }
    }
}

/// Access tier for a route.
///
/// Checked by [`route_to_block`] (via `check_access`) before dispatching to the
/// target block, for both built-in [`Route`]s and runtime-added [`ExtraRoute`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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
/// Carries an owned `block_name` `String` (rather than the built-in [`Route`]'s
/// `&'static str`) since projects supply these at build time.
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
    Route::new("/health", RouteAccess::Public, "suppers-ai/system"),
    Route::new("/b/static/", RouteAccess::Public, "suppers-ai/system"),
    // Inspector — runtime debugging UI (admin only). Feature-gated as
    // `suppers-ai/inspector` but dispatches to the `wafer-run/inspector` block.
    Route::proxy(
        "/b/inspector",
        RouteAccess::Admin,
        "suppers-ai/inspector",
        "wafer-run/inspector",
    ),
    // Auth — SSR pages + API under /b/auth/
    Route::new("/b/auth/", RouteAccess::Public, "suppers-ai/auth-ui"),
    // Admin settings — more specific prefix must come before the /b/admin/ catch-all
    Route::new("/b/admin/settings", RouteAccess::Admin, "suppers-ai/admin"),
    // Admin — SSR pages + API under /b/admin/
    Route::new("/b/admin/", RouteAccess::Admin, "suppers-ai/admin"),
    Route::new("/b/admin", RouteAccess::Admin, "suppers-ai/admin"),
    // Feature blocks — SSR + API under /b/{block}/
    Route::new("/b/storage/", RouteAccess::Public, "suppers-ai/files"),
    Route::new("/b/cloudstorage/", RouteAccess::Public, "suppers-ai/files"),
    Route::new("/b/products", RouteAccess::Public, "suppers-ai/products"),
    // Legalpages — public reads + admin writes/UI.
    // Admin and API prefixes must come BEFORE the bare `/b/legalpages` entry
    // because `route_to_block` matches on first-prefix-hit. Admin handlers
    // inside the block do not re-check `is_admin`, so this gate is the only
    // thing keeping random callers off `/admin/publish` and friends.
    Route::new(
        "/b/legalpages/admin",
        RouteAccess::Admin,
        "suppers-ai/legalpages",
    ),
    Route::new(
        "/b/legalpages/api",
        RouteAccess::Admin,
        "suppers-ai/legalpages",
    ),
    Route::new(
        "/b/legalpages",
        RouteAccess::Public,
        "suppers-ai/legalpages",
    ),
    Route::new(
        "/b/userportal",
        RouteAccess::Public,
        "suppers-ai/userportal",
    ),
    // Messages — generic thread/message system
    // Route is open; block enforces admin for UI pages, authenticated for API
    Route::new("/b/messages", RouteAccess::Public, "suppers-ai/messages"),
    // LLM — chat orchestrator
    // Route is open; block enforces admin for UI pages, authenticated for API
    Route::new("/b/llm", RouteAccess::Public, "suppers-ai/llm"),
    // Vector — similarity search, hybrid retrieval, RAG ingestion.
    //
    // Each endpoint from `VectorBlock::info().endpoints` is registered as a
    // separate entry so the inspector's routes document reflects the
    // granularity the block exposes. The prefix matcher uses `starts_with`,
    // so entries are ordered most-specific-first:
    //   - `DELETE /b/vector/api/indexes/{name}` must be listed BEFORE the
    //     generic `DELETE /b/vector/api/{index}/{id}` entry so the specific
    //     "delete index" route wins over the generic "delete vector" route.
    // All entries dispatch to `suppers-ai/vector`; per-method path-param
    // matching happens inside the block's `pages::route` dispatcher.
    Route::new(
        "/b/vector/api/indexes/{name}",
        RouteAccess::Public,
        "suppers-ai/vector",
    ),
    Route::new(
        "/b/vector/api/indexes",
        RouteAccess::Public,
        "suppers-ai/vector",
    ),
    Route::new(
        "/b/vector/api/upsert",
        RouteAccess::Public,
        "suppers-ai/vector",
    ),
    Route::new(
        "/b/vector/api/query",
        RouteAccess::Public,
        "suppers-ai/vector",
    ),
    Route::new(
        "/b/vector/api/ingest",
        RouteAccess::Public,
        "suppers-ai/vector",
    ),
    Route::new(
        "/b/vector/api/embed",
        RouteAccess::Public,
        "suppers-ai/vector",
    ),
    Route::new(
        "/b/vector/api/stats",
        RouteAccess::Public,
        "suppers-ai/vector",
    ),
    // Generic `DELETE /b/vector/api/{index}/{id}` — MUST come after the
    // more specific `/b/vector/api/indexes/{name}` entry above so that
    // path-prefix ordering routes index-deletes to the correct handler.
    Route::new(
        "/b/vector/api/{index}/{id}",
        RouteAccess::Public,
        "suppers-ai/vector",
    ),
    // SSR pages and any other /b/vector/* paths.
    Route::new("/b/vector/", RouteAccess::Public, "suppers-ai/vector"),
];

/// Generate the routing table as JSON config (same format as wafer-run/router).
/// Used to expose routes to the inspector.
pub fn routes_config() -> serde_json::Value {
    let routes: Vec<serde_json::Value> = ROUTES
        .iter()
        .map(|r| {
            let path = format!("{}**", r.prefix);
            serde_json::json!({ "path": path, "block": r.block })
        })
        .collect();
    serde_json::json!({ "routes": routes })
}

/// Enforce a route's [`RouteAccess`] tier against the request. Returns
/// `Some(forbidden_response)` when the caller fails the tier, or `None` to
/// proceed. Shared by the built-in and extra-route dispatch loops.
fn check_access(access: RouteAccess, msg: &Message) -> Option<OutputStream> {
    match access {
        RouteAccess::Public => None,
        RouteAccess::Authenticated if msg.user_id().is_empty() => {
            Some(crate::ui::forbidden_response(msg))
        }
        RouteAccess::Authenticated => None,
        RouteAccess::Admin if !helpers::is_admin(msg) => Some(crate::ui::forbidden_response(msg)),
        RouteAccess::Admin => None,
    }
}

/// Route a message to the appropriate solobase block based on request path.
///
/// Checks feature flags and admin role. Dispatches via `ctx.call_block` — all
/// solobase blocks are registered in the Wafer registry at boot (zero-arg
/// blocks via `register_static_block!`, LlmBlock via `register_llm()`).
pub async fn route_to_block(
    ctx: &dyn Context,
    msg: Message,
    input: InputStream,
    features: &dyn FeatureConfig,
    extra_routes: &[ExtraRoute],
) -> OutputStream {
    let path = msg.path().to_string();

    // Root: redirect logged-in users to portal dashboard, anonymous to login.
    // When the deployment ships a static landing page, serve it directly via
    // `wafer-run/web` instead. Gated by the `SOLOBASE_SHARED__HAS_LANDING_PAGE`
    // config var so the decision is explicit and works identically on native
    // and Cloudflare (no filesystem probe, which is meaningless on Workers and
    // CWD-relative on native).
    if path == "/" {
        let has_landing_page = ctx
            .config_get("SOLOBASE_SHARED__HAS_LANDING_PAGE")
            .unwrap_or("false")
            == "true";
        if has_landing_page {
            return ctx.call_block("wafer-run/web", msg, input).await;
        }
        return root_redirect(msg.user_id().is_empty());
    }

    for route in ROUTES {
        let matches = path == route.prefix || path.starts_with(route.prefix);
        if !matches {
            continue;
        }

        // Feature gate
        if !features.is_block_enabled(route.block) {
            return crate::blocks::helpers::err_not_found("endpoint not found");
        }

        // Access gate
        if let Some(denied) = check_access(route.access, &msg) {
            return denied;
        }

        // Dispatch via call_block so WRAP sees the correct caller identity.
        return ctx.call_block(route.dispatch_to, msg, input).await;
    }

    // Fall back to project-registered extra routes. Built-ins above win on
    // prefix collision — this loop only runs when no built-in matched.
    for route in extra_routes {
        let matches = path == route.prefix || path.starts_with(&route.prefix);
        if !matches {
            continue;
        }

        if let Some(denied) = check_access(route.access, &msg) {
            return denied;
        }

        return ctx.call_block(&route.block_name, msg, input).await;
    }

    crate::ui::not_found_response(&msg)
}

/// Build a root redirect response. Extracted for unit testability.
fn root_redirect(user_id_empty: bool) -> OutputStream {
    let target = if user_id_empty {
        "/b/auth/login"
    } else {
        "/b/userportal/"
    };
    crate::blocks::helpers::ResponseBuilder::new()
        .status(302)
        .set_header("Location", target)
        .body(Vec::new(), "text/plain")
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
            ("/health", "suppers-ai/system"),
            ("/b/static/app.css", "suppers-ai/system"),
            // Inspector
            ("/b/inspector", "suppers-ai/inspector"),
            ("/b/inspector/blocks", "suppers-ai/inspector"),
            // All block routes under /b/
            ("/b/auth/login", "suppers-ai/auth-ui"),
            ("/b/auth/signup", "suppers-ai/auth-ui"),
            ("/b/auth/api/me", "suppers-ai/auth-ui"),
            ("/b/admin/", "suppers-ai/admin"),
            ("/b/admin/users", "suppers-ai/admin"),
            ("/b/admin", "suppers-ai/admin"),
            ("/b/storage/buckets", "suppers-ai/files"),
            ("/b/cloudstorage/shares", "suppers-ai/files"),
            ("/b/products", "suppers-ai/products"),
            ("/b/legalpages", "suppers-ai/legalpages"),
            ("/b/userportal", "suppers-ai/userportal"),
        ];

        for (path, expected_block) in cases {
            let matched = ROUTES
                .iter()
                .find(|r| path == r.prefix || path.starts_with(r.prefix));
            assert!(matched.is_some(), "path {path} should match a route");
            assert_eq!(
                matched.unwrap().block,
                expected_block,
                "path {path} should route to {expected_block}"
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
                assert_eq!(
                    route.access,
                    RouteAccess::Admin,
                    "route {} should require admin",
                    route.prefix
                );
            }
        }
    }

    #[test]
    fn non_admin_routes_dont_require_admin() {
        // Note: `/b/legalpages` is intentionally omitted here because it has
        // sub-routes (`/b/legalpages/admin`, `/b/legalpages/api`) that DO
        // require admin. Those sub-routes are verified by
        // `legalpages_admin_routes_require_admin`.
        let non_admin_prefixes = vec![
            "/health",
            "/static/",
            "/b/auth/",
            "/b/storage/",
            "/b/products",
            "/b/userportal",
            "/b/cloudstorage/",
        ];
        for route in ROUTES {
            if non_admin_prefixes
                .iter()
                .any(|p| route.prefix == *p || route.prefix.starts_with(p))
            {
                assert_ne!(
                    route.access,
                    RouteAccess::Admin,
                    "route {} should NOT require admin",
                    route.prefix
                );
            }
        }
    }

    #[tokio::test]
    async fn root_redirects_anonymous_to_login() {
        let out = super::root_redirect(true);
        let buf = out.collect_buffered().await.unwrap();
        let status = buf
            .meta
            .iter()
            .find(|e| e.key == "resp.status")
            .map(|e| e.value.as_str())
            .unwrap_or("");
        let location = buf
            .meta
            .iter()
            .find(|e| e.key == "resp.header.Location")
            .map(|e| e.value.as_str())
            .unwrap_or("");
        assert_eq!(status, "302");
        assert_eq!(location, "/b/auth/login");
    }

    #[tokio::test]
    async fn root_redirects_authenticated_to_portal_home() {
        let out = super::root_redirect(false);
        let buf = out.collect_buffered().await.unwrap();
        let location = buf
            .meta
            .iter()
            .find(|e| e.key == "resp.header.Location")
            .map(|e| e.value.as_str())
            .unwrap_or("");
        assert_eq!(location, "/b/userportal/");
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

    /// The block names every built-in route feature-gates against. The
    /// `route_to_block` feature gate calls `features.is_block_enabled(route.block)`.
    const GATED_BLOCKS: &[&str] = &[
        "suppers-ai/auth-ui",
        "suppers-ai/admin",
        "suppers-ai/files",
        "suppers-ai/products",
        "suppers-ai/legalpages",
        "suppers-ai/userportal",
    ];

    #[test]
    fn feature_gating_all_enabled() {
        let all = AllEnabled;
        for block in GATED_BLOCKS {
            assert!(all.is_block_enabled(block), "{block} should be enabled");
        }
    }

    #[test]
    fn feature_gating_all_disabled() {
        let none = NoneEnabled;
        for block in GATED_BLOCKS {
            assert!(!none.is_block_enabled(block), "{block} should be disabled");
        }
    }

    #[test]
    fn legalpages_admin_routes_require_admin() {
        let admin_route = ROUTES
            .iter()
            .find(|r| r.prefix == "/b/legalpages/admin")
            .expect("legalpages admin route not declared");
        assert_eq!(
            admin_route.access,
            RouteAccess::Admin,
            "/b/legalpages/admin must require admin"
        );
        assert_eq!(admin_route.block, "suppers-ai/legalpages");

        let api_route = ROUTES
            .iter()
            .find(|r| r.prefix == "/b/legalpages/api")
            .expect("legalpages api route not declared");
        assert_eq!(
            api_route.access,
            RouteAccess::Admin,
            "/b/legalpages/api must require admin"
        );

        let public_route = ROUTES
            .iter()
            .find(|r| r.prefix == "/b/legalpages")
            .expect("public legalpages route not declared");
        assert_ne!(
            public_route.access,
            RouteAccess::Admin,
            "/b/legalpages must remain public"
        );

        // Most-specific-first ordering matters for the `starts_with` matcher.
        let positions: Vec<_> = ROUTES
            .iter()
            .enumerate()
            .filter(|(_, r)| r.block == "suppers-ai/legalpages")
            .map(|(i, r)| (i, r.prefix))
            .collect();
        assert_eq!(
            positions.iter().map(|(_, p)| *p).collect::<Vec<_>>(),
            vec!["/b/legalpages/admin", "/b/legalpages/api", "/b/legalpages"],
            "legalpages routes must be ordered most-specific-first",
        );
    }

    #[test]
    fn all_block_routes_are_under_b_prefix() {
        for route in ROUTES {
            let is_system = route.block == "suppers-ai/system";
            if !is_system {
                assert!(
                    route.prefix.starts_with("/b/"),
                    "block route {} should start with /b/",
                    route.prefix
                );
            }
        }
    }

    #[test]
    fn inspector_dispatch_diverges_from_block_name() {
        // The inspector is the one route whose dispatch target differs from its
        // feature/display name: gated as `suppers-ai/inspector`, dispatched to
        // the `wafer-run/inspector` runtime block.
        let inspector = ROUTES
            .iter()
            .find(|r| r.prefix == "/b/inspector")
            .expect("inspector route not declared");
        assert_eq!(inspector.block, "suppers-ai/inspector");
        assert_eq!(inspector.dispatch_to, "wafer-run/inspector");
    }

    #[test]
    fn only_inspector_has_a_dispatch_override() {
        // Every other route dispatches to its own block name (the `new`
        // constructor's invariant). Catches a stray `proxy` entry.
        for route in ROUTES {
            if route.prefix == "/b/inspector" {
                continue;
            }
            assert_eq!(
                route.dispatch_to, route.block,
                "route {} should dispatch to its own block",
                route.prefix
            );
        }
    }

    #[test]
    fn routes_config_uses_display_block_name_for_inspector() {
        // routes_config() must show the inspector as `suppers-ai/inspector`
        // (the display/feature name), not its `wafer-run/inspector` dispatch
        // target — the inspector UI keys its feature map on the former.
        let cfg = super::routes_config();
        let routes = cfg["routes"].as_array().expect("routes array");
        let inspector = routes
            .iter()
            .find(|r| r["path"] == "/b/inspector**")
            .expect("inspector route in config");
        assert_eq!(inspector["block"], "suppers-ai/inspector");
    }
}
