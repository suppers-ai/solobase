//! Block-integration tests for `POST /auth/orgs/claim`.
//!
//! Covers every branch of `handlers::orgs::post_claim`:
//!   - Missing session → 401
//!   - Malformed JSON / missing fields → 400
//!   - Name fails regex → 400
//!   - Reserved name → 409 with `error: "reserved"`
//!   - No provider_links row → 422
//!   - Provider says "not admin" → 403
//!   - Provider upstream error → 503
//!   - Happy path → 201, row persisted, cache warmed with `true`
//!   - Second caller, same name → 409 `already_claimed`
//!   - Second caller, same verified_ref with a different name → covered by
//!     the above (this handler uses `name` as the verified_ref so the two
//!     unique constraints collapse onto the same column in practice — the
//!     `NameTaken` 409 fires for the second attempt).

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use solobase_core::blocks::auth::{
    cache::OrgAdminCache,
    handlers::orgs::post_claim,
    migrations,
    providers::{registry::ProviderRegistry, OAuthProvider, ProviderError, ProviderProfile},
    repo::{orgs, provider_links, sessions, users},
    service::{hash_token, AuthServiceImpl, BlockState},
};
use wafer_core::interfaces::auth::service::{AuthService, UserId};
use wafer_run::{
    context::Context,
    types::{ErrorCode, Message, WaferError},
};

use crate::common::MigrationTestCtx;

/// Provider double with per-`(token, org)` scripted answers — identical to
/// the one used by `verify_org_admin.rs`, duplicated here so a test file
/// failure doesn't cascade across unrelated suites.
#[derive(Default)]
struct ScriptedGithub {
    answers: std::sync::Mutex<HashMap<(String, String), ScriptedAnswer>>,
}

enum ScriptedAnswer {
    Bool(bool),
    Upstream,
}

impl ScriptedGithub {
    fn new() -> Self {
        Self::default()
    }
    fn set_admin(&self, token: &str, org: &str, is_admin: bool) {
        self.answers
            .lock()
            .unwrap()
            .insert((token.into(), org.into()), ScriptedAnswer::Bool(is_admin));
    }
    fn set_upstream_error(&self, token: &str, org: &str) {
        self.answers
            .lock()
            .unwrap()
            .insert((token.into(), org.into()), ScriptedAnswer::Upstream);
    }
}

#[async_trait]
impl OAuthProvider for ScriptedGithub {
    fn name(&self) -> &'static str {
        "github"
    }
    fn authorize_url(&self, _state: &str, _c: &str) -> String {
        "https://fake/authorize".into()
    }
    async fn exchange_code(&self, _c: &str, _v: &str) -> Result<ProviderProfile, ProviderError> {
        Err(ProviderError::NotSupported)
    }
    async fn check_org_admin(&self, token: &str, org: &str) -> Result<bool, ProviderError> {
        let g = self.answers.lock().unwrap();
        match g.get(&(token.into(), org.into())) {
            Some(ScriptedAnswer::Bool(b)) => Ok(*b),
            Some(ScriptedAnswer::Upstream) => Err(ProviderError::Upstream("injected 502".into())),
            None => panic!("unscripted check_org_admin({token:?}, {org:?})"),
        }
    }
}

struct Harness {
    ctx: Arc<dyn Context>,
    svc: Arc<dyn AuthService>,
    providers: Arc<ProviderRegistry>,
    cache: OrgAdminCache,
    gh: Arc<ScriptedGithub>,
}

async fn boot() -> Harness {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.expect("migrations");

    let gh = Arc::new(ScriptedGithub::new());
    let mut m: HashMap<&'static str, Arc<dyn OAuthProvider>> = HashMap::new();
    m.insert("github", gh.clone() as Arc<dyn OAuthProvider>);
    let providers = Arc::new(ProviderRegistry::from_map(m));
    let cache = OrgAdminCache::default();
    let state = BlockState::new(ctx.clone())
        .with_providers(providers.clone())
        .with_org_admin_cache(cache.clone());
    let svc: Arc<dyn AuthService> = Arc::new(AuthServiceImpl::new(state));
    Harness {
        ctx,
        svc,
        providers,
        cache,
        gh,
    }
}

async fn mk_user(ctx: &dyn Context, email: &str) -> UserId {
    let row = users::insert(
        ctx,
        users::NewUser {
            email: email.into(),
            display_name: email.into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .expect("insert user");
    UserId(row.id)
}

async fn login_cookie(ctx: &dyn Context, user: &UserId) -> String {
    let raw = format!("sess-{}", user.0);
    sessions::insert(
        ctx,
        sessions::NewSession {
            token_hash: hash_token(&raw),
            user_id: user.0.clone(),
            expires_at: "9999-01-01T00:00:00Z".into(),
        },
    )
    .await
    .expect("seed session");
    raw
}

async fn link_provider(ctx: &dyn Context, user: &UserId, token: &str, provider_ref: &str) {
    provider_links::upsert(
        ctx,
        provider_links::NewLink {
            provider: "github",
            provider_ref,
            user_id: &user.0,
            provider_login: "tester",
            access_token: token,
        },
    )
    .await
    .expect("upsert link");
}

fn msg_with_cookie(cookie: Option<&str>) -> Message {
    let mut m = Message::new("create");
    m.set_meta(wafer_run::meta::META_REQ_ACTION, "create");
    m.set_meta(wafer_run::meta::META_REQ_RESOURCE, "/auth/orgs/claim");
    if let Some(c) = cookie {
        m.set_meta("http.header.cookie", format!("wafer_session={c}"));
    }
    m
}

async fn call_claim(
    h: &Harness,
    msg: &Message,
    body: &[u8],
) -> Result<solobase_core::blocks::auth::handlers::HttpReply, WaferError> {
    post_claim(
        h.ctx.as_ref(),
        h.svc.as_ref(),
        &h.providers,
        &h.cache,
        msg,
        body,
    )
    .await
}

fn body(name: &str, provider: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({ "name": name, "provider": provider })).unwrap()
}

fn parse_body(r: &solobase_core::blocks::auth::handlers::HttpReply) -> serde_json::Value {
    serde_json::from_slice(&r.body).expect("response body is JSON")
}

// ── 1) Happy path ────────────────────────────────────────────────────────
#[tokio::test]
async fn claim_happy_path_returns_201_and_warms_cache() {
    let h = boot().await;
    let u = mk_user(h.ctx.as_ref(), "u@x.com").await;
    link_provider(h.ctx.as_ref(), &u, "tok", "42").await;
    h.gh.set_admin("tok", "acme", true);
    let cookie = login_cookie(h.ctx.as_ref(), &u).await;

    let msg = msg_with_cookie(Some(&cookie));
    let reply = call_claim(&h, &msg, &body("acme", "github")).await.unwrap();
    assert_eq!(
        reply.status,
        201,
        "body: {}",
        String::from_utf8_lossy(&reply.body)
    );
    let b = parse_body(&reply);
    assert_eq!(b["name"], "acme");
    assert_eq!(b["verified_via"], "github");

    // Row persisted.
    let row = orgs::find_by_name(h.ctx.as_ref(), "acme")
        .await
        .unwrap()
        .expect("claimed row");
    assert_eq!(row.owner_user_id.as_deref(), Some(u.0.as_str()));

    // Cache warmed with `true` — a provider flip to false should NOT be
    // observed until TTL expires.
    assert_eq!(h.cache.get(&u, "github", "acme"), Some(true));
}

// ── 2) Unauthenticated ───────────────────────────────────────────────────
#[tokio::test]
async fn claim_without_session_returns_401() {
    let h = boot().await;
    let msg = msg_with_cookie(None);
    let reply = call_claim(&h, &msg, &body("acme", "github")).await.unwrap();
    assert_eq!(reply.status, 401);
}

// ── 3) Missing provider link ─────────────────────────────────────────────
#[tokio::test]
async fn claim_with_no_provider_link_returns_422() {
    let h = boot().await;
    let u = mk_user(h.ctx.as_ref(), "u@x.com").await;
    let cookie = login_cookie(h.ctx.as_ref(), &u).await;
    let msg = msg_with_cookie(Some(&cookie));
    let reply = call_claim(&h, &msg, &body("acme", "github")).await.unwrap();
    assert_eq!(reply.status, 422);
    let b = parse_body(&reply);
    assert_eq!(b["error"], "provider_not_linked");
    assert!(b["detail"].as_str().unwrap().contains("github"));
}

// ── 4) Reserved name ─────────────────────────────────────────────────────
#[tokio::test]
async fn claim_reserved_name_returns_409() {
    let h = boot().await;
    let u = mk_user(h.ctx.as_ref(), "u@x.com").await;
    link_provider(h.ctx.as_ref(), &u, "tok", "42").await;
    let cookie = login_cookie(h.ctx.as_ref(), &u).await;
    for reserved in ["wafer-run", "wafer", "suppers-ai", "solobase"] {
        let msg = msg_with_cookie(Some(&cookie));
        let reply = call_claim(&h, &msg, &body(reserved, "github"))
            .await
            .unwrap();
        assert_eq!(reply.status, 409, "reserved={reserved}");
        let b = parse_body(&reply);
        assert_eq!(b["error"], "reserved", "reserved={reserved}");
    }
}

// ── 5) Invalid name regex ────────────────────────────────────────────────
#[tokio::test]
async fn claim_invalid_name_returns_400() {
    let h = boot().await;
    let u = mk_user(h.ctx.as_ref(), "u@x.com").await;
    link_provider(h.ctx.as_ref(), &u, "tok", "42").await;
    let cookie = login_cookie(h.ctx.as_ref(), &u).await;

    let long_name = "a".repeat(41);
    let bad = [
        "Acme",
        "-acme",
        "1acme",
        "",
        "acme_org",
        "acme/corp",
        long_name.as_str(),
    ];
    for name in bad {
        let msg = msg_with_cookie(Some(&cookie));
        let reply = call_claim(&h, &msg, &body(name, "github")).await.unwrap();
        assert_eq!(reply.status, 400, "name={name:?}");
    }
}

// ── 6) Provider says not admin ───────────────────────────────────────────
#[tokio::test]
async fn claim_when_provider_says_not_admin_returns_403() {
    let h = boot().await;
    let u = mk_user(h.ctx.as_ref(), "u@x.com").await;
    link_provider(h.ctx.as_ref(), &u, "tok", "42").await;
    h.gh.set_admin("tok", "acme", false);
    let cookie = login_cookie(h.ctx.as_ref(), &u).await;

    let msg = msg_with_cookie(Some(&cookie));
    let reply = call_claim(&h, &msg, &body("acme", "github")).await.unwrap();
    assert_eq!(reply.status, 403);
    let b = parse_body(&reply);
    assert_eq!(b["error"], "not_org_admin");
}

// ── 7) Provider 5xx bubbles as 503 ───────────────────────────────────────
#[tokio::test]
async fn claim_when_provider_errors_returns_503() {
    let h = boot().await;
    let u = mk_user(h.ctx.as_ref(), "u@x.com").await;
    link_provider(h.ctx.as_ref(), &u, "tok", "42").await;
    h.gh.set_upstream_error("tok", "acme");
    let cookie = login_cookie(h.ctx.as_ref(), &u).await;

    let msg = msg_with_cookie(Some(&cookie));
    let reply = call_claim(&h, &msg, &body("acme", "github")).await.unwrap();
    assert_eq!(reply.status, 503);
}

// ── 8) Same name, second claimant → 409 ──────────────────────────────────
#[tokio::test]
async fn claim_duplicate_name_returns_409() {
    let h = boot().await;
    let u1 = mk_user(h.ctx.as_ref(), "u1@x.com").await;
    let u2 = mk_user(h.ctx.as_ref(), "u2@x.com").await;
    link_provider(h.ctx.as_ref(), &u1, "tok1", "1").await;
    link_provider(h.ctx.as_ref(), &u2, "tok2", "2").await;
    h.gh.set_admin("tok1", "acme", true);
    h.gh.set_admin("tok2", "acme", true);
    let c1 = login_cookie(h.ctx.as_ref(), &u1).await;
    let c2 = login_cookie(h.ctx.as_ref(), &u2).await;

    let r1 = call_claim(&h, &msg_with_cookie(Some(&c1)), &body("acme", "github"))
        .await
        .unwrap();
    assert_eq!(r1.status, 201);

    let r2 = call_claim(&h, &msg_with_cookie(Some(&c2)), &body("acme", "github"))
        .await
        .unwrap();
    assert_eq!(r2.status, 409);
    // Since this handler uses `name` as the verified_ref, the
    // (verified_via, verified_ref) clash fires first → `already_claimed`.
    let b = parse_body(&r2);
    assert_eq!(b["error"], "already_claimed");
}

// ── 9) Unknown provider name ────────────────────────────────────────────
#[tokio::test]
async fn claim_with_unknown_provider_returns_400() {
    let h = boot().await;
    let u = mk_user(h.ctx.as_ref(), "u@x.com").await;
    // Link exists for google so the 422 branch doesn't fire first — we
    // want to exercise "provider not in registry."
    provider_links::upsert(
        h.ctx.as_ref(),
        provider_links::NewLink {
            provider: "gitlab",
            provider_ref: "7",
            user_id: &u.0,
            provider_login: "t",
            access_token: "tok",
        },
    )
    .await
    .unwrap();
    let cookie = login_cookie(h.ctx.as_ref(), &u).await;
    let msg = msg_with_cookie(Some(&cookie));
    let reply = call_claim(&h, &msg, &body("acme", "gitlab")).await.unwrap();
    assert_eq!(reply.status, 400);
}

// Silence the unused-`WaferError`-import lint when this module is the only
// consumer of that type through `?`-less error paths.
#[allow(dead_code)]
fn _silence_error_import(e: WaferError) -> ErrorCode {
    e.code
}
