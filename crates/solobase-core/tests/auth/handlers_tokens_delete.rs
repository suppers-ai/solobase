//! Integration tests for `handlers::tokens::delete_token`.

use std::sync::Arc;

use solobase_core::blocks::auth::{
    handlers::tokens::delete_token,
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

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

fn msg_with_cookie(raw: &str) -> Message {
    let mut m = Message::new("DELETE");
    m.set_meta("http.header.cookie", format!("wafer_session={raw}"));
    m
}

#[tokio::test]
async fn owner_can_delete_returns_204() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.unwrap();
    let (uid, cookie) = seed(ctx.as_ref(), "a@b.c").await;
    let issued = pat::issue(ctx.as_ref(), &uid, "t", &[TokenScope::Publish], None)
        .await
        .unwrap();
    let id = hex(&hash_token(&issued.raw_token));

    let service = svc(ctx.clone());
    let reply = delete_token(
        ctx.as_ref(),
        service.as_ref(),
        &msg_with_cookie(&cookie),
        &id,
    )
    .await
    .unwrap();
    assert_eq!(reply.status, 204);
    assert!(
        pats::find_by_token_hash(ctx.as_ref(), &hash_token(&issued.raw_token))
            .await
            .unwrap()
            .is_none(),
        "row must be gone after delete"
    );
}

#[tokio::test]
async fn non_owner_gets_404_and_row_is_preserved() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.unwrap();
    let (uid_victim, _vc) = seed(ctx.as_ref(), "victim@b.c").await;
    let (_uid_attacker, cookie_attacker) = seed(ctx.as_ref(), "attacker@b.c").await;
    let issued = pat::issue(ctx.as_ref(), &uid_victim, "t", &[TokenScope::Publish], None)
        .await
        .unwrap();
    let id = hex(&hash_token(&issued.raw_token));

    let service = svc(ctx.clone());
    let reply = delete_token(
        ctx.as_ref(),
        service.as_ref(),
        &msg_with_cookie(&cookie_attacker),
        &id,
    )
    .await
    .unwrap();
    assert_eq!(reply.status, 404);
    // Row must still exist.
    assert!(
        pats::find_by_token_hash(ctx.as_ref(), &hash_token(&issued.raw_token))
            .await
            .unwrap()
            .is_some(),
        "victim's row must be untouched"
    );
}

#[tokio::test]
async fn unknown_id_returns_404() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.unwrap();
    let (_uid, cookie) = seed(ctx.as_ref(), "a@b.c").await;

    let service = svc(ctx.clone());
    let bogus = hex(&[0u8; 32]);
    let reply = delete_token(
        ctx.as_ref(),
        service.as_ref(),
        &msg_with_cookie(&cookie),
        &bogus,
    )
    .await
    .unwrap();
    assert_eq!(reply.status, 404);
}

#[tokio::test]
async fn malformed_id_returns_404() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.unwrap();
    let (_uid, cookie) = seed(ctx.as_ref(), "a@b.c").await;

    let service = svc(ctx.clone());
    let reply = delete_token(
        ctx.as_ref(),
        service.as_ref(),
        &msg_with_cookie(&cookie),
        "not-hex",
    )
    .await
    .unwrap();
    assert_eq!(reply.status, 404);
}

#[tokio::test]
async fn unauthenticated_returns_401() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.unwrap();
    let service = svc(ctx.clone());
    let id = hex(&[0u8; 32]);
    let reply = delete_token(ctx.as_ref(), service.as_ref(), &Message::new("DELETE"), &id)
        .await
        .unwrap();
    assert_eq!(reply.status, 401);
}
