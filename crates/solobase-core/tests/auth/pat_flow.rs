//! Layer-2 end-to-end PAT flow test.
//!
//! Cluster B covers PAT issuance, listing, and revocation in isolation.
//! This test stitches them into one sequence to catch pipeline regressions
//! (id hex vs raw, bearer parsing, revocation actually deleting the row):
//!
//! 1. login → wafer_session cookie
//! 2. POST /auth/tokens  → 201 + raw token + id
//! 3. GET  /auth/tokens  → list contains the new PAT (no token_hash leakage)
//! 4. GET  /auth/me with Bearer <raw>  → 200 (PAT authenticates request)
//! 5. DELETE /auth/tokens/{id}  → 204
//! 6. GET  /auth/me with same Bearer   → 401 (row is gone)

use std::{collections::HashMap, sync::Arc};

use solobase_core::blocks::auth::{
    block, migrations,
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

fn set_cookie(meta: &[MetaEntry], name: &str) -> Option<String> {
    let prefix = wafer_run::meta::META_RESP_COOKIE_PREFIX;
    meta.iter()
        .filter(|e| e.key.starts_with(prefix))
        .map(|e| e.value.clone())
        .find(|v| v.starts_with(&format!("{name}=")))
}

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
async fn issue_list_use_revoke_round_trip() {
    let (ctx, auth_block) = setup().await;
    seed_user_with_password(ctx.as_ref(), "a@b.c", "pw").await;

    // 1. Login and capture the session cookie header.
    let login_out = auth_block
        .handle(
            ctx.as_ref(),
            http_msg("POST", "/auth/login"),
            InputStream::from_bytes(br#"{"email":"a@b.c","password":"pw"}"#.to_vec()),
        )
        .await;
    let login = login_out.collect_buffered().await.unwrap();
    assert_eq!(status_of(&login.meta), Some(303));
    let session_cookie = cookie_pair(
        &set_cookie(&login.meta, "wafer_session")
            .expect("login must emit wafer_session Set-Cookie"),
    );

    // 2. POST /auth/tokens — create a PAT. Returns 201 + raw token + id.
    let mut issue_msg = http_msg("POST", "/auth/tokens");
    issue_msg.set_meta("http.header.cookie", session_cookie.clone());
    let issue_out = auth_block
        .handle(
            ctx.as_ref(),
            issue_msg,
            InputStream::from_bytes(br#"{"name":"cli","scopes":["publish"]}"#.to_vec()),
        )
        .await;
    let issue = issue_out.collect_buffered().await.unwrap();
    assert_eq!(status_of(&issue.meta), Some(201));
    let issue_body: serde_json::Value = serde_json::from_slice(&issue.body).unwrap();
    let raw_pat = issue_body["token"]
        .as_str()
        .expect("raw token present")
        .to_string();
    let pat_id = issue_body["id"].as_str().expect("id present").to_string();
    assert!(raw_pat.starts_with("wafer_pat_"));
    assert!(
        issue_body.get("token_hash").is_none(),
        "response must not leak token_hash"
    );

    // 3. GET /auth/tokens — the list contains exactly this one PAT.
    let mut list_msg = http_msg("GET", "/auth/tokens");
    list_msg.set_meta("http.header.cookie", session_cookie.clone());
    let list_out = auth_block
        .handle(ctx.as_ref(), list_msg, InputStream::empty())
        .await;
    let list = list_out.collect_buffered().await.unwrap();
    assert_eq!(status_of(&list.meta), Some(200));
    let list_body: serde_json::Value = serde_json::from_slice(&list.body).unwrap();
    let arr = list_body.as_array().expect("tokens list is a JSON array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "cli");
    assert_eq!(arr[0]["id"], pat_id);
    assert!(arr[0].get("token_hash").is_none());

    // 4. Use the raw PAT as a Bearer credential on /auth/me — 200.
    let mut bearer_me = http_msg("GET", "/auth/me");
    bearer_me.set_meta("http.header.authorization", format!("Bearer {raw_pat}"));
    let bearer_out = auth_block
        .handle(ctx.as_ref(), bearer_me, InputStream::empty())
        .await;
    let bearer = bearer_out.collect_buffered().await.unwrap();
    assert_eq!(
        status_of(&bearer.meta),
        Some(200),
        "PAT as Bearer must authenticate /auth/me"
    );
    let bearer_body: serde_json::Value = serde_json::from_slice(&bearer.body).unwrap();
    assert_eq!(bearer_body["email"], "a@b.c");

    // 5. DELETE /auth/tokens/{id} with the session cookie — 204.
    let mut del_msg = http_msg("DELETE", &format!("/auth/tokens/{pat_id}"));
    del_msg.set_meta("http.header.cookie", session_cookie.clone());
    let del_out = auth_block
        .handle(ctx.as_ref(), del_msg, InputStream::empty())
        .await;
    let del = del_out.collect_buffered().await.unwrap();
    assert_eq!(status_of(&del.meta), Some(204));

    // 6. Same Bearer now rejected — PAT row is gone.
    let mut stale_me = http_msg("GET", "/auth/me");
    stale_me.set_meta("http.header.authorization", format!("Bearer {raw_pat}"));
    let stale_out = auth_block
        .handle(ctx.as_ref(), stale_me, InputStream::empty())
        .await;
    let stale = stale_out.collect_buffered().await.unwrap();
    assert_eq!(
        status_of(&stale.meta),
        Some(401),
        "revoked PAT must no longer authenticate"
    );
}
