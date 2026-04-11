mod api_keys;
mod login;
mod oauth;
mod pages;

use super::helpers::{hex_encode, json_map};
use super::rate_limit::{check_rate_limit, RateLimit, UserRateLimiter};
use std::collections::HashMap;
use std::time::Duration;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions};
use wafer_core::clients::{config, crypto};
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

pub struct AuthBlock {
    limiter: UserRateLimiter,
}

impl Default for AuthBlock {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthBlock {
    pub fn new() -> Self {
        Self {
            limiter: UserRateLimiter::new(),
        }
    }
}

pub(crate) const USERS_COLLECTION: &str = "suppers_ai__auth__users";
pub(crate) const RATE_LIMITS_COLLECTION: &str = "suppers_ai__auth__rate_limits";
pub(crate) const TOKENS_COLLECTION: &str = "suppers_ai__auth__tokens";
pub(crate) const API_KEYS_COLLECTION: &str = "suppers_ai__auth__api_keys";

/// Pre-computed Argon2id hash used for timing equalization when user is not found.
const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

use crate::blocks::admin::USER_ROLES_COLLECTION;

// --- Shared helpers used by login.rs, oauth.rs, api_keys.rs ---

mod helpers {
    use super::*;

    /// Create a new user record and assign the default role.
    /// Admin role is granted only if the user's email matches the configured ADMIN_EMAIL.
    pub(super) async fn create_user_and_assign_role(
        ctx: &dyn Context,
        data: HashMap<String, serde_json::Value>,
    ) -> std::result::Result<(wafer_core::clients::database::Record, String), String> {
        let user = db::create(ctx, USERS_COLLECTION, data)
            .await
            .map_err(|e| format!("Failed to create user: {e}"))?;

        let admin_email = config::get_default(ctx, "SUPPERS_AI__AUTH__ADMIN_EMAIL", "").await;
        let user_email = user
            .data
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let role = if !admin_email.is_empty() && user_email.eq_ignore_ascii_case(&admin_email) {
            "admin"
        } else {
            "user"
        };

        let role_data = json_map(serde_json::json!({
            "user_id": user.id,
            "role": role,
            "assigned_at": crate::blocks::helpers::now_rfc3339()
        }));
        if let Err(e) = db::create(ctx, USER_ROLES_COLLECTION, role_data).await {
            tracing::warn!("Failed to assign default role during signup: {e}");
        }

        Ok((user, role.to_string()))
    }

    pub(super) async fn get_user_roles(ctx: &dyn Context, user_id: &str) -> Vec<String> {
        let opts = ListOptions {
            filters: vec![Filter {
                field: "user_id".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String(user_id.to_string()),
            }],
            ..Default::default()
        };
        match db::list(ctx, USER_ROLES_COLLECTION, &opts).await {
            Ok(r) => r
                .records
                .iter()
                .filter_map(|rec| {
                    rec.data
                        .get("role")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Returns (access_token, refresh_token, family).
    pub(super) async fn generate_tokens(
        ctx: &dyn Context,
        user_id: &str,
        email: &str,
        roles: &[String],
    ) -> Result<(String, String, String), Result_> {
        let family = match crypto::random_bytes(ctx, 16).await {
            Ok(bytes) => hex_encode(&bytes),
            Err(e) => return Err(Result_::error(e)),
        };

        let mut access_claims = HashMap::new();
        access_claims.insert(
            "user_id".to_string(),
            serde_json::Value::String(user_id.to_string()),
        );
        access_claims.insert(
            "sub".to_string(),
            serde_json::Value::String(user_id.to_string()),
        );
        access_claims.insert(
            "email".to_string(),
            serde_json::Value::String(email.to_string()),
        );
        access_claims.insert("roles".to_string(), serde_json::json!(roles));
        access_claims.insert(
            "type".to_string(),
            serde_json::Value::String("access".to_string()),
        );

        let access_token = crypto::sign(ctx, &access_claims, Duration::from_secs(86400))
            .await
            .map_err(Result_::error)?;

        let mut refresh_claims = HashMap::new();
        refresh_claims.insert(
            "user_id".to_string(),
            serde_json::Value::String(user_id.to_string()),
        );
        refresh_claims.insert(
            "sub".to_string(),
            serde_json::Value::String(user_id.to_string()),
        );
        refresh_claims.insert(
            "type".to_string(),
            serde_json::Value::String("refresh".to_string()),
        );
        refresh_claims.insert(
            "family".to_string(),
            serde_json::Value::String(family.clone()),
        );

        let refresh_token = crypto::sign(ctx, &refresh_claims, Duration::from_secs(604800))
            .await
            .map_err(Result_::error)?;

        Ok((access_token, refresh_token, family))
    }

    pub(super) async fn store_refresh_token(
        ctx: &dyn Context,
        user_id: &str,
        token: &str,
        family: &str,
    ) {
        let data = json_map(serde_json::json!({
            "user_id": user_id,
            "token": token,
            "family": family,
            "created_at": crate::blocks::helpers::now_rfc3339()
        }));
        if let Err(e) = db::create(ctx, TOKENS_COLLECTION, data).await {
            tracing::warn!("Failed to store refresh token: {e}");
        }
    }

    pub(super) async fn build_auth_cookie(token: &str, max_age: u64, ctx: &dyn Context) -> String {
        let env = config::get_default(ctx, "SOLOBASE_SHARED__ENVIRONMENT", "development").await;
        let secure = env.to_lowercase() != "development";
        format!(
            "auth_token={}; HttpOnly; Path=/; SameSite=Lax; Max-Age={}{}",
            token,
            max_age,
            if secure { "; Secure" } else { "" }
        )
    }

    pub(super) fn urlencode(s: &str) -> String {
        s.as_bytes()
            .iter()
            .map(|&b| match b {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    String::from(b as char)
                }
                _ => format!("%{:02X}", b),
            })
            .collect()
    }
}

/// Seed a default admin user if no users exist yet.
pub async fn seed_admin_user(ctx: &dyn Context) {
    let count = db::count(ctx, USERS_COLLECTION, &[]).await.unwrap_or(0);
    if count > 0 {
        return;
    }

    let admin_email =
        config::get_default(ctx, "SUPPERS_AI__AUTH__ADMIN_EMAIL", "admin@example.com").await;
    let admin_password_env = config::get_default(ctx, "SUPPERS_AI__AUTH__ADMIN_PASSWORD", "").await;

    let (password_to_use, was_randomly_generated) = if !admin_password_env.is_empty() {
        (admin_password_env, false)
    } else {
        let random_bytes = match crypto::random_bytes(ctx, 16).await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Failed to generate random password: {e}");
                return;
            }
        };
        (hex_encode(&random_bytes), true)
    };

    let password_hash = match crypto::hash(ctx, &password_to_use).await {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to hash default admin password: {e}");
            return;
        }
    };

    let mut data = json_map(serde_json::json!({
        "email": admin_email,
        "password_hash": password_hash,
        "name": "Admin",
        "disabled": false
    }));
    super::helpers::stamp_created(&mut data);

    match db::create(ctx, USERS_COLLECTION, data).await {
        Ok(user) => {
            let role_data = json_map(serde_json::json!({
                "user_id": user.id,
                "role": "admin",
                "assigned_at": super::helpers::now_rfc3339()
            }));
            if let Err(e) = db::create(ctx, USER_ROLES_COLLECTION, role_data).await {
                tracing::warn!("Failed to assign admin role to seeded user: {e}");
            }
            tracing::info!("==========================================================");
            tracing::info!("Default admin user seeded:");
            tracing::info!("  Email:    {}", admin_email);
            if was_randomly_generated {
                // Log password to stderr only — never persist in log aggregation
                eprintln!("  ADMIN PASSWORD (one-time display): {}", password_to_use);
                tracing::info!("  Password: (displayed on stderr — CHANGE IMMEDIATELY)");
            } else {
                tracing::info!("  Password: (set via SUPPERS_AI__AUTH__ADMIN_PASSWORD env var)");
            }
            tracing::info!("==========================================================");
        }
        Err(e) => {
            tracing::error!("Failed to seed admin user: {e}");
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for AuthBlock {
    fn info(&self) -> BlockInfo {
        use wafer_run::types::CollectionSchema;
        use wafer_run::AuthLevel;

        BlockInfo::new("suppers-ai/auth", "0.0.1", "http-handler@v1", "Authentication: login, signup, JWT, refresh tokens, OAuth, API keys")
            .instance_mode(InstanceMode::Singleton)
            .requires(vec!["wafer-run/database".into(), "wafer-run/crypto".into(), "wafer-run/config".into(), "suppers-ai/email".into()])
            .collections(vec![
                CollectionSchema::new(USERS_COLLECTION)
                    .field_unique("email", "string")
                    .field_default("password_hash", "string", "")
                    .field_default("name", "string", "")
                    .field_default("disabled", "bool", "false")
                    .field_default("avatar_url", "string", "")
                    .field_default("oauth_provider", "string", "")
                    .field_default("email_verified", "bool", "false")
                    .field_default("verification_token", "string", "")
                    .field_default("reset_token", "string", "")
                    .field_optional("reset_token_expires", "datetime")
                    .field_optional("last_verification_sent", "datetime")
                    .field_optional("last_login_at", "datetime")
                    .field_optional("deleted_at", "datetime"),
                CollectionSchema::new(TOKENS_COLLECTION)
                    .field_ref("user_id", "string", &format!("{}.id", USERS_COLLECTION))
                    .field("token", "string")
                    .index(&["user_id"]),
                CollectionSchema::new(API_KEYS_COLLECTION)
                    .field_ref("user_id", "string", &format!("{}.id", USERS_COLLECTION))
                    .field_default("name", "string", "")
                    .field("key_hash", "string")
                    .field_default("key_prefix", "string", "")
                    .field_optional("last_used", "datetime")
                    .field_optional("revoked_at", "datetime")
                    .field_optional("expires_at", "datetime")
                    .index(&["user_id"]),
                CollectionSchema::new(RATE_LIMITS_COLLECTION)
                    .field_unique("key", "string")
                    .field_default("count", "int", "0")
                    .field_default("window_start", "int", "0"),
            ])
            .grants(vec![
                wafer_run::ResourceGrant::read("suppers-ai/admin", "suppers_ai__auth__*"),
                wafer_run::ResourceGrant::read_write("suppers-ai/userportal", USERS_COLLECTION),
                wafer_run::ResourceGrant::read("suppers-ai/products", USERS_COLLECTION),
            ])
            .category(wafer_run::BlockCategory::Feature)
            .description("Handles user authentication, registration, and session management. Supports email/password login, OAuth providers (Google, GitHub, Microsoft), email verification, password reset, and API key management.")
            .endpoints(vec![
                // SSR pages
                BlockEndpoint::get("/b/auth/login").summary("Login page"),
                BlockEndpoint::get("/b/auth/signup").summary("Signup page"),
                BlockEndpoint::get("/b/auth/change-password").summary("Change password page").auth(AuthLevel::Authenticated),
                BlockEndpoint::get("/b/auth/oauth/login").summary("Start OAuth flow"),
                // JSON API
                BlockEndpoint::post("/b/auth/api/login").summary("Authenticate with email/password"),
                BlockEndpoint::post("/b/auth/api/signup").summary("Create account"),
                BlockEndpoint::post("/b/auth/api/logout").summary("Sign out").auth(AuthLevel::Authenticated),
                BlockEndpoint::get("/b/auth/api/me").summary("Get current user").auth(AuthLevel::Authenticated),
                BlockEndpoint::post("/b/auth/api/change-password").summary("Change password").auth(AuthLevel::Authenticated),
                BlockEndpoint::get("/b/auth/api/api-keys").summary("List API keys").auth(AuthLevel::Authenticated),
                BlockEndpoint::post("/b/auth/api/api-keys").summary("Create API key").auth(AuthLevel::Authenticated),
            ])
            .config_keys(vec![
                ConfigVar::new("SUPPERS_AI__AUTH__JWT_SECRET", "Secret key for signing auth tokens", "")
                    .name("JWT Secret")
                    .input_type(InputType::Password)
                    .auto_generate()
                    .warning("Changing this will invalidate all existing user sessions"),
                ConfigVar::new("SUPPERS_AI__AUTH__REQUIRE_VERIFICATION", "Require email verification before login", "false")
                    .name("Require Email Verification")
                    .input_type(InputType::Toggle),
                ConfigVar::new("SUPPERS_AI__AUTH__ALLOWED_EMAIL_DOMAINS", "Restrict signup to specific email domains (comma-separated)", "")
                    .name("Allowed Email Domains"),
                ConfigVar::new("SUPPERS_AI__AUTH__ADMIN_EMAIL", "Email address that gets the admin role on signup", "")
                    .name("Admin Email"),
                ConfigVar::new("SUPPERS_AI__AUTH__OAUTH_GOOGLE_CLIENT_ID", "Google OAuth client ID", "")
                    .name("Google Client ID"),
                ConfigVar::new("SUPPERS_AI__AUTH__OAUTH_GOOGLE_CLIENT_SECRET", "Google OAuth client secret", "")
                    .name("Google Client Secret")
                    .input_type(InputType::Password),
                ConfigVar::new("SUPPERS_AI__AUTH__OAUTH_GITHUB_CLIENT_ID", "GitHub OAuth client ID", "")
                    .name("GitHub Client ID"),
                ConfigVar::new("SUPPERS_AI__AUTH__OAUTH_GITHUB_CLIENT_SECRET", "GitHub OAuth client secret", "")
                    .name("GitHub Client Secret")
                    .input_type(InputType::Password),
                ConfigVar::new("SUPPERS_AI__AUTH__OAUTH_MICROSOFT_CLIENT_ID", "Microsoft OAuth client ID", "")
                    .name("Microsoft Client ID"),
                ConfigVar::new("SUPPERS_AI__AUTH__OAUTH_MICROSOFT_CLIENT_SECRET", "Microsoft OAuth client secret", "")
                    .name("Microsoft Client Secret")
                    .input_type(InputType::Password),
                ConfigVar::new("SUPPERS_AI__AUTH__ADMIN_PASSWORD", "Password for the default admin account", "")
                    .name("Admin Password")
                    .input_type(InputType::Password),
                ConfigVar::new("SUPPERS_AI__AUTH__INTERNAL_SECRET", "Secret for internal API authentication", "")
                    .name("Internal Secret")
                    .input_type(InputType::Password),
                ConfigVar::new("SUPPERS_AI__AUTH__OAUTH_REDIRECT_URI", "OAuth callback URL", "")
                    .name("OAuth Redirect URI")
                    .input_type(InputType::Url),
            ])
            .admin_url("/b/auth/admin/settings")
    }

    fn ui_routes(&self) -> Vec<wafer_run::UiRoute> {
        vec![
            wafer_run::UiRoute::public("/login"),
            wafer_run::UiRoute::public("/signup"),
            wafer_run::UiRoute::authenticated("/change-password"),
            wafer_run::UiRoute::admin("/admin/settings"),
        ]
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let action = msg.action().to_string();
        // Normalize: /b/auth/... → /auth/...
        let raw_path = msg.path().to_string();
        let path = if let Some(stripped) = raw_path.strip_prefix("/b") {
            stripped.to_string()
        } else {
            raw_path
        };

        // Apply per-user/IP rate limiting based on endpoint category
        match (action.as_str(), path.as_str()) {
            // Unauthenticated sensitive endpoints: rate limit by IP
            ("create", "/auth/api/login") | ("create", "/auth/api/signup") => {
                let ip = msg.remote_addr().to_string();
                let identity = if ip.is_empty() {
                    "unknown".to_string()
                } else {
                    ip
                };
                if let Some(r) =
                    check_rate_limit(&self.limiter, ctx, msg, &identity, "auth", RateLimit::AUTH)
                        .await
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
                if let Some(r) = check_rate_limit(
                    &self.limiter,
                    ctx,
                    msg,
                    &identity,
                    "refresh",
                    RateLimit::REFRESH,
                )
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
                if let Some(r) =
                    check_rate_limit(&self.limiter, ctx, msg, &identity, "auth", RateLimit::AUTH)
                        .await
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
                    if let Some(r) = check_rate_limit(
                        &self.limiter,
                        ctx,
                        msg,
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
                    if let Some(r) = check_rate_limit(
                        &self.limiter,
                        ctx,
                        msg,
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
                if !msg
                    .get_meta("auth.user_roles")
                    .split(',')
                    .any(|r| r.trim() == "admin")
                {
                    return crate::ui::forbidden_response(msg);
                }
                pages::settings_page(ctx, msg).await
            }
            ("create", "/auth/admin/settings") => {
                if !msg
                    .get_meta("auth.user_roles")
                    .split(',')
                    .any(|r| r.trim() == "admin")
                {
                    return crate::ui::forbidden_response(msg);
                }
                pages::handle_save_settings(ctx, msg).await
            }
            // ── SSR pages (HTML) ──────────────────────────────────────
            ("retrieve", "/auth/login") => pages::login_page(ctx, msg).await,
            ("retrieve", "/auth/signup") => pages::signup_page(ctx, msg).await,
            ("retrieve", "/auth/change-password") => {
                if msg.user_id().is_empty() {
                    return pages::login_page(ctx, msg).await;
                }
                pages::change_password_page(ctx, msg).await
            }
            ("retrieve", "/auth/reset-password") => self.handle_reset_password_form(ctx, msg).await,
            // OAuth browser redirects
            ("retrieve", "/auth/oauth/login") => self.handle_oauth_login(ctx, msg).await,
            ("retrieve", "/auth/oauth/callback") => self.handle_oauth_callback(ctx, msg).await,

            // ── JSON API under /auth/api/ ─────────────────────────────
            ("create", "/auth/api/login") => self.handle_login(ctx, msg).await,
            ("create", "/auth/api/signup") => self.handle_signup(ctx, msg).await,
            ("create", "/auth/api/refresh") => self.handle_refresh(ctx, msg).await,
            ("create", "/auth/api/logout") => self.handle_logout(ctx, msg).await,
            ("retrieve", "/auth/api/me") => self.handle_me_get(ctx, msg).await,
            ("update", "/auth/api/me") => self.handle_me_update(ctx, msg).await,
            ("create", "/auth/api/change-password") => self.handle_change_password(ctx, msg).await,
            // API keys
            ("retrieve", "/auth/api/api-keys") => self.handle_api_keys_list(ctx, msg).await,
            ("create", "/auth/api/api-keys") => self.handle_api_keys_create(ctx, msg).await,
            ("update", _) if path.starts_with("/auth/api/api-keys/") => {
                self.handle_api_keys_revoke(ctx, msg).await
            }
            ("delete", _) if path.starts_with("/auth/api/api-keys/") => {
                self.handle_api_keys_delete(ctx, msg).await
            }
            // Email verification
            ("retrieve" | "create", "/auth/api/verify") => self.handle_verify_email(ctx, msg).await,
            ("create", "/auth/api/resend-verification") => {
                self.handle_resend_verification(ctx, msg).await
            }
            // Password reset
            ("create", "/auth/api/forgot-password") => self.handle_forgot_password(ctx, msg).await,
            ("create", "/auth/api/reset-password") => self.handle_reset_password(ctx, msg).await,
            // OAuth API
            ("retrieve", "/auth/api/oauth/providers") => {
                self.handle_oauth_providers(ctx, msg).await
            }
            ("create", "/auth/api/oauth/sync-user") => self.handle_sync_user(ctx, msg).await,
            _ => err_not_found(msg, "not found"),
        }
    }

    async fn lifecycle(
        &self,
        ctx: &dyn Context,
        event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        if matches!(event.event_type, LifecycleType::Init) {
            seed_admin_user(ctx).await;
        }
        Ok(())
    }
}

/// Authenticate a request using an API key.
///
/// Hashes the key with SHA-256, looks it up in the database by key_hash,
/// checks it's not revoked/expired, and sets auth meta on the message.
/// Silently does nothing if the key is invalid (request continues as
/// unauthenticated), matching JWT behavior.
pub async fn authenticate_api_key(
    ctx: &dyn wafer_run::context::Context,
    api_key: &str,
    msg: &mut wafer_run::types::Message,
) {
    use crate::blocks::helpers::{sha256_hex, RecordExt};
    use wafer_core::clients::database as db;
    use wafer_run::meta::*;

    let key_hash = sha256_hex(api_key.as_bytes());

    // Look up by key_hash
    let key_record = match db::get_by_field(
        ctx,
        API_KEYS_COLLECTION,
        "key_hash",
        serde_json::Value::String(key_hash),
    )
    .await
    {
        Ok(r) => r,
        Err(_) => return,
    };

    // Check if revoked
    let revoked_at = key_record.str_field("revoked_at");
    if !revoked_at.is_empty() {
        return;
    }

    // Check if expired
    let expires_at = key_record.str_field("expires_at");
    if !expires_at.is_empty() {
        let now = crate::blocks::helpers::now_rfc3339();
        if now > expires_at.to_string() {
            return;
        }
    }

    // Look up the user to get email and roles
    let user_id = key_record.str_field("user_id");
    if user_id.is_empty() {
        return;
    }

    let user = match db::get(ctx, USERS_COLLECTION, user_id).await {
        Ok(u) => u,
        Err(_) => return,
    };

    // Fetch roles from user_roles collection (roles are not stored on the user record)
    let roles = helpers::get_user_roles(ctx, user_id).await;
    let roles_str = roles.join(",");

    // Set auth meta (same fields as JWT auth)
    msg.set_meta(META_AUTH_USER_ID, user_id);
    msg.set_meta(META_AUTH_USER_EMAIL, user.str_field("email"));
    msg.set_meta(META_AUTH_USER_ROLES, &roles_str);
}
