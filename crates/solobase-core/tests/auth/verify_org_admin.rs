//! `AuthService::verify_org_admin` — full dispatch matrix (8 cases).
//!
//! Covers:
//!   1. Reserved org + admin user  → true
//!   2. Reserved org + plain user  → false
//!   3. Non-reserved org owner     → true (no provider call)
//!   4. Provider says admin        → true
//!   5. Provider says no           → false
//!   6. Non-github provider        → false
//!   7. Provider 5xx               → ProviderDown error, not cached
//!   8. Unknown org                → false
//!
//! The fake GitHub provider is shared from Plan B (`tests/auth/fake_github.rs`)
//! and extended with an error-injection knob.

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use solobase_core::blocks::auth::{
    cache::OrgAdminCache,
    migrations,
    providers::{registry::ProviderRegistry, OAuthProvider, ProviderError, ProviderProfile},
    repo::{orgs, provider_links, users},
    service::{AuthServiceImpl, BlockState},
};
use wafer_core::interfaces::auth::service::{AuthError, AuthService, UserId};
use wafer_run::context::Context;

use crate::common::MigrationTestCtx;

/// Provider double that lets each test script a specific `check_org_admin`
/// answer per `(access_token, org_ref)` pair. Independent of the shared
/// `FakeGithub` from Plan B so verify_org_admin tests don't race with
/// OAuth-callback tests that wire up their own fake.
#[derive(Default)]
struct ScriptedGithub {
    answers: std::sync::Mutex<HashMap<(String, String), ScriptedAnswer>>,
}

enum ScriptedAnswer {
    Bool(bool),
    Upstream,
    #[allow(dead_code)]
    NotSupported,
}

impl ScriptedGithub {
    fn new() -> Self {
        Self::default()
    }

    fn set_admin(&self, token: &str, org_ref: &str, is_admin: bool) {
        self.answers.lock().unwrap().insert(
            (token.into(), org_ref.into()),
            ScriptedAnswer::Bool(is_admin),
        );
    }

    fn set_upstream_error(&self, token: &str, org_ref: &str) {
        self.answers
            .lock()
            .unwrap()
            .insert((token.into(), org_ref.into()), ScriptedAnswer::Upstream);
    }
}

#[async_trait]
impl OAuthProvider for ScriptedGithub {
    fn name(&self) -> &'static str {
        "github"
    }
    fn authorize_url(&self, _state: &str, _pkce: &str) -> String {
        "https://fake/authorize".into()
    }
    async fn exchange_code(
        &self,
        _code: &str,
        _verifier: &str,
    ) -> Result<ProviderProfile, ProviderError> {
        Err(ProviderError::NotSupported)
    }
    async fn check_org_admin(
        &self,
        access_token: &str,
        org_ref: &str,
    ) -> Result<bool, ProviderError> {
        let guard = self.answers.lock().unwrap();
        match guard.get(&(access_token.into(), org_ref.into())) {
            Some(ScriptedAnswer::Bool(b)) => Ok(*b),
            Some(ScriptedAnswer::Upstream) => Err(ProviderError::Upstream("injected 502".into())),
            Some(ScriptedAnswer::NotSupported) => Err(ProviderError::NotSupported),
            None => panic!(
                "test did not script an answer for check_org_admin({access_token:?}, {org_ref:?})"
            ),
        }
    }
}

struct Harness {
    ctx: Arc<dyn Context>,
    svc: AuthServiceImpl,
    gh: Arc<ScriptedGithub>,
}

async fn boot() -> Harness {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.expect("migrations");

    let gh = Arc::new(ScriptedGithub::new());
    let mut inner: HashMap<&'static str, Arc<dyn OAuthProvider>> = HashMap::new();
    inner.insert("github", gh.clone() as Arc<dyn OAuthProvider>);
    let providers = Arc::new(ProviderRegistry::from_map(inner));

    let state = BlockState::new(ctx.clone())
        .with_providers(providers)
        .with_org_admin_cache(OrgAdminCache::default());
    let svc = AuthServiceImpl::new(state);
    Harness { ctx, svc, gh }
}

async fn mk_user(ctx: &dyn Context, email: &str, role: &str) -> UserId {
    let row = users::insert(
        ctx,
        users::NewUser {
            email: email.into(),
            display_name: email.into(),
            avatar_url: None,
            role: role.into(),
        },
    )
    .await
    .expect("insert user");
    UserId(row.id)
}

async fn seed_claimed(ctx: &dyn Context, name: &str, owner: &UserId, verified_ref: &str) {
    orgs::upsert_claimed(
        ctx,
        orgs::NewClaim {
            name,
            owner_user_id: &owner.0,
            verified_via: "github",
            verified_ref,
        },
    )
    .await
    .expect("seed claimed org");
}

async fn link_provider(
    ctx: &dyn Context,
    user: &UserId,
    provider: &str,
    provider_ref: &str,
    login: &str,
    access_token: &str,
) {
    provider_links::upsert(
        ctx,
        provider_links::NewLink {
            provider,
            provider_ref,
            user_id: &user.0,
            provider_login: login,
            access_token,
        },
    )
    .await
    .expect("upsert provider link");
}

// ── Case 1 ───────────────────────────────────────────────────────────────
// Migration 002 already seeds `wafer-run` as reserved.
#[tokio::test]
async fn reserved_org_with_admin_user_returns_true() {
    let h = boot().await;
    let admin = mk_user(h.ctx.as_ref(), "admin@x.com", "admin").await;
    assert_eq!(
        h.svc
            .verify_org_admin(admin, "github", "wafer-run")
            .await
            .unwrap(),
        true
    );
}

// ── Case 2 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn reserved_org_with_plain_user_returns_false() {
    let h = boot().await;
    let u = mk_user(h.ctx.as_ref(), "u@x.com", "user").await;
    assert_eq!(
        h.svc
            .verify_org_admin(u, "github", "wafer-run")
            .await
            .unwrap(),
        false
    );
}

// ── Case 3 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn non_reserved_owner_returns_true_without_provider_call() {
    let h = boot().await;
    let owner = mk_user(h.ctx.as_ref(), "o@x.com", "user").await;
    seed_claimed(h.ctx.as_ref(), "acme", &owner, "acme").await;
    // No scripted answer → if code calls the provider, it panics.
    assert_eq!(
        h.svc
            .verify_org_admin(owner, "github", "acme")
            .await
            .unwrap(),
        true
    );
}

// ── Case 4 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn provider_says_admin_returns_true() {
    let h = boot().await;
    let owner = mk_user(h.ctx.as_ref(), "o@x.com", "user").await;
    let other = mk_user(h.ctx.as_ref(), "other@x.com", "user").await;
    link_provider(
        h.ctx.as_ref(),
        &other,
        "github",
        "999",
        "other",
        "token-other",
    )
    .await;
    seed_claimed(h.ctx.as_ref(), "acme", &owner, "acme").await;
    h.gh.set_admin("token-other", "acme", true);
    assert_eq!(
        h.svc
            .verify_org_admin(other, "github", "acme")
            .await
            .unwrap(),
        true
    );
}

// ── Case 5 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn provider_says_no_returns_false() {
    let h = boot().await;
    let owner = mk_user(h.ctx.as_ref(), "o@x.com", "user").await;
    let other = mk_user(h.ctx.as_ref(), "other@x.com", "user").await;
    link_provider(
        h.ctx.as_ref(),
        &other,
        "github",
        "999",
        "other",
        "token-other",
    )
    .await;
    seed_claimed(h.ctx.as_ref(), "acme", &owner, "acme").await;
    h.gh.set_admin("token-other", "acme", false);
    assert_eq!(
        h.svc
            .verify_org_admin(other, "github", "acme")
            .await
            .unwrap(),
        false
    );
}

// ── Case 6 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn non_github_provider_returns_ok_false() {
    let h = boot().await;
    let owner = mk_user(h.ctx.as_ref(), "o@x.com", "user").await;
    seed_claimed(h.ctx.as_ref(), "acme", &owner, "acme").await;
    // Owner is owner on github; asking as google → provider mismatch → false.
    let got = h
        .svc
        .verify_org_admin(owner, "google", "acme")
        .await
        .unwrap();
    assert_eq!(got, false);
}

// ── Case 7 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn provider_5xx_bubbles_as_provider_down_and_is_not_cached() {
    let h = boot().await;
    let owner = mk_user(h.ctx.as_ref(), "o@x.com", "user").await;
    let other = mk_user(h.ctx.as_ref(), "other@x.com", "user").await;
    link_provider(
        h.ctx.as_ref(),
        &other,
        "github",
        "999",
        "other",
        "token-other",
    )
    .await;
    seed_claimed(h.ctx.as_ref(), "acme", &owner, "acme").await;
    h.gh.set_upstream_error("token-other", "acme");
    let err = h
        .svc
        .verify_org_admin(other.clone(), "github", "acme")
        .await
        .unwrap_err();
    assert!(matches!(err, AuthError::ProviderDown(_)), "got {err:?}");

    // Now flip to a real answer — the second call should NOT return a
    // cached `false` from the error path, it should hit the provider again.
    h.gh.set_admin("token-other", "acme", true);
    assert_eq!(
        h.svc
            .verify_org_admin(other, "github", "acme")
            .await
            .unwrap(),
        true
    );
}

// ── Case 8 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn unknown_org_returns_ok_false() {
    let h = boot().await;
    let u = mk_user(h.ctx.as_ref(), "u@x.com", "user").await;
    assert_eq!(
        h.svc
            .verify_org_admin(u, "github", "never-claimed")
            .await
            .unwrap(),
        false
    );
}
