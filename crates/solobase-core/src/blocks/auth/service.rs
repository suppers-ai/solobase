//! AuthServiceImpl — implements the wafer-core `AuthService` trait.
//!
//! Extracts credentials from incoming [`Message`]s (Bearer header or
//! `wafer_session` cookie), looks them up in the `suppers_ai__auth__sessions`
//! or `suppers_ai__auth__personal_access_tokens` tables, and bumps
//! `last_used_at`.
//!
//! See `docs/superpowers/specs/2026-04-21-auth-block-design.md` §4 for the
//! cross-block contract and §6 for the bootstrap-token fallback.

use std::sync::Arc;

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use wafer_core::interfaces::auth::service::{
    AuthError, AuthService, Role, TokenScope, UserId, UserProfile,
};
use wafer_run::{context::Context, types::Message};

use super::repo::{pats, sessions, users};

/// Per-block state captured at `register` time. Holds the [`Context`] handle
/// so service methods can dispatch messages to `wafer-run/database` etc.
#[derive(Clone)]
pub struct BlockState {
    /// The auth block's context. In production this is the `&dyn Context`
    /// forwarded from `Block::handle`; in tests it's the in-memory sqlite
    /// context. Stored as `Arc` so `AuthServiceImpl` can be cloned into
    /// multiple call sites.
    pub ctx: Arc<dyn Context>,
}

impl BlockState {
    pub fn new(ctx: Arc<dyn Context>) -> Self {
        Self { ctx }
    }

    /// Test-only constructor. Kept simple on purpose — takes the same
    /// `Arc<dyn Context>` as production but exists so test setup reads
    /// naturally and so the two entry points are clearly labelled.
    pub fn for_test(ctx: Arc<dyn Context>) -> Self {
        Self { ctx }
    }
}

/// `AuthService` implementation backed by the auth block's repo layer.
pub struct AuthServiceImpl {
    state: BlockState,
}

impl AuthServiceImpl {
    pub fn new(state: BlockState) -> Self {
        Self { state }
    }
}

/// sha256 of a raw token string. Exposed so tests and the (future) session
/// issuance helper in Plan A2 agree on the hash format.
pub fn hash_token(raw: &str) -> Vec<u8> {
    Sha256::digest(raw.as_bytes()).to_vec()
}

/// Extract a Bearer token from the `Authorization` header.
fn bearer_from(msg: &Message) -> Option<String> {
    let v = msg.header("authorization");
    if v.is_empty() {
        return None;
    }
    v.strip_prefix("Bearer ").map(str::to_owned)
}

/// Extract the `wafer_session` cookie value, if any.
fn session_cookie_from(msg: &Message) -> Option<String> {
    let v = msg.cookie("wafer_session");
    if v.is_empty() {
        None
    } else {
        Some(v.to_owned())
    }
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Internal credential classification used by all three require_* methods.
enum Creds {
    Session(Vec<u8>),
    Pat(Vec<u8>),
}

fn extract_creds(msg: &Message) -> Result<Creds, AuthError> {
    if let Some(bearer) = bearer_from(msg) {
        return Ok(Creds::Pat(hash_token(&bearer)));
    }
    if let Some(cookie) = session_cookie_from(msg) {
        return Ok(Creds::Session(hash_token(&cookie)));
    }
    Err(AuthError::Unauthorized)
}

#[async_trait]
impl AuthService for AuthServiceImpl {
    async fn require_user(&self, msg: &Message) -> Result<UserId, AuthError> {
        let ctx = self.state.ctx.as_ref();
        match extract_creds(msg)? {
            Creds::Session(h) => {
                let row = sessions::find_by_token_hash(ctx, &h)
                    .await
                    .map_err(|e| AuthError::Internal(e.to_string()))?
                    .ok_or(AuthError::Unauthorized)?;
                if row.expires_at.as_str() < now_iso().as_str() {
                    return Err(AuthError::Unauthorized);
                }
                sessions::touch_last_used(ctx, &h)
                    .await
                    .map_err(|e| AuthError::Internal(e.to_string()))?;
                Ok(UserId(row.user_id))
            }
            Creds::Pat(h) => {
                let row = pats::find_by_token_hash(ctx, &h)
                    .await
                    .map_err(|e| AuthError::Internal(e.to_string()))?
                    .ok_or(AuthError::Unauthorized)?;
                if let Some(exp) = row.expires_at.as_deref() {
                    if exp < now_iso().as_str() {
                        return Err(AuthError::Unauthorized);
                    }
                }
                pats::touch_last_used(ctx, &h)
                    .await
                    .map_err(|e| AuthError::Internal(e.to_string()))?;
                Ok(UserId(row.user_id))
            }
        }
    }

    async fn require_token(&self, msg: &Message, scope: TokenScope) -> Result<UserId, AuthError> {
        let ctx = self.state.ctx.as_ref();
        let creds = extract_creds(msg)?;
        // Scopes live exclusively on PATs. A session cookie presented here is
        // a category error — treat it as Forbidden so the caller knows the
        // credentials are valid but wrong type, not just missing.
        let h = match creds {
            Creds::Pat(h) => h,
            Creds::Session(_) => return Err(AuthError::Forbidden),
        };
        let row = pats::find_by_token_hash(ctx, &h)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
            .ok_or(AuthError::Unauthorized)?;
        if let Some(exp) = row.expires_at.as_deref() {
            if exp < now_iso().as_str() {
                return Err(AuthError::Unauthorized);
            }
        }
        let needed = match scope {
            TokenScope::Publish => "publish",
        };
        if !row.scopes.iter().any(|s| s == needed) {
            return Err(AuthError::Forbidden);
        }
        pats::touch_last_used(ctx, &h)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        Ok(UserId(row.user_id))
    }

    async fn require_role(&self, msg: &Message, role: Role) -> Result<UserId, AuthError> {
        let ctx = self.state.ctx.as_ref();

        // Bootstrap-token fast path: if the caller presents a Bearer token
        // that matches an unexpired row in `bootstrap_tokens`, grant Admin.
        // Bootstrap tokens are not tied to a user — use a sentinel id.
        // Admin-gated handlers read `role`, not id, at this stage (user-id
        // coupling lands in Plan A2 when bootstrap consumption creates the
        // first real admin user).
        if matches!(role, Role::Admin) {
            if let Some(bearer) = bearer_from(msg) {
                let h = hash_token(&bearer);
                let valid = super::repo::bootstrap_tokens::is_valid(ctx, &h)
                    .await
                    .map_err(|e| AuthError::Internal(e.to_string()))?;
                if valid {
                    return Ok(UserId("bootstrap".to_string()));
                }
            }
        }

        let uid = self.require_user(msg).await?;
        let row = users::find_by_id(ctx, &uid.0)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
            .ok_or(AuthError::NotFound)?;
        let has = match role {
            Role::Admin => row.role == "admin",
            Role::User => true, // any authenticated user
        };
        if has {
            Ok(uid)
        } else {
            Err(AuthError::Forbidden)
        }
    }

    async fn verify_org_admin(
        &self,
        _user: UserId,
        _provider: &str,
        _org_ref: &str,
    ) -> Result<bool, AuthError> {
        // Lands in Plan C (org-ownership verification).
        Ok(false)
    }

    async fn user_profile(&self, user: UserId) -> Result<UserProfile, AuthError> {
        let ctx = self.state.ctx.as_ref();
        let row = users::find_by_id(ctx, &user.0)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
            .ok_or(AuthError::NotFound)?;
        let role = match row.role.as_str() {
            "admin" => Role::Admin,
            _ => Role::User,
        };
        Ok(UserProfile {
            id: UserId(row.id),
            email: row.email,
            display_name: row.display_name,
            avatar_url: row.avatar_url,
            role,
            orgs: Vec::new(), // populated by Plan C
        })
    }
}
