//! AuthServiceImpl — implements the wafer-core `AuthService` trait.
//!
//! Extracts credentials from incoming [`Message`]s (Bearer header or
//! `wafer_session` cookie), looks them up in the `suppers_ai__auth__sessions`
//! or `suppers_ai__auth__personal_access_tokens` tables, and bumps
//! `last_used_at`.
//!
//! See `docs/superpowers/specs/2026-04-21-auth-block-design.md` §4 for the
//! cross-block contract and §6 for the bootstrap-token fallback.

use std::sync::{Arc, OnceLock};

use wafer_core::interfaces::auth::service::{
    AuthError, AuthService, Role, TokenScope, UserId, UserProfile,
};
use wafer_run::{context::Context, types::Message};

use super::{
    cache::OrgAdminCache,
    repo::{orgs, pats, sessions, users},
};

/// Per-block state. Holds a lazy [`Context`] handle so service methods can
/// dispatch messages to `wafer-run/database` etc., and the shared
/// `OrgAdminCache`.
///
/// `ctx` is populated lazily because `AuthServiceImpl` is constructed at
/// block-registration time (when no `Context` exists yet) and the framework
/// `AuthBlock::lifecycle(Init)` later passes one in via
/// [`AuthService::init`]. The Init hook calls `ctx.clone_arc()` (wafer-run
/// #46) and stores the resulting `Arc<dyn Context>` in the cell.
#[derive(Clone)]
pub struct BlockState {
    /// Lazy auth context handle. Populated by [`AuthServiceImpl::init`] from
    /// the framework AuthBlock's `lifecycle(Init)` hook; tests pre-populate
    /// via [`BlockState::for_test`].
    pub ctx: Arc<OnceLock<Arc<dyn Context>>>,
    /// Shared in-process cache for `verify_org_admin` results. Cloned
    /// cheaply (it's `Arc` internally) into the handler layer so the
    /// logout handler can invalidate a user's entries.
    pub org_admin_cache: OrgAdminCache,
}

impl BlockState {
    /// Production constructor — context cell starts empty and is populated
    /// later by [`AuthServiceImpl::init`] when the framework AuthBlock fires
    /// its `Init` lifecycle event.
    pub fn new() -> Self {
        Self {
            ctx: Arc::new(OnceLock::new()),
            org_admin_cache: OrgAdminCache::default(),
        }
    }

    /// Test-only constructor. Pre-populates the context cell so service
    /// methods can run without going through the full `init` lifecycle.
    pub fn for_test(ctx: Arc<dyn Context>) -> Self {
        let cell = OnceLock::new();
        let _ = cell.set(ctx);
        Self {
            ctx: Arc::new(cell),
            org_admin_cache: OrgAdminCache::default(),
        }
    }

    /// Attach a shared [`OrgAdminCache`]. The production wiring uses this
    /// so the logout handler and the service share the same cache instance.
    pub fn with_org_admin_cache(mut self, cache: OrgAdminCache) -> Self {
        self.org_admin_cache = cache;
        self
    }
}

impl Default for BlockState {
    fn default() -> Self {
        Self::new()
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

    /// Borrow the lazy context handle. Returns `Err(Internal)` if `init`
    /// hasn't run yet — callers that hit this path are pre-init dispatches
    /// (a `handle` arriving before the framework AuthBlock's
    /// `lifecycle(Init)`), which would only happen on a runtime bug.
    fn ctx(&self) -> Result<&dyn Context, AuthError> {
        self.state
            .ctx
            .get()
            .map(|arc| arc.as_ref())
            .ok_or_else(|| AuthError::Internal("auth service ctx not initialized".to_string()))
    }
}

/// sha256 of a raw token string. Exposed so tests and the (future) session
/// issuance helper in Plan A2 agree on the hash format. Thin wrapper over
/// [`crate::blocks::helpers::sha256`] — there is one canonical sha256
/// implementation in `blocks::helpers`.
pub fn hash_token(raw: &str) -> Vec<u8> {
    crate::blocks::helpers::sha256(raw.as_bytes()).to_vec()
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

/// Returns `true` iff `expires_at` parses as an RFC3339 timestamp earlier
/// than now. Parsing the timestamp avoids the mixed-format trap of string
/// comparison (`+00:00` vs `Z`) — the auth tables intermix both because
/// some repo helpers write `…Z` and others use `to_rfc3339()`.
///
/// Unparseable inputs are treated as "expired" — a malformed expiry on a
/// session row is safer to reject than silently grant.
fn is_expired(expires_at: &str) -> bool {
    match chrono::DateTime::parse_from_rfc3339(expires_at) {
        Ok(exp) => chrono::Utc::now() >= exp.with_timezone(&chrono::Utc),
        Err(_) => true,
    }
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

/// Static WRAP grants for the framework `suppers-ai/auth` block. Returned by
/// both [`AuthService::grants`] (consumed by `AuthBlock::info()` so the
/// runtime registers them at startup) and called directly by userportal
/// pages that reflect over auth's grant list to compose their own WRAP
/// scope. Keep these in sync with the spec at
/// `docs/superpowers/specs/2026-04-21-auth-block-design.md`.
pub fn auth_grants() -> Vec<wafer_block::types::ResourceGrant> {
    // String literals are used (instead of repo::*::TABLE consts) so the
    // static WRAP-grant audit script (scripts/audit-wrap-grants.sh) can
    // resolve every grant target — its const-resolver only follows
    // top-level `super::NAME` paths, not nested module paths like
    // `repo::users::TABLE`. Each literal must stay in sync with the
    // corresponding `pub const TABLE` in repo/*.rs.
    vec![
        // auth-ui owns the SSR / JSON / OAuth handlers and writes every
        // auth table during login, signup, OAuth callback, etc. The
        // wildcard covers users / sessions / pats / provider_links /
        // bootstrap_tokens / orgs / api_keys without enumerating each.
        wafer_run::ResourceGrant::read_write("suppers-ai/auth-ui", "suppers_ai__auth__*"),
        // The pipeline router (SolobaseRouterBlock, id `suppers-ai/router`)
        // calls `jwt_blocklist::contains()` from `crate::crypto::extract_auth_meta`
        // during request preprocessing — SEC-042 logout invalidates JWTs
        // via this table. The call runs in the router's context, so the
        // router needs read access. Without it WRAP denies and the
        // contains() fail-closed path treats every JWT as blocklisted,
        // 403-ing every signed-in admin request.
        wafer_run::ResourceGrant::read("suppers-ai/router", "suppers_ai__auth__jwt_blocklist"),
        // Admin block reads auth tables for the admin dashboards. The
        // wildcard mirrors the legacy AuthBlock grant — admin/pages/users
        // reads users, sessions, AND api_keys (the API-key tab) so the
        // narrower per-table list would regress.
        wafer_run::ResourceGrant::read("suppers-ai/admin", "suppers_ai__auth__*"),
        // Userportal `/b/userportal/sessions` page lists the caller's
        // sessions and revokes individual rows. Read+write because revoke
        // deletes the row; reads are scoped to the caller's user_id by
        // the repo helper.
        wafer_run::ResourceGrant::read_write("suppers-ai/userportal", "suppers_ai__auth__sessions"),
        // Userportal `/b/userportal/security` lists the caller's
        // linked OAuth providers. Read-only — unlinking goes
        // through an auth POST endpoint, not the userportal block.
        wafer_run::ResourceGrant::read("suppers-ai/userportal", "suppers_ai__auth__provider_links"),
        wafer_run::ResourceGrant::read_write("suppers-ai/userportal", "suppers_ai__auth__users"),
        wafer_run::ResourceGrant::read("suppers-ai/products", "suppers_ai__auth__users"),
        // Wave 3: rate_limit.rs (called from products + files blocks) writes to
        // suppers_ai__auth__rate_limits on the wasm32 (Cloudflare Workers) path.
        // Native uses an in-memory Mutex<HashMap> counter and never touches the DB.
        // auth-ui is already covered by the wildcard grant above.
        wafer_run::ResourceGrant::read_write(
            "suppers-ai/products",
            "suppers_ai__auth__rate_limits",
        ),
        wafer_run::ResourceGrant::read_write("suppers-ai/files", "suppers_ai__auth__rate_limits"),
    ]
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl AuthService for AuthServiceImpl {
    /// Apply auth migrations and run the bootstrap admin step. Invoked by the
    /// framework `AuthBlock::lifecycle(Init)` (wafer-run #41/#45) once at
    /// startup. Mirrors the body of the custom solobase `AuthBlock::lifecycle`
    /// so the framework block has a self-sufficient service to delegate to.
    async fn init(&self, ctx: &dyn Context) -> Result<(), AuthError> {
        // Capture an owning `Arc<dyn Context>` so subsequent `require_*`
        // calls have a context handle to dispatch repo lookups through.
        // wafer-run #46 added `Context::clone_arc` for exactly this. `set`
        // returns `Err` if the cell was already populated (e.g. test
        // pre-populated via `for_test`, or a duplicate `Init` event); both
        // cases are harmless — the existing handle keeps pointing at the
        // same shared snapshots.
        let _ = self.state.ctx.set(ctx.clone_arc());

        super::migrations::apply(ctx)
            .await
            .map_err(|e| AuthError::Internal(format!("auth migrations: {e}")))?;
        let cfg = super::config::AuthConfig::from_ctx(ctx).await;
        super::bootstrap::run(ctx, &cfg)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        Ok(())
    }

    /// WRAP grants the auth block declares for downstream consumers. The
    /// framework `AuthBlock::info()` embeds these into `BlockInfo::grants`
    /// (wafer-run #45) so the runtime registers them at startup.
    ///
    /// Delegates to the [`auth_grants`] free function so non-trait callers
    /// (e.g. userportal's WRAP-grant reflection in `pages/sessions.rs` and
    /// `pages/security.rs`) can see the same list without instantiating
    /// the framework block.
    fn grants(&self) -> Vec<wafer_block::types::ResourceGrant> {
        auth_grants()
    }

    async fn require_user(&self, msg: &Message) -> Result<UserId, AuthError> {
        let ctx = self.ctx()?;
        match extract_creds(msg)? {
            Creds::Session(h) => {
                let row = sessions::find_by_token_hash(ctx, &h)
                    .await
                    .map_err(|e| AuthError::Internal(e.to_string()))?
                    .ok_or(AuthError::Unauthorized)?;
                if is_expired(&row.expires_at) {
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
                    if is_expired(exp) {
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
        let ctx = self.ctx()?;
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
            if is_expired(exp) {
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
        let ctx = self.ctx()?;

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

        let ctx = self.ctx()?;

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

        // 3) Reserved orgs (spec §3): site-admin only.
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

        // 5) Owner short-circuit: the original claimant is always an admin.
        //    Without an `OAuthProvider` registry there is no upstream
        //    membership check, so non-owner members of a claimed org are
        //    not granted admin from here.
        let is_owner = org.owner_user_id.as_deref() == Some(user.0.as_str());
        cache.insert(&user, provider, org_ref, is_owner);
        Ok(is_owner)
    }

    async fn user_profile(&self, user: UserId) -> Result<UserProfile, AuthError> {
        let ctx = self.ctx()?;
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

#[cfg(test)]
mod tests {
    //! Trait-level dispatch tests for `init` + `grants`. The underlying
    //! `migrations::apply` and `bootstrap::run` helpers have their own
    //! integration tests in `tests/auth/` — what we exercise here is that
    //! `<AuthServiceImpl as AuthService>::init` actually calls them, and
    //! that `grants()` returns the expected consumer set.
    use std::sync::Arc;

    use wafer_core::{clients::database as db, interfaces::auth::service::AuthService as _};

    use super::*;
    use crate::test_support::TestContext;

    #[tokio::test]
    async fn init_applies_migrations_and_runs_bootstrap_on_fresh_ctx() {
        // Admin migrations are pre-applied so the `block_settings` tracking
        // table exists — `apply_if_blessed` requires it to upsert the
        // `current_hash` row. In production `register_all_static_blocks`
        // registers admin first, so its Init runs before auth's.
        let ctx = Arc::new(TestContext::with_admin().await);
        let service = AuthServiceImpl::new(BlockState::for_test(ctx.clone()));

        service
            .init(&*ctx)
            .await
            .expect("init applies migrations and runs bootstrap");

        // Migrations applied → users table exists and is queryable.
        let rows = db::list_all(&*ctx, users::TABLE, vec![])
            .await
            .expect("users table exists after init");
        // No bootstrap admin env vars → bootstrap no-ops, table stays empty.
        assert_eq!(rows.len(), 0);
    }

    #[tokio::test]
    async fn init_is_idempotent() {
        // Running init twice must be safe — migrations track applied
        // versions and bootstrap short-circuits when users already exist.
        // Admin pre-applied for the same reason as above.
        let ctx = Arc::new(TestContext::with_admin().await);
        let service = AuthServiceImpl::new(BlockState::for_test(ctx.clone()));

        service.init(&*ctx).await.expect("first init");
        service
            .init(&*ctx)
            .await
            .expect("second init is idempotent");
    }

    #[test]
    fn grants_declares_expected_consumers() {
        // We don't need a context for grants(); construct the service with
        // a stub ctx and inspect the returned vec directly.
        let rt = tokio::runtime::Runtime::new().expect("tokio rt");
        let ctx = rt.block_on(async { Arc::new(TestContext::new().await) });
        let service = AuthServiceImpl::new(BlockState::for_test(ctx));

        let grants = service.grants();

        // The grant struct exposes grantee + resource as public fields;
        // we just check coverage of the four consumers.
        let consumers: Vec<&str> = grants.iter().map(|g| g.grantee.as_str()).collect();
        assert!(
            consumers.contains(&"suppers-ai/admin"),
            "grants must include admin: {consumers:?}"
        );
        assert!(
            consumers.contains(&"suppers-ai/userportal"),
            "grants must include userportal: {consumers:?}"
        );
        assert!(
            consumers.contains(&"suppers-ai/products"),
            "grants must include products: {consumers:?}"
        );
        // The pipeline router (suppers-ai/router) calls
        // jwt_blocklist::contains() during extract_auth_meta to honour
        // SEC-042 (logout invalidates JWT). Without a grant, WRAP denies
        // the read, jwt_blocklist::contains fails closed → true, and
        // every signed-in request is treated as anonymous.
        assert!(
            consumers.contains(&"suppers-ai/router"),
            "grants must include router (SEC-042 blocklist read): {consumers:?}"
        );
        // Sanity: at least one grant exists per consumer (non-empty list).
        assert!(
            grants.len() >= 5,
            "expected ≥5 grants, got {}",
            grants.len()
        );
    }
}
