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
    handlers::cli::post_issue,
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
    let issued = pat::issue(
        h.ctx.as_ref(),
        &u.0,
        "test",
        &[TokenScope::Publish],
        None,
    )
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
