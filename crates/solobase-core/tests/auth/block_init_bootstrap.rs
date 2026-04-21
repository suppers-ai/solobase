//! `SolobaseAuthBlock::lifecycle(Init)` runs migrations + bootstrap with the
//! configured env. This covers Plan A2 Task 5: wiring bootstrap into block
//! init so a fresh solobase instance provisions the first admin without a
//! manual migration + seed step.

use std::{collections::HashMap, sync::Arc};

use solobase_core::blocks::auth::{
    block,
    config::AuthConfig,
    repo::{local_credentials, users},
};
use wafer_run::{
    block::Block,
    context::Context,
    types::{LifecycleEvent, LifecycleType},
    BlockRegistry, RuntimeError,
};

use crate::common::MigrationTestCtx;

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

async fn init(
    block: &Arc<dyn Block>,
    ctx: &dyn Context,
) -> Result<(), wafer_run::types::WaferError> {
    block
        .lifecycle(
            ctx,
            LifecycleEvent {
                event_type: LifecycleType::Init,
                data: Vec::new(),
            },
        )
        .await
}

#[tokio::test]
async fn init_runs_migrations_and_bootstraps_admin_when_env_set() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    let cfg = AuthConfig::from_env_for_test(&[
        ("SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_EMAIL", "a@x.io"),
        ("SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_PASSWORD", "pw"),
    ]);

    let mut registry = TestRegistry::default();
    block::register_with_config(&mut registry, ctx.clone(), cfg).expect("register");

    let b = registry
        .blocks
        .get("suppers-ai/auth")
        .cloned()
        .expect("block");
    init(&b, ctx.as_ref())
        .await
        .expect("init runs migrations + bootstrap");

    let u = users::find_by_email(ctx.as_ref(), "a@x.io")
        .await
        .expect("lookup")
        .expect("admin created on init");
    assert_eq!(u.role, "admin");
    let creds = local_credentials::find_by_user_id(ctx.as_ref(), &u.id)
        .await
        .expect("lookup creds")
        .expect("credentials created");
    assert!(!creds.password_hash.is_empty());
}

#[tokio::test]
async fn init_is_idempotent_when_users_already_exist() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    let cfg = AuthConfig::from_env_for_test(&[
        ("SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_EMAIL", "a@x.io"),
        ("SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_PASSWORD", "pw"),
    ]);

    let mut registry = TestRegistry::default();
    block::register_with_config(&mut registry, ctx.clone(), cfg).expect("register");
    let b = registry
        .blocks
        .get("suppers-ai/auth")
        .cloned()
        .expect("block");

    // First Init: creates admin.
    init(&b, ctx.as_ref()).await.expect("first init");
    // Second Init: should be a no-op (migrations idempotent, bootstrap skipped).
    init(&b, ctx.as_ref()).await.expect("second init");

    assert_eq!(
        users::count(ctx.as_ref()).await.expect("count"),
        1,
        "bootstrap must not re-create the admin on subsequent inits"
    );
}

#[tokio::test]
async fn init_is_noop_when_no_bootstrap_env() {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    let mut registry = TestRegistry::default();
    block::register(&mut registry, ctx.clone()).expect("register");
    let b = registry
        .blocks
        .get("suppers-ai/auth")
        .cloned()
        .expect("block");

    init(&b, ctx.as_ref()).await.expect("init");

    assert_eq!(
        users::count(ctx.as_ref()).await.expect("count"),
        0,
        "no env means no seed"
    );
}
