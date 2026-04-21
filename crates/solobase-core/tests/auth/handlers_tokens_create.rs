//! Integration tests for `handlers::tokens::create_token`.

use std::sync::Arc;

use solobase_core::blocks::auth::{
    handlers::tokens::create_token,
    migrations,
    repo::{pats, sessions, users},
    service::{hash_token, AuthServiceImpl, BlockState},
};
use wafer_core::interfaces::auth::service::AuthService;
use wafer_run::{context::Context, types::Message};

use crate::common::MigrationTestCtx;

fn svc(ctx: Arc<dyn Context>) -> Arc<dyn AuthService> {
    Arc::new(AuthServiceImpl::new(BlockState::for_test(ctx)))
}

async fn seed(ctx: &dyn Context, email: &str) -> (String, String) {
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

fn msg_with_cookie(raw: &str) -> Message {
    let mut m = Message::new("POST");
    m.set_meta("http.header.cookie", format!("wafer_session={raw}"));
    m
}

#[tokio::test]
async fn post_returns_raw_token_once_and_stores_hash() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.unwrap();
    let (_uid, cookie) = seed(ctx.as_ref(), "a@b.c").await;

    let service = svc(ctx.clone());
    let reply = create_token(
        ctx.as_ref(),
        service.as_ref(),
        &msg_with_cookie(&cookie),
        br#"{"name":"dev","scopes":["publish"]}"#,
    )
    .await
    .unwrap();
    assert_eq!(reply.status, 201);
    let body: serde_json::Value = serde_json::from_slice(&reply.body).unwrap();
    let raw = body["token"].as_str().unwrap();
    assert!(raw.starts_with("wafer_pat_"), "raw token is prefixed");
    assert!(body.get("id").is_some());
    assert_eq!(body["name"], "dev");
    assert_eq!(body["scopes"], serde_json::json!(["publish"]));

    // Row must exist by token_hash; handler must not leak token_hash back.
    let found = pats::find_by_token_hash(ctx.as_ref(), &hash_token(raw))
        .await
        .unwrap();
    assert!(found.is_some(), "PAT row must be persisted");
    assert!(body.get("token_hash").is_none());
}

#[tokio::test]
async fn post_without_name_returns_422() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.unwrap();
    let (_uid, cookie) = seed(ctx.as_ref(), "a@b.c").await;

    let service = svc(ctx.clone());
    let reply = create_token(
        ctx.as_ref(),
        service.as_ref(),
        &msg_with_cookie(&cookie),
        br#"{"scopes":["publish"]}"#,
    )
    .await
    .unwrap();
    assert_eq!(reply.status, 422);
}

#[tokio::test]
async fn post_without_auth_returns_401() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.unwrap();

    let service = svc(ctx.clone());
    let reply = create_token(
        ctx.as_ref(),
        service.as_ref(),
        &Message::new("POST"),
        br#"{"name":"x","scopes":["publish"]}"#,
    )
    .await
    .unwrap();
    assert_eq!(reply.status, 401);
}

#[tokio::test]
async fn post_accepts_rfc3339_expires_at() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.unwrap();
    let (_uid, cookie) = seed(ctx.as_ref(), "a@b.c").await;

    let service = svc(ctx.clone());
    let body = br#"{"name":"exp","scopes":["publish"],"expires_at":"2099-01-02T03:04:05Z"}"#;
    let reply = create_token(
        ctx.as_ref(),
        service.as_ref(),
        &msg_with_cookie(&cookie),
        body,
    )
    .await
    .unwrap();
    assert_eq!(reply.status, 201);
    let body: serde_json::Value = serde_json::from_slice(&reply.body).unwrap();
    let raw = body["token"].as_str().unwrap();
    let row = pats::find_by_token_hash(ctx.as_ref(), &hash_token(raw))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.expires_at.as_deref(), Some("2099-01-02T03:04:05Z"));
}
