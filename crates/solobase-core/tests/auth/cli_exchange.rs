//! Block-integration tests for `POST /auth/cli/issue` and
//! `POST /auth/cli/exchange`.
//!
//! Covers:
//!   - `issue_*` (T7): requires a session cookie, rejects Bearer PATs,
//!     returns `{code, expires_at}`.
//!   - `exchange_*` (T8): happy path (unauth), unknown code → 401,
//!     single-use semantics, PAT usable against /auth/me.

use std::sync::Arc;

use solobase_core::blocks::auth::{
    cache::OrgAdminCache,
    handlers::{
        cli::{post_exchange, post_issue},
        me::get_me,
    },
    migrations, pat,
    providers::registry::ProviderRegistry,
    repo::{sessions, users},
    service::{hash_token, AuthServiceImpl, BlockState},
};
use wafer_core::interfaces::auth::service::{AuthService, TokenScope, UserId};
use wafer_run::{context::Context, types::Message};

use crate::common::MigrationTestCtx;

struct Harness {
    ctx: Arc<dyn Context>,
    svc: Arc<dyn AuthService>,
}

async fn boot() -> Harness {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.expect("migrations");
    let providers = Arc::new(ProviderRegistry::empty());
    let state = BlockState::new(ctx.clone())
        .with_providers(providers)
        .with_org_admin_cache(OrgAdminCache::default());
    let svc: Arc<dyn AuthService> = Arc::new(AuthServiceImpl::new(state));
    Harness { ctx, svc }
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

fn msg_for(path: &str) -> Message {
    let mut m = Message::new("create");
    m.set_meta(wafer_run::meta::META_REQ_ACTION, "create");
    m.set_meta(wafer_run::meta::META_REQ_RESOURCE, path);
    m
}

fn with_cookie(mut m: Message, raw: &str) -> Message {
    m.set_meta("http.header.cookie", format!("wafer_session={raw}"));
    m
}

fn with_bearer(mut m: Message, token: &str) -> Message {
    m.set_meta("http.header.authorization", format!("Bearer {token}"));
    m
}

fn json_of(r: &solobase_core::blocks::auth::handlers::HttpReply) -> serde_json::Value {
    serde_json::from_slice(&r.body)
        .unwrap_or_else(|e| panic!("response body is not JSON: {e} — body: {:?}", r.body))
}

// ── Task 7 ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn issue_without_session_returns_401() {
    let h = boot().await;
    let msg = msg_for("/auth/cli/issue");
    let r = post_issue(h.ctx.as_ref(), h.svc.as_ref(), &msg)
        .await
        .unwrap();
    assert_eq!(r.status, 401);
}

#[tokio::test]
async fn issue_with_session_returns_code_and_expiry() {
    let h = boot().await;
    let u = mk_user(h.ctx.as_ref(), "u@x.com").await;
    let cookie = login_cookie(h.ctx.as_ref(), &u).await;

    let msg = with_cookie(msg_for("/auth/cli/issue"), &cookie);
    let r = post_issue(h.ctx.as_ref(), h.svc.as_ref(), &msg)
        .await
        .unwrap();
    assert_eq!(r.status, 200, "body: {}", String::from_utf8_lossy(&r.body));
    let b = json_of(&r);
    let code = b["code"].as_str().expect("code present");
    // 32 random bytes → 43 base64url chars (no padding).
    assert!(code.len() >= 32, "code too short: {code}");
    assert!(b["expires_at"].is_string());
}

#[tokio::test]
async fn issue_with_bearer_pat_is_rejected_even_if_pat_would_authenticate() {
    // Spec §5: /auth/cli/issue is session-only. Allowing a PAT would let a
    // leaked publish-scope token mint a fresh CLI token and effectively
    // dodge revocation.
    let h = boot().await;
    let u = mk_user(h.ctx.as_ref(), "u@x.com").await;
    let issued = pat::issue(h.ctx.as_ref(), &u.0, "test", &[TokenScope::Publish], None)
        .await
        .expect("mint pat");

    let msg = with_bearer(msg_for("/auth/cli/issue"), &issued.raw_token);
    let r = post_issue(h.ctx.as_ref(), h.svc.as_ref(), &msg)
        .await
        .unwrap();
    assert_eq!(r.status, 401, "bearer must be rejected");
    let b = json_of(&r);
    assert_eq!(b["error"], "unauthorized");
}

// ── Task 8 — /auth/cli/exchange ──────────────────────────────────────────

/// Helper: run the full issue + exchange chain for `user`, returning the
/// raw PAT. Exercises the real code paths rather than poking the repo.
async fn issue_and_exchange(h: &Harness, user: &UserId) -> String {
    let cookie = login_cookie(h.ctx.as_ref(), user).await;
    let imsg = with_cookie(msg_for("/auth/cli/issue"), &cookie);
    let iresp = post_issue(h.ctx.as_ref(), h.svc.as_ref(), &imsg)
        .await
        .expect("issue");
    assert_eq!(iresp.status, 200, "issue status: {}", iresp.status);
    let code = json_of(&iresp)["code"].as_str().unwrap().to_string();

    let ebody = serde_json::to_vec(&serde_json::json!({ "code": code })).unwrap();
    let eresp = post_exchange(h.ctx.as_ref(), &ebody)
        .await
        .expect("exchange");
    assert_eq!(eresp.status, 200, "exchange status: {}", eresp.status);
    json_of(&eresp)["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn exchange_happy_path_returns_pat() {
    let h = boot().await;
    let u = mk_user(h.ctx.as_ref(), "u@x.com").await;
    let pat_raw = issue_and_exchange(&h, &u).await;
    assert!(pat_raw.starts_with("wafer_pat_"), "token: {pat_raw}");
}

#[tokio::test]
async fn exchange_unknown_code_returns_401() {
    let h = boot().await;
    let body = serde_json::to_vec(&serde_json::json!({ "code": "does-not-exist" })).unwrap();
    let r = post_exchange(h.ctx.as_ref(), &body).await.unwrap();
    assert_eq!(r.status, 401);
}

#[tokio::test]
async fn exchange_invalid_body_returns_400() {
    let h = boot().await;
    let r = post_exchange(h.ctx.as_ref(), b"not json").await.unwrap();
    assert_eq!(r.status, 400);
}

#[tokio::test]
async fn exchange_consumes_code_single_use() {
    let h = boot().await;
    let u = mk_user(h.ctx.as_ref(), "u@x.com").await;
    let cookie = login_cookie(h.ctx.as_ref(), &u).await;
    let imsg = with_cookie(msg_for("/auth/cli/issue"), &cookie);
    let iresp = post_issue(h.ctx.as_ref(), h.svc.as_ref(), &imsg)
        .await
        .unwrap();
    let code = json_of(&iresp)["code"].as_str().unwrap().to_string();

    let body = serde_json::to_vec(&serde_json::json!({ "code": code })).unwrap();
    let first = post_exchange(h.ctx.as_ref(), &body).await.unwrap();
    assert_eq!(first.status, 200);

    let second = post_exchange(h.ctx.as_ref(), &body).await.unwrap();
    assert_eq!(second.status, 401, "code must be single-use");
}

#[tokio::test]
async fn exchanged_pat_has_publish_scope_and_works_on_me() {
    let h = boot().await;
    let u = mk_user(h.ctx.as_ref(), "u@x.com").await;
    let pat_raw = issue_and_exchange(&h, &u).await;

    // Use the PAT against /auth/me — round-trip confirms the hash was
    // persisted correctly and the service resolves it back to the user.
    let memsg = with_bearer(msg_for("/auth/me"), &pat_raw);
    let reply = get_me(h.svc.as_ref(), &memsg).await;
    assert_eq!(reply.status, 200, "me via PAT: {:?}", reply.body);
    let b = json_of(&reply);
    assert_eq!(b["id"], u.0);
    assert_eq!(b["email"], "u@x.com");
}
