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
//! This file is the **scaffold** committed in Task 4. It declares the full
//! `BlockInfo` (endpoints, ui_routes, requires, OAuth-creds config_keys),
//! ports the rate-limit middleware preamble verbatim from `auth/mod.rs`,
//! and dispatches every route to a leaf module under `api/`, `pages/`, or
//! `oauth/` whose handler currently panics with `unimplemented!()`. The
//! handler bodies are relocated from `auth/` in Task 5.
//!
//! Static-block registration is intentionally left commented out at the
//! bottom of this file — it gets enabled in Task 7, once the framework
//! `AuthBlock` takes over `/b/auth/*`. Until then, the live `auth` block
//! still owns those paths and registering this stub would shadow it.

pub mod api;
pub mod oauth;
pub mod pages;

use wafer_run::{
    block::{Block, BlockInfo},
    context::Context,
    types::*,
    InputStream, OutputStream,
};

use super::rate_limit::{check_rate_limit, RateLimit, RateLimitOutcome, UserRateLimiter};
use crate::blocks::helpers::err_not_found;

pub const AUTH_UI_BLOCK_ID: &str = "suppers-ai/auth-ui";

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
            // SSR pages
            BlockEndpoint::get("/b/auth/login").summary("Login page"),
            BlockEndpoint::get("/b/auth/signup").summary("Signup page"),
            BlockEndpoint::get("/b/auth/change-password")
                .summary("Change password page")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/auth/dashboard")
                .summary("Portal home")
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
        .config_keys({
            // OAuth provider creds belong with the OAuth UI flows, so they
            // live on auth-ui. The framework AuthBlock keeps JWT_SECRET,
            // REQUIRE_VERIFICATION, ALLOWED_EMAIL_DOMAINS, INTERNAL_SECRET
            // — those are auth identity infra used by AuthServiceImpl, not
            // UI. Note: framework AuthBlock currently doesn't expose
            // config_keys at all (declared in upstream wafer-run); that gap
            // is not fixed in this task. Those vars still resolve via env
            // for now.
            vec![
                ConfigVar::new(
                    "SUPPERS_AI__AUTH__OAUTH_GOOGLE_CLIENT_ID",
                    "Google OAuth client ID",
                    "",
                )
                .name("Google Client ID")
                .optional(),
                ConfigVar::new(
                    "SUPPERS_AI__AUTH__OAUTH_GOOGLE_CLIENT_SECRET",
                    "Google OAuth client secret",
                    "",
                )
                .name("Google Client Secret")
                .input_type(InputType::Password)
                .optional(),
                ConfigVar::new(
                    "SUPPERS_AI__AUTH__OAUTH_GITHUB_CLIENT_ID",
                    "GitHub OAuth client ID",
                    "",
                )
                .name("GitHub Client ID")
                .optional(),
                ConfigVar::new(
                    "SUPPERS_AI__AUTH__OAUTH_GITHUB_CLIENT_SECRET",
                    "GitHub OAuth client secret",
                    "",
                )
                .name("GitHub Client Secret")
                .input_type(InputType::Password)
                .optional(),
                ConfigVar::new(
                    "SUPPERS_AI__AUTH__OAUTH_MICROSOFT_CLIENT_ID",
                    "Microsoft OAuth client ID",
                    "",
                )
                .name("Microsoft Client ID")
                .optional(),
                ConfigVar::new(
                    "SUPPERS_AI__AUTH__OAUTH_MICROSOFT_CLIENT_SECRET",
                    "Microsoft OAuth client secret",
                    "",
                )
                .name("Microsoft Client Secret")
                .input_type(InputType::Password)
                .optional(),
                ConfigVar::new(
                    "SUPPERS_AI__AUTH__OAUTH_REDIRECT_URI",
                    "OAuth callback URL",
                    "",
                )
                .name("OAuth Redirect URI")
                .input_type(InputType::Url)
                .optional(),
            ]
        })
        .admin_url("/b/auth/admin/settings")
    }

    fn ui_routes(&self) -> Vec<wafer_run::UiRoute> {
        vec![
            wafer_run::UiRoute::public("/login"),
            wafer_run::UiRoute::public("/signup"),
            wafer_run::UiRoute::authenticated("/change-password"),
            wafer_run::UiRoute::authenticated("/dashboard"),
            wafer_run::UiRoute::authenticated("/orgs"),
            wafer_run::UiRoute::admin("/admin/settings"),
        ]
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

        // Apply per-user/IP rate limiting based on endpoint category.
        // Ported verbatim from auth/mod.rs:434-524.
        match (action.as_str(), path.as_str()) {
            // Unauthenticated sensitive endpoints: rate limit by IP
            ("create", "/auth/api/login") | ("create", "/auth/api/signup") => {
                let ip = msg.remote_addr().to_string();
                let identity = if ip.is_empty() {
                    "unknown".to_string()
                } else {
                    ip
                };
                // TODO: RateLimitOutcome::Allowed(headers) is currently discarded.
                // Injecting X-RateLimit-* headers requires a streaming middleware
                // pattern that doesn't exist yet.
                if let RateLimitOutcome::Limited(r) =
                    check_rate_limit(&self.limiter, ctx, &identity, "auth", RateLimit::AUTH).await
                {
                    return r;
                }
            }
            ("create", "/auth/api/refresh") => {
                let ip = msg.remote_addr().to_string();
                let identity = if ip.is_empty() {
                    "unknown".to_string()
                } else {
                    ip
                };
                // TODO: Allowed(headers) discarded — needs streaming middleware to inject.
                if let RateLimitOutcome::Limited(r) =
                    check_rate_limit(&self.limiter, ctx, &identity, "refresh", RateLimit::REFRESH)
                        .await
                {
                    return r;
                }
            }
            // Forgot/reset password + verification: rate limit by IP
            ("create", "/auth/api/forgot-password")
            | ("create", "/auth/api/reset-password")
            | ("create", "/auth/api/resend-verification")
            | ("retrieve" | "create", "/auth/api/verify") => {
                let ip = msg.remote_addr().to_string();
                let identity = if ip.is_empty() {
                    "unknown".to_string()
                } else {
                    ip
                };
                // TODO: Allowed(headers) discarded — needs streaming middleware to inject.
                if let RateLimitOutcome::Limited(r) =
                    check_rate_limit(&self.limiter, ctx, &identity, "auth", RateLimit::AUTH).await
                {
                    return r;
                }
            }
            // Authenticated write endpoints: rate limit by user_id
            ("update", _)
            | ("create", "/auth/api/change-password")
            | ("create", "/auth/api/api-keys")
            | ("delete", _) => {
                let user_id = msg.user_id().to_string();
                if !user_id.is_empty() {
                    // TODO: Allowed(headers) discarded — needs streaming middleware to inject.
                    if let RateLimitOutcome::Limited(r) = check_rate_limit(
                        &self.limiter,
                        ctx,
                        &user_id,
                        "auth_write",
                        RateLimit::API_WRITE,
                    )
                    .await
                    {
                        return r;
                    }
                }
            }
            // Authenticated read endpoints: rate limit by user_id
            ("retrieve", "/auth/api/me") | ("retrieve", "/auth/api/api-keys") => {
                let user_id = msg.user_id().to_string();
                if !user_id.is_empty() {
                    // TODO: Allowed(headers) discarded — needs streaming middleware to inject.
                    if let RateLimitOutcome::Limited(r) = check_rate_limit(
                        &self.limiter,
                        ctx,
                        &user_id,
                        "auth_read",
                        RateLimit::API_READ,
                    )
                    .await
                    {
                        return r;
                    }
                }
            }
            _ => {}
        }

        match (action.as_str(), path.as_str()) {
            // ── Admin settings ───────────────────────────────────────
            ("retrieve", "/auth/admin/settings") => {
                if !crate::blocks::helpers::is_admin(&msg) {
                    return crate::ui::forbidden_response(&msg);
                }
                pages::settings::handle_get(ctx, &msg).await
            }
            ("create", "/auth/admin/settings") => {
                if !crate::blocks::helpers::is_admin(&msg) {
                    return crate::ui::forbidden_response(&msg);
                }
                pages::settings::handle_post(ctx, input).await
            }
            // ── SSR pages (HTML) ──────────────────────────────────────
            ("retrieve", "/auth/login") => pages::login::handle(ctx, &msg).await,
            ("retrieve", "/auth/signup") => pages::signup::handle(ctx, &msg).await,
            ("retrieve", "/auth/change-password") => {
                if msg.user_id().is_empty() {
                    return pages::login::handle(ctx, &msg).await;
                }
                pages::change_password::handle(ctx, &msg).await
            }
            ("retrieve", "/auth/dashboard") => pages::dashboard::handle(ctx, &msg).await,
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
            ("update", _) if path.starts_with("/auth/api/api-keys/") => {
                api::api_keys::handle_revoke(ctx, &msg).await
            }
            ("delete", _) if path.starts_with("/auth/api/api-keys/") => {
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

// Static-block registration is enabled in Task 7 once the framework
// AuthBlock takes over /b/auth/* routes. Until then, this stub auth_ui
// would shadow the live auth block (both blocks would declare the same
// /b/auth/* endpoints), and every dispatch arm panics on hit.
// #[cfg(not(target_arch = "wasm32"))]
// ::wafer_run::register_static_block!("suppers-ai/auth-ui", AuthUiBlock);
