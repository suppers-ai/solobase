//! End-to-end: register the `suppers-ai/auth` block against a minimal
//! in-memory `BlockRegistry`, dispatch an `auth.require_user` Message
//! through the registered block, and verify the response body contains
//! the expected `user_id`.
//!
//! This exercises the full chain: `blocks::auth::block::register` →
//! `wafer_core::service_blocks::auth::AuthBlock::handle` →
//! `wafer_core::interfaces::auth::handler::handle_message` →
//! `AuthServiceImpl::require_user` → repo layer → in-memory SQLite.

use std::{collections::HashMap, sync::Arc};

use solobase_core::blocks::auth::{
    block, migrations,
    repo::{sessions, users},
    service::hash_token,
};
use wafer_run::{
    block::Block,
    common::ServiceOp,
    context::Context,
    types::{LifecycleEvent, Message, WaferError},
    BlockRegistry, InputStream, OutputStream, RuntimeError,
};

use crate::common::MigrationTestCtx;

/// Minimal `BlockRegistry` that just stashes the registered block so we can
/// invoke `handle()` on it directly. We don't need aliases or config for
/// this test.
#[derive(Default)]
struct TestRegistry {
    blocks: HashMap<String, Arc<dyn Block>>,
}

impl BlockRegistry for TestRegistry {
    fn register_block(&mut self, name: &str, block: Arc<dyn Block>) -> Result<(), RuntimeError> {
        if self.blocks.contains_key(name) {
            return Err(RuntimeError::DuplicateBlock { name: name.into() });
        }
        self.blocks.insert(name.into(), block);
        Ok(())
    }

    fn add_alias(&mut self, _alias: &str, _target: &str) {}

    fn add_block_config(&mut self, _name: &str, _config: serde_json::Value) {}
}

#[tokio::test]
async fn auth_block_responds_to_require_user_message() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.expect("migrations");

    // Seed a user + a live session token so `require_user` has something to
    // resolve the cookie to.
    let u = users::insert(
        ctx.as_ref(),
        users::NewUser {
            email: "e@e.com".into(),
            display_name: "E".into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .expect("seed user");

    let raw = "s-raw";
    sessions::insert(
        ctx.as_ref(),
        sessions::NewSession {
            token_hash: hash_token(raw),
            user_id: u.id.clone(),
            expires_at: "9999-01-01T00:00:00Z".into(),
        },
    )
    .await
    .expect("seed session");

    // Register the block.
    let mut registry = TestRegistry::default();
    block::register(&mut registry, ctx.clone()).expect("register auth block");
    let auth_block = registry
        .blocks
        .get("suppers-ai/auth")
        .cloned()
        .expect("auth block registered under suppers-ai/auth");

    // Sanity: BlockInfo advertises the right name + category.
    let info = auth_block.info();
    assert_eq!(info.name, "suppers-ai/auth");

    // Dispatch an `auth.require_user` Message carrying the session cookie.
    // `Message::header()` reads `http.header.<name>` meta keys, so we set it
    // that way — same convention the real HTTP adapter uses.
    let mut msg = Message::new(ServiceOp::AUTH_REQUIRE_USER);
    msg.set_meta("http.header.cookie", format!("wafer_session={raw}"));

    let out: OutputStream = auth_block
        .handle(ctx.as_ref(), msg, InputStream::empty())
        .await;
    let buf = out
        .collect_buffered()
        .await
        .expect("handler returned non-error terminal");

    let body: serde_json::Value = serde_json::from_slice(&buf.body).expect("response body is JSON");
    assert_eq!(
        body.get("user_id").and_then(|v| v.as_str()),
        Some(u.id.as_str()),
        "require_user response must echo the looked-up user id: {body:?}",
    );
}

#[tokio::test]
async fn auth_block_missing_creds_returns_unauthenticated_error() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.expect("migrations");

    let mut registry = TestRegistry::default();
    block::register(&mut registry, ctx.clone()).expect("register auth block");
    let auth_block = registry.blocks.get("suppers-ai/auth").cloned().unwrap();

    let msg = Message::new(ServiceOp::AUTH_REQUIRE_USER);
    let out = auth_block
        .handle(ctx.as_ref(), msg, InputStream::empty())
        .await;
    let err: WaferError = out
        .collect_buffered()
        .await
        .expect_err("expected error terminal")
        .into();
    assert_eq!(err.code, wafer_run::types::ErrorCode::Unauthenticated);
}

#[tokio::test]
async fn auth_block_lifecycle_init_is_noop() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    let mut registry = TestRegistry::default();
    block::register(&mut registry, ctx.clone()).expect("register auth block");
    let auth_block = registry.blocks.get("suppers-ai/auth").cloned().unwrap();

    // The unified block's `lifecycle` is a no-op (migrations are run by the
    // host before register). This just verifies it doesn't panic.
    auth_block
        .lifecycle(
            ctx.as_ref(),
            LifecycleEvent {
                event_type: wafer_run::types::LifecycleType::Init,
                data: Vec::new(),
            },
        )
        .await
        .expect("lifecycle init is infallible");
}
