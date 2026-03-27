mod login;
mod api_keys;
mod oauth;
mod pages;

use std::collections::HashMap;
use std::time::Duration;
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions};
use wafer_core::clients::{crypto, config};
use super::helpers::{hex_encode, json_map};
use super::rate_limit::{UserRateLimiter, RateLimit, check_rate_limit};

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
        Self { limiter: UserRateLimiter::new() }
    }
}

const USERS_COLLECTION: &str = "auth_users";
const TOKENS_COLLECTION: &str = "auth_tokens";
const API_KEYS_COLLECTION: &str = "api_keys";
const USER_ROLES_COLLECTION: &str = "iam_user_roles";

// --- Shared helpers used by login.rs, oauth.rs, api_keys.rs ---

mod helpers {
    use super::*;

    /// Create a new user record and assign the default role.
    /// Admin role is granted only if the user's email matches the configured ADMIN_EMAIL.
    pub(super) async fn create_user_and_assign_role(
        ctx: &dyn Context,
        data: HashMap<String, serde_json::Value>,
    ) -> std::result::Result<(wafer_core::clients::database::Record, String), String> {
        let user = db::create(ctx, USERS_COLLECTION, data).await
            .map_err(|e| format!("Failed to create user: {e}"))?;

        let admin_email = config::get_default(ctx, "ADMIN_EMAIL", "").await;
        let user_email = user.data.get("email").and_then(|v| v.as_str()).unwrap_or("");
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
            Ok(r) => r.records.iter()
                .filter_map(|rec| rec.data.get("role").and_then(|v| v.as_str()).map(|s| s.to_string()))
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    pub(super) async fn generate_tokens(
        ctx: &dyn Context,
        user_id: &str,
        email: &str,
        roles: &[String],
    ) -> Result<(String, String), Result_> {
        let family = match crypto::random_bytes(ctx, 16).await {
            Ok(bytes) => hex_encode(&bytes),
            Err(e) => return Err(Result_::error(e)),
        };

        let mut access_claims = HashMap::new();
        access_claims.insert("user_id".to_string(), serde_json::Value::String(user_id.to_string()));
        access_claims.insert("sub".to_string(), serde_json::Value::String(user_id.to_string()));
        access_claims.insert("email".to_string(), serde_json::Value::String(email.to_string()));
        access_claims.insert("roles".to_string(), serde_json::json!(roles));
        access_claims.insert("type".to_string(), serde_json::Value::String("access".to_string()));

        let access_token = crypto::sign(ctx, &access_claims, Duration::from_secs(86400)).await
            .map_err(Result_::error)?;

        let mut refresh_claims = HashMap::new();
        refresh_claims.insert("user_id".to_string(), serde_json::Value::String(user_id.to_string()));
        refresh_claims.insert("sub".to_string(), serde_json::Value::String(user_id.to_string()));
        refresh_claims.insert("type".to_string(), serde_json::Value::String("refresh".to_string()));
        refresh_claims.insert("family".to_string(), serde_json::Value::String(family));

        let refresh_token = crypto::sign(ctx, &refresh_claims, Duration::from_secs(604800)).await
            .map_err(Result_::error)?;

        Ok((access_token, refresh_token))
    }

    pub(super) async fn store_refresh_token(ctx: &dyn Context, user_id: &str, token: &str) {
        let data = json_map(serde_json::json!({
            "user_id": user_id,
            "token": token,
            "created_at": crate::blocks::helpers::now_rfc3339()
        }));
        if let Err(e) = db::create(ctx, TOKENS_COLLECTION, data).await {
            tracing::warn!("Failed to store refresh token: {e}");
        }
    }

    pub(super) async fn build_auth_cookie(token: &str, max_age: u64, ctx: &dyn Context) -> String {
        let env = config::get_default(ctx, "ENVIRONMENT", "development").await;
        let secure = env == "production";
        format!(
            "auth_token={}; HttpOnly; Path=/; SameSite=Lax; Max-Age={}{}",
            token, max_age, if secure { "; Secure" } else { "" }
        )
    }

    pub(super) fn urlencode(s: &str) -> String {
        s.as_bytes().iter().map(|&b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                String::from(b as char)
            }
            _ => format!("%{:02X}", b),
        }).collect()
    }
}

/// Seed a default admin user if no users exist yet.
pub async fn seed_admin_user(ctx: &dyn Context) {
    let count = db::count(ctx, USERS_COLLECTION, &[]).await.unwrap_or(0);
    if count > 0 {
        return;
    }

    let admin_email = config::get_default(ctx, "ADMIN_EMAIL", "admin@example.com").await;
    let admin_password_env = config::get_default(ctx, "ADMIN_PASSWORD", "").await;

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
                tracing::info!("  Password: {}", password_to_use);
                tracing::info!("  CHANGE THIS PASSWORD IMMEDIATELY!");
            } else {
                tracing::info!("  Password: (set via ADMIN_PASSWORD env var)");
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

        BlockInfo {
            name: "suppers-ai/auth".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "Authentication: login, signup, JWT, refresh tokens, OAuth, API keys".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
            runtime: wafer_run::types::BlockRuntime::Native,
            requires: Vec::new(),
            collections: vec![
                CollectionSchema::new("auth_users")
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
                CollectionSchema::new("auth_tokens")
                    .field_ref("user_id", "string", "auth_users.id")
                    .field("token", "string")
                    .index(&["user_id"]),
                CollectionSchema::new("api_keys")
                    .field_ref("user_id", "string", "auth_users.id")
                    .field_default("name", "string", "")
                    .field("key_hash", "string")
                    .field_default("key_prefix", "string", "")
                    .field_optional("last_used", "datetime")
                    .field_optional("revoked_at", "datetime")
                    .field_optional("expires_at", "datetime")
                    .index(&["user_id"]),
            ],
        }
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let action = msg.action().to_string();
        let path = msg.path().to_string();

        // Apply per-user/IP rate limiting based on endpoint category
        match (action.as_str(), path.as_str()) {
            // Unauthenticated sensitive endpoints: rate limit by IP
            ("create", "/auth/login") | ("create", "/auth/signup") => {
                let ip = msg.remote_addr().to_string();
                let identity = if ip.is_empty() { "unknown".to_string() } else { ip };
                if let Some(r) = check_rate_limit(&self.limiter, ctx, msg, &identity, "auth", RateLimit::AUTH).await {
                    return r;
                }
            }
            ("create", "/auth/refresh") => {
                let ip = msg.remote_addr().to_string();
                let identity = if ip.is_empty() { "unknown".to_string() } else { ip };
                if let Some(r) = check_rate_limit(&self.limiter, ctx, msg, &identity, "refresh", RateLimit::REFRESH).await {
                    return r;
                }
            }
            // Authenticated write endpoints: rate limit by user_id
            ("update", _) | ("create", "/auth/change-password") | ("create", "/auth/api-keys") | ("delete", _) => {
                let user_id = msg.user_id().to_string();
                if !user_id.is_empty() {
                    if let Some(r) = check_rate_limit(&self.limiter, ctx, msg, &user_id, "auth_write", RateLimit::API_WRITE).await {
                        return r;
                    }
                }
            }
            // Authenticated read endpoints: rate limit by user_id
            ("retrieve", "/auth/me") | ("retrieve", "/auth/api-keys") => {
                let user_id = msg.user_id().to_string();
                if !user_id.is_empty() {
                    if let Some(r) = check_rate_limit(&self.limiter, ctx, msg, &user_id, "auth_read", RateLimit::API_READ).await {
                        return r;
                    }
                }
            }
            _ => {}
        }

        match (action.as_str(), path.as_str()) {
            // SSR auth pages
            ("retrieve", "/auth/login") => pages::login_page(ctx, msg).await,
            ("retrieve", "/auth/signup") => pages::signup_page(ctx, msg).await,
            ("retrieve", "/auth/change-password") => {
                if msg.user_id().is_empty() {
                    return pages::login_page(ctx, msg).await;
                }
                pages::change_password_page(ctx, msg).await
            }
            // API endpoints
            ("create", "/auth/login") => self.handle_login(ctx, msg).await,
            ("create", "/auth/signup") => self.handle_signup(ctx, msg).await,
            ("create", "/auth/refresh") => self.handle_refresh(ctx, msg).await,
            ("create", "/auth/logout") => self.handle_logout(ctx, msg).await,
            ("retrieve", "/auth/me") => self.handle_me_get(ctx, msg).await,
            ("update", "/auth/me") => self.handle_me_update(ctx, msg).await,
            ("create", "/auth/change-password") => self.handle_change_password(ctx, msg).await,
            // API keys
            ("retrieve", "/auth/api-keys") => self.handle_api_keys_list(ctx, msg).await,
            ("create", "/auth/api-keys") => self.handle_api_keys_create(ctx, msg).await,
            ("update", _) if path.starts_with("/auth/api-keys/") => self.handle_api_keys_revoke(ctx, msg).await,
            ("delete", _) if path.starts_with("/auth/api-keys/") => self.handle_api_keys_delete(ctx, msg).await,
            // Email verification
            ("retrieve" | "create", "/auth/verify") => self.handle_verify_email(ctx, msg).await,
            ("create", "/auth/resend-verification") => self.handle_resend_verification(ctx, msg).await,
            // Password reset
            ("create", "/auth/forgot-password") => self.handle_forgot_password(ctx, msg).await,
            ("retrieve", "/auth/reset-password") => self.handle_reset_password_form(ctx, msg).await,
            ("create", "/auth/reset-password") => self.handle_reset_password(ctx, msg).await,
            // OAuth
            ("retrieve", "/auth/oauth/providers") => self.handle_oauth_providers(ctx, msg).await,
            ("retrieve", "/auth/oauth/login") => self.handle_oauth_login(ctx, msg).await,
            ("retrieve", "/auth/oauth/callback") => self.handle_oauth_callback(ctx, msg).await,
            // Internal
            ("create", "/internal/oauth/sync-user") => self.handle_sync_user(ctx, msg).await,
            _ => err_not_found(msg, "not found"),
        }
    }

    async fn lifecycle(&self, ctx: &dyn Context, event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        if matches!(event.event_type, LifecycleType::Init) {
            seed_admin_user(ctx).await;
        }
        Ok(())
    }
}
