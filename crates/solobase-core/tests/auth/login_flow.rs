//! Layer-2 end-to-end login flow test.
//!
//! Cluster B's `handlers_login`, `handlers_me`, and `routes_mounted` tests
//! exercise each step in isolation. This test stitches them into a single
//! sequence:
//!
//! 1. `POST /auth/login` → 303 + `Set-Cookie: wafer_session=...`
//! 2. `GET  /auth/me` with the cookie → 200 + profile JSON
//! 3. `POST /auth/logout` with the cookie → 204 + `Max-Age=0` clear cookie
//! 4. `GET  /auth/me` with the same cookie → 401 (session row is gone)
//!
//! The goal is to catch regressions where each step works alone but the
//! pipeline breaks (cookie name mismatch between issue and extract, session
//! row deletion wired to the wrong token, etc).

use std::{collections::HashMap, sync::Arc};

use solobase_core::blocks::auth::{
    block::{self, SolobaseAuthBlock},
    migrations,
    repo::{local_credentials, users},
};
use wafer_core::clients::crypto as crypto_client;
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

fn status_of(meta: &[MetaEntry]) -> Option<u16> {
    meta.iter()
        .find(|e| e.key == wafer_run::meta::META_RESP_STATUS)
        .and_then(|e| e.value.parse().ok())
}

/// Extract a single `Set-Cookie` value by cookie name from the response meta.
/// Cluster B's handlers emit Set-Cookie as individual meta entries under the
/// `META_RESP_COOKIE_PREFIX` namespace (not a flat header).
fn set_cookie(meta: &[MetaEntry], name: &str) -> Option<String> {
    let prefix = wafer_run::meta::META_RESP_COOKIE_PREFIX;
    meta.iter()
        .filter(|e| e.key.starts_with(prefix))
        .map(|e| e.value.clone())
        .find(|v| v.starts_with(&format!("{name}=")))
}

/// Parse the cookie value (`wafer_session=raw...`) out of a full Set-Cookie
/// header, stripping attributes (`; HttpOnly; Path=/` etc).
fn cookie_pair(set_cookie: &str) -> String {
    set_cookie
        .split(';')
        .next()
        .expect("Set-Cookie always has a name=value segment")
        .to_string()
}

async fn setup() -> (Arc<dyn Context>, Arc<dyn Block>) {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.unwrap();
    let mut registry = TestRegistry::default();
    block::register(&mut registry, ctx.clone()).expect("register auth block");
    let auth_block = registry.blocks.get("suppers-ai/auth").cloned().unwrap();
    let _ = SolobaseAuthBlock::mounted_routes(); // sanity: routes compile-visible.
    (ctx, auth_block)
}

async fn seed_user_with_password(ctx: &dyn Context, email: &str, password: &str) -> String {
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
    .expect("insert user");
    let hash = crypto_client::hash(ctx, password).await.expect("hash");
    local_credentials::insert(ctx, &u.id, &hash, false)
        .await
        .expect("insert credentials");
    u.id
}

#[tokio::test]
async fn end_to_end_login_then_me_then_logout_then_me_returns_401() {
    let (ctx, auth_block) = setup().await;
    seed_user_with_password(ctx.as_ref(), "a@b.c", "pw").await;

    // Step 1: login — expect 303 + Set-Cookie with the session token.
    let login_out = auth_block
        .handle(
            ctx.as_ref(),
            http_msg("POST", "/auth/login"),
            InputStream::from_bytes(br#"{"email":"a@b.c","password":"pw"}"#.to_vec()),
        )
        .await;
    let login = login_out.collect_buffered().await.unwrap();
    assert_eq!(status_of(&login.meta), Some(303), "login status");
    let session_cookie =
        set_cookie(&login.meta, "wafer_session").expect("login must emit wafer_session Set-Cookie");
    assert!(session_cookie.contains("HttpOnly"));
    assert!(session_cookie.contains("SameSite=Lax"));
    assert!(session_cookie.contains("Path=/"));
    let cookie_header = cookie_pair(&session_cookie);
    assert!(cookie_header.starts_with("wafer_session=wafer_session_"));

    // Step 2: GET /auth/me with the session cookie — 200 + profile.
    let mut me_msg = http_msg("GET", "/auth/me");
    me_msg.set_meta("http.header.cookie", cookie_header.clone());
    let me_out = auth_block
        .handle(ctx.as_ref(), me_msg, InputStream::empty())
        .await;
    let me = me_out.collect_buffered().await.unwrap();
    assert_eq!(status_of(&me.meta), Some(200), "me status");
    let body: serde_json::Value = serde_json::from_slice(&me.body).unwrap();
    assert_eq!(body["email"], "a@b.c");

    // Step 3: POST /auth/logout with the same cookie — 204 + Max-Age=0 cookie.
    let mut out_msg = http_msg("POST", "/auth/logout");
    out_msg.set_meta("http.header.cookie", cookie_header.clone());
    let out_out = auth_block
        .handle(ctx.as_ref(), out_msg, InputStream::empty())
        .await;
    let out = out_out.collect_buffered().await.unwrap();
    assert_eq!(status_of(&out.meta), Some(204), "logout status");
    let clear =
        set_cookie(&out.meta, "wafer_session").expect("logout must emit a clearing Set-Cookie");
    assert!(clear.contains("Max-Age=0"), "logout cookie must expire now");

    // Step 4: /auth/me with the now-invalidated cookie must fail closed.
    let mut stale_msg = http_msg("GET", "/auth/me");
    stale_msg.set_meta("http.header.cookie", cookie_header);
    let stale_out = auth_block
        .handle(ctx.as_ref(), stale_msg, InputStream::empty())
        .await;
    let stale = stale_out.collect_buffered().await.unwrap();
    assert_eq!(
        status_of(&stale.meta),
        Some(401),
        "me with revoked session must return 401"
    );
}
