//! Integration tests for `handlers::me::get_me`.

use std::sync::Arc;

use solobase_core::blocks::auth::{
    handlers::me::get_me,
    migrations,
    repo::{sessions, users},
    service::{hash_token, AuthServiceImpl, BlockState},
};
use wafer_core::interfaces::auth::service::AuthService;
use wafer_run::{context::Context, types::Message};

use crate::common::MigrationTestCtx;

fn svc(ctx: Arc<dyn Context>) -> impl AuthService {
    AuthServiceImpl::new(BlockState::for_test(ctx))
}

#[tokio::test]
async fn returns_profile_with_empty_orgs_when_authenticated_via_cookie() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.expect("migrations");
    let u = users::insert(
        ctx.as_ref(),
        users::NewUser {
            email: "a@b.c".into(),
            display_name: "Al".into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .unwrap();
    let raw = "sess-raw";
    sessions::insert(
        ctx.as_ref(),
        sessions::NewSession {
            token_hash: hash_token(raw),
            user_id: u.id.clone(),
            expires_at: "9999-01-01T00:00:00Z".into(),
        },
    )
    .await
    .unwrap();

    let mut msg = Message::new("GET");
    msg.set_meta("http.header.cookie", format!("wafer_session={raw}"));

    let service = svc(ctx.clone());
    let reply = get_me(&service, &msg).await;
    assert_eq!(reply.status, 200);
    let body: serde_json::Value = serde_json::from_slice(&reply.body).unwrap();
    assert_eq!(body["id"], u.id);
    assert_eq!(body["email"], "a@b.c");
    assert_eq!(body["display_name"], "Al");
    assert_eq!(body["role"], "user");
    assert_eq!(body["orgs"], serde_json::json!([]));
}

#[tokio::test]
async fn returns_401_without_credentials() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.expect("migrations");

    let msg = Message::new("GET");
    let service = svc(ctx);
    let reply = get_me(&service, &msg).await;
    assert_eq!(reply.status, 401);
    let body: serde_json::Value = serde_json::from_slice(&reply.body).unwrap();
    assert_eq!(body["error"], "unauthorized");
}
