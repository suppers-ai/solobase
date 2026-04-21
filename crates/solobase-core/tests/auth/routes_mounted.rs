//! Layer-2 test: verify the Plan A2 HTTP routes are mounted on
//! `SolobaseAuthBlock` and dispatch to the right handler.
//!
//! Exercises `SolobaseAuthBlock::handle` via `auth_block.handle(ctx, msg)` —
//! same entry-point production traffic uses after the HTTP adapter has
//! translated a request into a `Message`.

use std::{collections::HashMap, sync::Arc};

use solobase_core::blocks::auth::{
    block::{self, SolobaseAuthBlock},
    migrations, pat,
    repo::{local_credentials, sessions, users},
    service::hash_token,
};
use wafer_core::{clients::crypto as crypto_client, interfaces::auth::service::TokenScope};
use wafer_run::{
    block::Block,
    context::Context,
    types::{Message, MetaEntry},
    BlockRegistry, InputStream, RuntimeError,
};

use crate::common::MigrationTestCtx;

#[derive(Default)]
struct TestRegistry {
    blocks: HashMap<String, Arc<dyn Block>>,
}

impl BlockRegistry for TestRegistry {
    fn register_block(&mut self, name: &str, blk: Arc<dyn Block>) -> Result<(), RuntimeError> {
        self.blocks.insert(name.into(), blk);
        Ok(())
    }
    fn add_alias(&mut self, _: &str, _: &str) {}
    fn add_block_config(&mut self, _: &str, _: serde_json::Value) {}
}

fn http_msg(method: &str, path: &str) -> Message {
    let mut m = Message::new(method);
    m.set_meta(wafer_run::meta::META_REQ_ACTION, method);
    m.set_meta(wafer_run::meta::META_REQ_RESOURCE, path);
    m
}

#[test]
fn mounted_routes_are_declared() {
    let expected = [
        "POST /auth/login",
        "POST /auth/logout",
        "GET /auth/me",
        "GET /auth/tokens",
        "POST /auth/tokens",
        "DELETE /auth/tokens/{id}",
    ];
    let mounted = SolobaseAuthBlock::mounted_routes();
    for e in expected {
        assert!(mounted.iter().any(|r| *r == e), "missing mounted route {e}");
    }
}

async fn setup() -> (Arc<dyn Context>, Arc<dyn Block>) {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.unwrap();
    let mut registry = TestRegistry::default();
    block::register(&mut registry, ctx.clone()).expect("register");
    let auth_block = registry.blocks.get("suppers-ai/auth").cloned().unwrap();
    (ctx, auth_block)
}

fn status_of(meta: &[MetaEntry]) -> Option<u16> {
    meta.iter()
        .find(|e| e.key == wafer_run::meta::META_RESP_STATUS)
        .and_then(|e| e.value.parse().ok())
}

#[tokio::test]
async fn post_login_route_issues_303_and_cookie() {
    let (ctx, auth_block) = setup().await;
    let u = users::insert(
        ctx.as_ref(),
        users::NewUser {
            email: "a@b.c".into(),
            display_name: "A".into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .unwrap();
    let h = crypto_client::hash(ctx.as_ref(), "pw").await.unwrap();
    local_credentials::insert(ctx.as_ref(), &u.id, &h, false)
        .await
        .unwrap();

    let msg = http_msg("POST", "/auth/login");
    let input = InputStream::from_bytes(br#"{"email":"a@b.c","password":"pw"}"#.to_vec());
    let out = auth_block.handle(ctx.as_ref(), msg, input).await;
    let buf = out.collect_buffered().await.unwrap();
    assert_eq!(status_of(&buf.meta), Some(303));
    let has_cookie = buf
        .meta
        .iter()
        .any(|e| e.key.starts_with(wafer_run::meta::META_RESP_COOKIE_PREFIX));
    assert!(has_cookie, "login must emit a Set-Cookie");
}

#[tokio::test]
async fn get_me_route_returns_profile_json() {
    let (ctx, auth_block) = setup().await;
    let u = users::insert(
        ctx.as_ref(),
        users::NewUser {
            email: "a@b.c".into(),
            display_name: "A".into(),
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

    let mut msg = http_msg("GET", "/auth/me");
    msg.set_meta("http.header.cookie", format!("wafer_session={raw}"));
    let out = auth_block
        .handle(ctx.as_ref(), msg, InputStream::empty())
        .await;
    let buf = out.collect_buffered().await.unwrap();
    assert_eq!(status_of(&buf.meta), Some(200));
    let body: serde_json::Value = serde_json::from_slice(&buf.body).unwrap();
    assert_eq!(body["email"], "a@b.c");
    assert_eq!(body["id"], u.id);
}

#[tokio::test]
async fn tokens_list_route_dispatches() {
    let (ctx, auth_block) = setup().await;
    let u = users::insert(
        ctx.as_ref(),
        users::NewUser {
            email: "a@b.c".into(),
            display_name: "A".into(),
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
    pat::issue(ctx.as_ref(), &u.id, "cli", &[TokenScope::Publish], None)
        .await
        .unwrap();

    let mut msg = http_msg("GET", "/auth/tokens");
    msg.set_meta("http.header.cookie", format!("wafer_session={raw}"));
    let out = auth_block
        .handle(ctx.as_ref(), msg, InputStream::empty())
        .await;
    let buf = out.collect_buffered().await.unwrap();
    assert_eq!(status_of(&buf.meta), Some(200));
    let body: serde_json::Value = serde_json::from_slice(&buf.body).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn delete_token_route_extracts_id_from_path() {
    let (ctx, auth_block) = setup().await;
    let u = users::insert(
        ctx.as_ref(),
        users::NewUser {
            email: "a@b.c".into(),
            display_name: "A".into(),
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
    let issued = pat::issue(ctx.as_ref(), &u.id, "t", &[TokenScope::Publish], None)
        .await
        .unwrap();
    let hash = hash_token(&issued.raw_token);
    let id: String = hash.iter().map(|b| format!("{b:02x}")).collect();

    let mut msg = http_msg("DELETE", &format!("/auth/tokens/{id}"));
    msg.set_meta("http.header.cookie", format!("wafer_session={raw}"));
    let out = auth_block
        .handle(ctx.as_ref(), msg, InputStream::empty())
        .await;
    let buf = out.collect_buffered().await.unwrap();
    assert_eq!(status_of(&buf.meta), Some(204));
}

#[tokio::test]
async fn service_op_still_dispatched_when_http_route_doesnt_match() {
    // require_user with no creds → Unauthenticated error, same as before T11.
    let (ctx, auth_block) = setup().await;
    let msg = Message::new(wafer_run::common::ServiceOp::AUTH_REQUIRE_USER);
    let out = auth_block
        .handle(ctx.as_ref(), msg, InputStream::empty())
        .await;
    let err = out
        .collect_buffered()
        .await
        .expect_err("expected Unauthenticated")
        .into();
    let err: wafer_run::types::WaferError = err;
    assert_eq!(err.code, wafer_run::types::ErrorCode::Unauthenticated);
}
