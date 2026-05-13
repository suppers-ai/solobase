//! `suppers-ai/auth` — service module.
//!
//! Plan A2 PR 5 split the old monolithic `AuthBlock` in two:
//!
//! - The framework `wafer_core::service_blocks::auth::AuthBlock` wraps
//!   `service::AuthServiceImpl` and owns the `suppers-ai/auth` block id
//!   (registered via `crate::blocks::register_auth`). It has no HTTP routes.
//! - `crate::blocks::auth_ui::AuthUiBlock` owns every `/b/auth/*` HTTP
//!   route (login, signup, OAuth, API keys, settings, dashboard, orgs, …).
//!
//! What lives in this module after the split:
//!
//! - Module decls for the supporting layers (`bootstrap`, `cache`, `config`,
//!   `migrations`, `pat`, `providers`, `repo`, `service`, `session`).
//! - Constants other blocks still reference (`AUTH_BLOCK_ID`, `JWT_SECRET_KEY`,
//!   the four `*_TABLE` re-exports from `repo/{api_keys,rate_limits,tokens,
//!   users}.rs`, `DUMMY_HASH`).
//! - `helpers` — token/cookie/role utilities consumed by `auth_ui::api::*`.
//! - `brand_panel` — shared UI panel consumed by `auth_ui::pages::*`.
//! - `authenticate_api_key` — called by `crate::pipeline` to populate auth
//!   meta from an `Authorization: Bearer <api-key>` header.

pub mod bootstrap;
pub mod cache;
pub mod config;
pub mod migrations;
pub mod pat;
pub mod repo;
pub mod service;
pub mod session;

use std::{collections::HashMap, time::Duration};

use wafer_core::clients::{
    config as config_client, crypto,
    database::{self as db, Filter, FilterOp},
};

use super::helpers::{hex_encode, json_map};

pub const AUTH_BLOCK_ID: &str = "suppers-ai/auth";

/// Config key for the JWT signing secret used by the auth block.
/// Owner: the `suppers-ai/auth` block. Read by the SolobaseRouter
/// for token validation and by the Cloudflare adapter to seed the
/// crypto service.
pub const JWT_SECRET_KEY: &str = "SUPPERS_AI__AUTH__JWT_SECRET";

// Cross-block table-name re-exports. Each auth table is owned by its repo
// module (`repo/users.rs`, `repo/tokens.rs`, etc.). These aliases keep
// existing crate-local consumers (admin/, userportal/, products/,
// rate_limit/, auth_ui/api/*) on stable identifiers without forcing them
// to import the qualified `repo::*::TABLE` path.
// Only consumer is `rate_limit::UserRateLimiter::check` on wasm32; native
// code path doesn't reference it. Re-export separately so we can attach
// the dead-code allow on the import binding.
#[allow(unused_imports)]
pub(crate) use repo::rate_limits::TABLE as RATE_LIMITS_TABLE;
pub(crate) use repo::{
    api_keys::TABLE as API_KEYS_TABLE, tokens::TABLE as TOKENS_TABLE, users::TABLE as USERS_TABLE,
};

/// Pre-computed Argon2id hash used for timing equalization when user is not found.
pub(crate) const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

use crate::blocks::admin::USER_ROLES_TABLE;

// --- Shared helpers used by auth_ui::api::* and auth_ui::oauth::* ---

pub(crate) mod helpers {
    use super::*;

    pub(crate) async fn get_user_roles(
        ctx: &dyn wafer_run::context::Context,
        user_id: &str,
    ) -> Vec<String> {
        // Plan A2 stores role inline on `users.role`; legacy
        // USER_ROLES_TABLE carries multi-role-per-user history. Merge
        // both: the inline role is the bootstrap path, the table is the
        // legacy path. Dedup since both can produce "admin" for the
        // bootstrapped admin.
        use crate::blocks::helpers::RecordExt;
        let mut roles: Vec<String> = Vec::new();
        if let Ok(rec) = db::get(ctx, USERS_TABLE, user_id).await {
            let inline = rec.str_field("role");
            if !inline.is_empty() {
                roles.push(inline.to_string());
            }
        }
        let filters = vec![Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.to_string()),
        }];
        if let Ok(records) = db::list_all(ctx, USER_ROLES_TABLE, filters).await {
            for rec in &records {
                if let Some(role) = rec.data.get("role").and_then(|v| v.as_str()) {
                    if !roles.iter().any(|r| r == role) {
                        roles.push(role.to_string());
                    }
                }
            }
        }
        roles
    }

    /// Resolve user roles, idempotently granting `admin` if the user's email
    /// matches the configured `SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_EMAIL`
    /// and they don't already have it.
    ///
    /// This closes a real footgun: roles are normally only assigned at signup,
    /// so changing the configured admin email after a user already exists
    /// never elevates them. With this helper, every login re-checks the rule
    /// and grants admin once when appropriate.
    ///
    /// Intentionally **upgrade-only**: never removes a role, never demotes.
    /// Unsetting the admin email does not revoke admin from anyone — that has
    /// to be done explicitly via the admin UI / DB. Removing roles silently
    /// on login would be an availability foot-gun (one typo in env locks
    /// everyone out).
    pub(crate) async fn ensure_admin_role(
        ctx: &dyn wafer_run::context::Context,
        user_id: &str,
        email: &str,
    ) -> Vec<String> {
        let mut roles = get_user_roles(ctx, user_id).await;

        let admin_email =
            config_client::get_default(ctx, "SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_EMAIL", "")
                .await;
        if admin_email.is_empty()
            || !email.eq_ignore_ascii_case(&admin_email)
            || roles.iter().any(|r| r == "admin")
        {
            return roles;
        }

        // Email matches and admin role is missing — grant it.
        let role_data = json_map(serde_json::json!({
            "user_id": user_id,
            "role": "admin",
            "assigned_at": crate::blocks::helpers::now_rfc3339(),
        }));
        match db::create(ctx, USER_ROLES_TABLE, role_data).await {
            Ok(_) => {
                tracing::info!(
                    user_id = %user_id,
                    email = %email,
                    "granted admin role on login (email matches ADMIN_EMAIL)"
                );
                roles.push("admin".to_string());
            }
            Err(e) => {
                tracing::warn!(
                    user_id = %user_id,
                    "failed to grant admin role on login: {e}"
                );
            }
        }
        roles
    }

    /// Returns (access_token, refresh_token, family).
    ///
    /// `auth_method` records *how* the user authenticated for this token —
    /// `"password"` for email/password login or signup, `"oauth.<provider>"`
    /// for OAuth (e.g. `"oauth.github"`). The claim rides on both access and
    /// refresh tokens so it survives refresh, letting downstream gates (like
    /// the wafer registry's publish endpoint) require a stronger method.
    pub(crate) async fn generate_tokens(
        ctx: &dyn wafer_run::context::Context,
        user_id: &str,
        email: &str,
        roles: &[String],
        auth_method: &str,
    ) -> std::result::Result<(String, String, String), wafer_run::OutputStream> {
        let family = match crypto::random_bytes(ctx, 16).await {
            Ok(bytes) => hex_encode(&bytes),
            Err(e) => return Err(wafer_run::OutputStream::error(e)),
        };

        // [SEC-038] Stamp `iss` on every token we mint so the read side can
        // reject tokens minted by a different deployment (e.g. a sibling
        // env's leaked secret) instead of trusting any signature with the
        // same HMAC key.
        let issuer = expected_issuer(ctx).await;

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
        access_claims.insert(
            "auth_method".to_string(),
            serde_json::Value::String(auth_method.to_string()),
        );
        access_claims.insert("iss".to_string(), serde_json::Value::String(issuer.clone()));

        let access_token = crypto::sign(ctx, &access_claims, Duration::from_secs(86400))
            .await
            .map_err(wafer_run::OutputStream::error)?;

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
        refresh_claims.insert(
            "auth_method".to_string(),
            serde_json::Value::String(auth_method.to_string()),
        );
        refresh_claims.insert("iss".to_string(), serde_json::Value::String(issuer.clone()));

        let refresh_token = crypto::sign(ctx, &refresh_claims, Duration::from_secs(604800))
            .await
            .map_err(wafer_run::OutputStream::error)?;

        Ok((access_token, refresh_token, family))
    }

    /// [SEC-038] Resolve the canonical JWT `iss` value for this deployment.
    ///
    /// `SOLOBASE_SHARED__FRONTEND_URL` doubles as the issuer: it's the only
    /// per-deployment URL admins reliably set, and treating it as the issuer
    /// means a token minted in dev (`http://localhost:5173`) won't validate
    /// against a production secret if one leaks between environments.
    pub(crate) async fn expected_issuer(ctx: &dyn wafer_run::context::Context) -> String {
        config_client::get_default(
            ctx,
            "SOLOBASE_SHARED__FRONTEND_URL",
            "http://localhost:5173",
        )
        .await
    }

    pub(crate) async fn store_refresh_token(
        ctx: &dyn wafer_run::context::Context,
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
        if let Err(e) = db::create(ctx, TOKENS_TABLE, data).await {
            tracing::warn!("Failed to store refresh token: {e}");
        }
    }

    pub(crate) async fn build_auth_cookie(
        token: &str,
        max_age: u64,
        ctx: &dyn wafer_run::context::Context,
    ) -> String {
        let env =
            config_client::get_default(ctx, "SOLOBASE_SHARED__ENVIRONMENT", "development").await;
        let secure = env.to_lowercase() != "development";
        format!(
            "auth_token={}; HttpOnly; Path=/; SameSite=Lax; Max-Age={}{}",
            token,
            max_age,
            if secure { "; Secure" } else { "" }
        )
    }

    pub(crate) fn urlencode(s: &str) -> String {
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
    use wafer_run::meta::*;

    use crate::blocks::helpers::{sha256_hex, RecordExt};

    let key_hash = sha256_hex(api_key.as_bytes());

    // Look up by key_hash
    let key_record = match db::get_by_field(
        ctx,
        API_KEYS_TABLE,
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

    let user = match db::get(ctx, USERS_TABLE, user_id).await {
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

use crate::ui::{templates::BrandPanel, SiteConfig};

/// Shared brand panel used by `auth_ui::pages::*` (login / signup / reset /
/// OAuth / change-password / bootstrap).
pub(crate) fn brand_panel(config: &SiteConfig) -> BrandPanel<'_> {
    BrandPanel {
        logo_html: None,
        headline: &config.app_name,
        tagline: Some("Sign in to continue."),
    }
}
