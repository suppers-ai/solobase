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

use sha2::{Digest, Sha256};
use wafer_core::interfaces::auth::service::{
    AuthError, AuthService, Role, TokenScope, UserId, UserProfile,
};
use wafer_run::{context::Context, types::Message};

use super::{
    cache::OrgAdminCache,
    providers::registry::ProviderRegistry,
    repo::{orgs, pats, provider_links, sessions, users},
};

/// Per-block state captured at `register` time. Holds the [`Context`] handle
/// so service methods can dispatch messages to `wafer-run/database` etc.,
/// along with the OAuth provider registry (for `verify_org_admin` lookups)
/// and the shared `OrgAdminCache`.
#[derive(Clone)]
pub struct BlockState {
    /// The auth block's context. In production this is the `&dyn Context`
    /// forwarded from `Block::handle`; in tests it's the in-memory sqlite
    /// context. Stored as `Arc` so `AuthServiceImpl` can be cloned into
    /// multiple call sites.
    pub ctx: Arc<dyn Context>,
    /// OAuth provider registry. Defaults to empty — populated by
    /// `with_providers` when the block registers. `verify_org_admin` uses
    /// this to dispatch to `check_org_admin` for the caller's provider.
    pub providers: Arc<ProviderRegistry>,
    /// Shared in-process cache for `verify_org_admin` results. Cloned
    /// cheaply (it's `Arc` internally) into the handler layer so the
    /// logout handler can invalidate a user's entries.
    pub org_admin_cache: OrgAdminCache,
}

impl BlockState {
    pub fn new(ctx: Arc<dyn Context>) -> Self {
        Self {
            ctx,
            providers: Arc::new(ProviderRegistry::empty()),
            org_admin_cache: OrgAdminCache::default(),
        }
    }

    /// Test-only constructor. Kept simple on purpose — takes the same
    /// `Arc<dyn Context>` as production but exists so test setup reads
    /// naturally and so the two entry points are clearly labelled.
    pub fn for_test(ctx: Arc<dyn Context>) -> Self {
        Self::new(ctx)
    }

    /// Attach an OAuth provider registry. Returns `self` for builder-style
    /// chaining at block construction.
    pub fn with_providers(mut self, providers: Arc<ProviderRegistry>) -> Self {
        self.providers = providers;
        self
    }

    /// Attach a shared [`OrgAdminCache`]. The production wiring uses this
    /// so the logout handler and the service share the same cache instance.
    pub fn with_org_admin_cache(mut self, cache: OrgAdminCache) -> Self {
        self.org_admin_cache = cache;
        self
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

/// Look up a user's provider link by `(user_id, provider)`. Thin wrapper
/// around the repo helper; kept in service.rs because `verify_org_admin` is
/// the only caller and the name is clearer at the call site than the
/// full `provider_links::find_by_user_provider` path.
async fn find_link_for_user(
    ctx: &dyn Context,
    user: &UserId,
    provider: &str,
) -> Result<Option<provider_links::ProviderLink>, super::repo::RepoError> {
    provider_links::find_by_user_provider(ctx, &user.0, provider).await
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

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
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
        user: UserId,
        provider: &str,
        org_ref: &str,
    ) -> Result<bool, AuthError> {
        let cache = &self.state.org_admin_cache;

        // 1) Cache hit? Cached values (both true and false) are authoritative
        //    until TTL expires or the user logs out.
        if let Some(cached) = cache.get(&user, provider, org_ref) {
            return Ok(cached);
        }

        let ctx = self.state.ctx.as_ref();

        // 2) Look up the org by name. No row → not an admin. Reserved orgs
        //    are identified by name (migration 002 seeds them); claimed orgs
        //    are keyed on name plus (verified_via, verified_ref).
        let Some(org) = orgs::find_by_name(ctx, org_ref)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
        else {
            cache.insert(&user, provider, org_ref, false);
            return Ok(false);
        };

        // 3) Reserved orgs (spec §3): site-admin only, no provider call.
        if org.is_reserved {
            let row = users::find_by_id(ctx, &user.0)
                .await
                .map_err(|e| AuthError::Internal(e.to_string()))?
                .ok_or(AuthError::NotFound)?;
            let is_admin = row.role == "admin";
            cache.insert(&user, provider, org_ref, is_admin);
            return Ok(is_admin);
        }

        // 4) Non-reserved org but the caller's provider doesn't match the
        //    org's verified_via (e.g. google caller asking about a
        //    github-verified org) → not an admin. Owner short-circuit only
        //    applies when the provider matches.
        if org.verified_via.as_deref() != Some(provider) {
            cache.insert(&user, provider, org_ref, false);
            return Ok(false);
        }

        // 5) Owner short-circuit: the original claimant is always an admin
        //    and we don't need to ask the provider.
        if org.owner_user_id.as_deref() == Some(user.0.as_str()) {
            cache.insert(&user, provider, org_ref, true);
            return Ok(true);
        }

        // 6) Ask the provider. Need the caller's access token from
        //    provider_links — we look up by (provider, provider_ref) using
        //    the user's rows. find_by_provider_ref is keyed on
        //    (provider, provider_ref); we need a by-(user, provider) shape.
        let link = find_link_for_user(ctx, &user, provider)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        let Some(link) = link else {
            cache.insert(&user, provider, org_ref, false);
            return Ok(false);
        };

        // 7) Resolve the provider impl. An unknown provider name here means
        //    the block was configured without the provider enabled — treat
        //    as "not an admin" and cache it.
        let Some(provider_impl) = self.state.providers.get(provider) else {
            cache.insert(&user, provider, org_ref, false);
            return Ok(false);
        };

        let verified_ref = org.verified_ref.as_deref().unwrap_or(org_ref);
        match provider_impl
            .check_org_admin(&link.access_token, verified_ref)
            .await
        {
            Ok(is_admin) => {
                cache.insert(&user, provider, org_ref, is_admin);
                Ok(is_admin)
            }
            Err(super::providers::ProviderError::NotSupported) => {
                cache.insert(&user, provider, org_ref, false);
                Ok(false)
            }
            Err(super::providers::ProviderError::Unauthorized) => {
                // Revoked / expired upstream token — not admin, cache the
                // negative so a dashboard refresh loop doesn't retry.
                cache.insert(&user, provider, org_ref, false);
                Ok(false)
            }
            Err(super::providers::ProviderError::Upstream(msg)) => {
                // Transient upstream failure — DO NOT cache. Let the caller
                // decide whether to retry.
                Err(AuthError::ProviderDown(msg))
            }
            Err(other) => Err(AuthError::Internal(other.to_string())),
        }
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
