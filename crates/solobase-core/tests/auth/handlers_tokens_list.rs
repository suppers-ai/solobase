//! Integration tests for `handlers::tokens::list_tokens`.

use std::sync::Arc;

use solobase_core::blocks::auth::{
    handlers::tokens::list_tokens,
    migrations, pat,
    repo::{pats, sessions, users},
    service::{hash_token, AuthServiceImpl, BlockState},
};
use wafer_core::interfaces::auth::service::{AuthService, TokenScope};
use wafer_run::{context::Context, types::Message};

use crate::common::MigrationTestCtx;

fn svc(ctx: Arc<dyn Context>) -> Arc<dyn AuthService> {
    Arc::new(AuthServiceImpl::new(BlockState::for_test(ctx)))
}

async fn seed_user_and_session(
    ctx: &dyn Context,
    email: &str,
) -> (String, String) {
    let u = users::insert(
        ctx,
        users::NewUser {
            email: email.into(),
            display_name: "T".into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .unwrap();
    let raw = format!("sess-{email}");
    sessions::insert(
        ctx,
        sessions::NewSession {
            token_hash: hash_token(&raw),
            user_id: u.id.clone(),
            expires_at: "9999-01-01T00:00:00Z".into(),
        },
    )
    .await
    .unwrap();
    (u.id, raw)
}

#[tokio::test]
async fn list_returns_callers_pats_without_token_hash() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.unwrap();

    let (uid, cookie) = seed_user_and_session(ctx.as_ref(), "a@b.c").await;
    let _ = pat::issue(ctx.as_ref(), &uid, "cli", &[TokenScope::Publish], None)
        .await
        .unwrap();

    let mut msg = Message::new("GET");
    msg.set_meta("http.header.cookie", format!("wafer_session={cookie}"));

    let service = svc(ctx.clone());
    let reply = list_tokens(ctx.as_ref(), service.as_ref(), &msg)
        .await
        .unwrap();
    assert_eq!(reply.status, 200);
    let body: serde_json::Value = serde_json::from_slice(&reply.body).unwrap();
    let arr = body.as_array().expect("JSON array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "cli");
    assert_eq!(arr[0]["scopes"], serde_json::json!(["publish"]));
    assert!(arr[0].get("token_hash").is_none(), "must not expose hash");
    assert!(arr[0].get("token").is_none(), "must not leak raw token");
    assert!(
        arr[0]["id"].as_str().unwrap().len() == 64,
        "id is hex-encoded sha256"
    );
}

#[tokio::test]
async fn list_excludes_other_users_pats() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.unwrap();

    let (uid_a, cookie_a) = seed_user_and_session(ctx.as_ref(), "a@b.c").await;
    let (uid_b, _cookie_b) = seed_user_and_session(ctx.as_ref(), "b@c.d").await;
    pat::issue(ctx.as_ref(), &uid_a, "mine", &[TokenScope::Publish], None)
        .await
        .unwrap();
    pat::issue(
        ctx.as_ref(),
        &uid_b,
        "theirs",
        &[TokenScope::Publish],
        None,
    )
    .await
    .unwrap();

    let mut msg = Message::new("GET");
    msg.set_meta("http.header.cookie", format!("wafer_session={cookie_a}"));
    let service = svc(ctx.clone());
    let reply = list_tokens(ctx.as_ref(), service.as_ref(), &msg)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&reply.body).unwrap();
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "mine");

    // sanity — repo sees both
    let all = pats::list_for_user(ctx.as_ref(), &uid_b).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, "theirs");
}

#[tokio::test]
async fn list_returns_401_when_unauthenticated() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.unwrap();
    let service = svc(ctx.clone());
    let msg = Message::new("GET");
    let reply = list_tokens(ctx.as_ref(), service.as_ref(), &msg)
        .await
        .unwrap();
    assert_eq!(reply.status, 401);
}
