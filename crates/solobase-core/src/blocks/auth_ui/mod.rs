//! `suppers-ai/auth-ui` — SSR pages + JSON API + OAuth flows + bootstrap token
//! redemption for solobase auth.
//!
//! Plan A2 PR 5 splits the legacy `suppers-ai/auth` block into two halves:
//!
//! - **Framework auth** (`suppers-ai/auth`, lives in `wafer-run` proper):
//!   service-shaped block exposing `auth@v1` (`require_user`/`require_role`/
//!   token issue+verify). Owns `JWT_SECRET`, `REQUIRE_VERIFICATION`,
//!   `ALLOWED_EMAIL_DOMAINS`, `INTERNAL_SECRET`. No HTTP routes.
//!
//! - **auth-ui** (this module): all `/b/auth/*` HTTP routes. Reads/writes
//!   auth tables via `repo::*` under WRAP grant. Calls the framework auth
//!   block via the `auth@v1` typed client for identity primitives.
//!
//! Declares the full `BlockInfo` (endpoints, requires,
//! OAuth-creds config_keys), runs the per-user/IP rate-limit middleware,
//! and dispatches every `/b/auth/*` route to a leaf module under `api/`,
//! `pages/`, or `oauth/`. The framework `suppers-ai/auth` block (in
//! `auth/`) owns the auth *service*; this block owns the HTTP surface.

pub mod api;
pub mod oauth;
pub mod pages;
pub mod redirect;

use wafer_run::{
    context::Context, AuthLevel, Block, BlockEndpoint, BlockInfo, ConfigVar, InputStream,
    InputType, InstanceMode, LifecycleEvent, Message, OutputStream, WaferError,
};

use super::rate_limit::{
    check_route_limits, LimitKey, RateLimit, RateLimitOutcome, RouteLimit, UserRateLimiter,
};
use crate::{blocks::helpers::err_not_found, endpoint_match};

pub const AUTH_UI_BLOCK_ID: &str = "suppers-ai/auth-ui";

/// Declarative rate-limit table for the auth-ui HTTP surface, replacing the
/// hand-rolled five-arm match. Rules are tried top-down; the first
/// `(action, path)` match wins (see [`check_route_limits`]). IP-keyed rules
/// guard unauthenticated endpoints; User-keyed rules guard authenticated ones.
const RATE_LIMIT_ROUTES: &[RouteLimit] = &[
    // Unauthenticated sensitive endpoints: login / signup — keyed by IP.
    RouteLimit {
        matches: |a, p| a == "create" && matches!(p, "/auth/api/login" | "/auth/api/signup"),
        key: LimitKey::Ip,
        category: "auth",
        limit: RateLimit::AUTH,
    },
    // Token refresh — keyed by IP, its own (looser) category.
    RouteLimit {
        matches: |a, p| a == "create" && p == "/auth/api/refresh",
        key: LimitKey::Ip,
        category: "refresh",
        limit: RateLimit::REFRESH,
    },
    // Forgot/reset password + verification — keyed by IP, shares the auth bucket.
    RouteLimit {
        matches: |a, p| match p {
            "/auth/api/forgot-password"
            | "/auth/api/reset-password"
            | "/auth/api/resend-verification" => a == "create",
            "/auth/api/verify" => a == "retrieve" || a == "create",
            _ => false,
        },
        key: LimitKey::Ip,
        category: "auth",
        limit: RateLimit::AUTH,
    },
    // Authenticated read endpoints — keyed by user_id.
    RouteLimit {
        matches: |a, p| a == "retrieve" && matches!(p, "/auth/api/me" | "/auth/api/api-keys"),
        key: LimitKey::User,
        category: "auth_read",
        limit: RateLimit::API_READ,
    },
    // Authenticated write endpoints — keyed by user_id. Catches every update /
    // delete plus the two non-update write endpoints. Ordered last so the
    // read rule above wins for retrieves.
    RouteLimit {
        matches: |a, p| {
            a == "update"
                || a == "delete"
                || (a == "create"
                    && matches!(p, "/auth/api/change-password" | "/auth/api/api-keys"))
        },
        key: LimitKey::User,
        category: "auth_write",
        limit: RateLimit::API_WRITE,
    },
];

/// The auth-ui block's own declared config vars (OAuth provider creds). Single
/// source of truth for both `BlockInfo::config_keys` and the admin settings
/// page (rendered via `ui::settings_form`, not a parallel tuple table).
///
/// OAuth provider creds live under the auth-ui prefix
/// (`SUPPERS_AI__AUTH_UI__OAUTH_*`) to keep the prefix-equals-block-name
/// invariant the runtime enforces (see `block_name_to_var_prefix`). The
/// auth-identity vars JWT_SECRET / REQUIRE_VERIFICATION / ALLOWED_EMAIL_DOMAINS
/// are `SUPPERS_AI__AUTH__*` and declared in `auth::config` instead.
pub(crate) fn config_vars() -> Vec<ConfigVar> {
    vec![
        ConfigVar::new(
            "SUPPERS_AI__AUTH_UI__OAUTH_GOOGLE_CLIENT_ID",
            "Google OAuth client ID",
            "",
        )
        .name("Google Client ID")
        .optional(),
        ConfigVar::new(
            "SUPPERS_AI__AUTH_UI__OAUTH_GOOGLE_CLIENT_SECRET",
            "Google OAuth client secret",
            "",
        )
        .name("Google Client Secret")
        .input_type(InputType::Password)
        .optional(),
        ConfigVar::new(
            "SUPPERS_AI__AUTH_UI__OAUTH_GITHUB_CLIENT_ID",
            "GitHub OAuth client ID",
            "",
        )
        .name("GitHub Client ID")
        .optional(),
        ConfigVar::new(
            "SUPPERS_AI__AUTH_UI__OAUTH_GITHUB_CLIENT_SECRET",
            "GitHub OAuth client secret",
            "",
        )
        .name("GitHub Client Secret")
        .input_type(InputType::Password)
        .optional(),
        ConfigVar::new(
            "SUPPERS_AI__AUTH_UI__OAUTH_MICROSOFT_CLIENT_ID",
            "Microsoft OAuth client ID",
            "",
        )
        .name("Microsoft Client ID")
        .optional(),
        ConfigVar::new(
            "SUPPERS_AI__AUTH_UI__OAUTH_MICROSOFT_CLIENT_SECRET",
            "Microsoft OAuth client secret",
            "",
        )
        .name("Microsoft Client Secret")
        .input_type(InputType::Password)
        .optional(),
        ConfigVar::new(
            "SUPPERS_AI__AUTH_UI__OAUTH_REDIRECT_URI",
            "OAuth callback URL",
            "",
        )
        .name("OAuth Redirect URI")
        .input_type(InputType::Url)
        .optional(),
    ]
}

pub struct AuthUiBlock {
    limiter: UserRateLimiter,
}

impl Default for AuthUiBlock {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthUiBlock {
    pub fn new() -> Self {
        Self {
            limiter: UserRateLimiter::new(),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for AuthUiBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo::new(
            AUTH_UI_BLOCK_ID,
            "0.0.1",
            "http-handler@v1",
            "SSR auth pages + login/signup/oauth/bootstrap handlers",
        )
        .instance_mode(InstanceMode::Singleton)
        .requires(vec![
            "wafer-run/database".into(),
            "wafer-run/crypto".into(),
            "wafer-run/config".into(),
            "wafer-run/network".into(),
            "suppers-ai/email".into(),
            "suppers-ai/auth".into(),
        ])
        .category(wafer_run::BlockCategory::Feature)
        .description(
            "Solobase auth HTTP surface (SSR pages, JSON API, OAuth, bootstrap \
             token redemption). Reads/writes auth tables via repo::* under WRAP \
             grant. Calls suppers-ai/auth via auth@v1 for require_user/role/token.",
        )
        .endpoints(vec![
            // Admin settings — declared `Admin` so the central router enforces
            // the tier; the handler no longer re-checks `is_admin`. (The
            // auth-ui prefix route is Public, so this declared level is the
            // gate for the admin settings surface.)
            BlockEndpoint::get("/b/auth/admin/settings")
                .summary("Auth settings page")
                .auth(AuthLevel::Admin),
            BlockEndpoint::post("/b/auth/admin/settings")
                .summary("Save auth settings")
                .auth(AuthLevel::Admin),
            // SSR pages
            BlockEndpoint::get("/b/auth/login").summary("Login page"),
            BlockEndpoint::get("/b/auth/signup").summary("Signup page"),
            BlockEndpoint::get("/b/auth/change-password")
                .summary("Change password page")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/auth/orgs")
                .summary("Claimed organizations")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/auth/oauth/login").summary("Start OAuth flow"),
            // JSON API
            BlockEndpoint::post("/b/auth/api/login").summary("Authenticate with email/password"),
            BlockEndpoint::post("/b/auth/api/signup").summary("Create account"),
            BlockEndpoint::post("/b/auth/api/logout")
                .summary("Sign out")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/auth/api/me")
                .summary("Get current user")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::post("/b/auth/api/change-password")
                .summary("Change password")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/auth/api/api-keys")
                .summary("List API keys")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::post("/b/auth/api/api-keys")
                .summary("Create API key")
                .auth(AuthLevel::Authenticated),
            // Bootstrap token redemption (filled in Task 6)
            BlockEndpoint::get("/b/auth/bootstrap").summary("Bootstrap token redemption form"),
            BlockEndpoint::post("/b/auth/api/bootstrap").summary("Redeem bootstrap admin token"),
        ])
        .config_keys(config_vars())
        .admin_url("/b/auth/admin/settings")
    }

    async fn handle(&self, ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
        let action = msg.action().to_string();
        // Normalize: /b/auth/... → /auth/...
        let raw_path = msg.path().to_string();
        let path = if let Some(stripped) = raw_path.strip_prefix("/b") {
            stripped.to_string()
        } else {
            raw_path
        };

        // Apply per-user/IP rate limiting via the declarative RATE_LIMIT_ROUTES
        // table (see its doc comment for the rule set).
        //
        // `RateLimitOutcome::Allowed(headers)` is discarded here — injecting
        // X-RateLimit-* response headers needs a streaming-middleware shape we
        // don't have yet. Tracked as a single follow-up, not a per-route TODO.
        if let Some(RateLimitOutcome::Limited(r)) = check_route_limits(
            &self.limiter,
            ctx,
            &msg,
            action.as_str(),
            path.as_str(),
            RATE_LIMIT_ROUTES,
        )
        .await
        {
            return r;
        }

        match (action.as_str(), path.as_str()) {
            // ── Admin settings ───────────────────────────────────────
            // Admin tier enforced centrally from the declared
            // `AuthLevel::Admin` on `GET|POST /b/auth/admin/settings`.
            ("retrieve", "/auth/admin/settings") => pages::settings::handle_get(ctx, &msg).await,
            ("create", "/auth/admin/settings") => pages::settings::handle_post(ctx, input).await,
            // ── SSR pages (HTML) ──────────────────────────────────────
            ("retrieve", "/auth/login") => pages::login::handle(ctx, &msg).await,
            ("retrieve", "/auth/signup") => pages::signup::handle(ctx, &msg).await,
            ("retrieve", "/auth/change-password") => {
                if msg.user_id().is_empty() {
                    return pages::login::handle(ctx, &msg).await;
                }
                pages::change_password::handle(ctx, &msg).await
            }
            ("retrieve", "/auth/orgs") => pages::orgs::handle(ctx, &msg).await,
            ("retrieve", "/auth/reset-password") => pages::reset_password::handle(ctx, &msg).await,
            // Bootstrap token redemption (NEW — filled in Task 6)
            ("retrieve", "/auth/bootstrap") => pages::bootstrap::handle_get(ctx, &msg).await,
            // OAuth browser redirects
            ("retrieve", "/auth/oauth/login") => oauth::start::handle(ctx, &msg).await,
            ("retrieve", "/auth/oauth/callback") => oauth::callback::handle(ctx, &msg).await,

            // ── JSON API under /auth/api/ ─────────────────────────────
            ("create", "/auth/api/login") => api::login::handle(ctx, input).await,
            ("create", "/auth/api/signup") => api::signup::handle(ctx, input).await,
            ("create", "/auth/api/refresh") => api::refresh::handle(ctx, input).await,
            ("create", "/auth/api/logout") => api::logout::handle(ctx, &msg).await,
            ("retrieve", "/auth/api/me") => api::me::handle_get(ctx, &msg).await,
            ("update", "/auth/api/me") => api::me::handle_update(ctx, &msg, input).await,
            ("create", "/auth/api/change-password") => {
                api::change_password::handle(ctx, &msg, input).await
            }
            // API keys (admin user-management still hits these via htmx)
            ("retrieve", "/auth/api/api-keys") => api::api_keys::handle_list(ctx, &msg).await,
            ("create", "/auth/api/api-keys") => {
                api::api_keys::handle_create(ctx, &msg, input).await
            }
            ("update", p)
                if endpoint_match::match_template("/auth/api/api-keys/{id}", p).is_some() =>
            {
                api::api_keys::handle_revoke(ctx, &msg).await
            }
            ("delete", p)
                if endpoint_match::match_template("/auth/api/api-keys/{id}", p).is_some() =>
            {
                api::api_keys::handle_delete(ctx, &msg).await
            }
            // Email verification
            ("retrieve" | "create", "/auth/api/verify") => {
                api::verify::handle(ctx, &msg, input).await
            }
            ("create", "/auth/api/resend-verification") => {
                api::verify::handle_resend(ctx, input).await
            }
            // Password reset
            ("create", "/auth/api/forgot-password") => {
                api::forgot_password::handle(ctx, input).await
            }
            ("create", "/auth/api/reset-password") => api::reset_password::handle(ctx, input).await,
            // OAuth API
            ("retrieve", "/auth/api/oauth/providers") => oauth::providers::handle(ctx).await,
            ("create", "/auth/api/oauth/sync-user") => {
                api::sync_user::handle(ctx, &msg, input).await
            }
            // Bootstrap admin token redemption (NEW — filled in Task 6)
            ("create", "/auth/api/bootstrap") => api::bootstrap::handle(ctx, input).await,
            _ => err_not_found("not found"),
        }
    }

    async fn lifecycle(
        &self,
        _ctx: &dyn Context,
        _event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}

// PR 5 Task 7: framework AuthBlock now owns `suppers-ai/auth` (the auth
// service primitive); auth-ui owns the `/b/auth/*` HTTP surface.
#[cfg(not(target_arch = "wasm32"))]
::wafer_block::register_static_block!("suppers-ai/auth-ui", AuthUiBlock);
