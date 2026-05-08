//! PR 1: confirm `migrations::apply` runs from `AuthBlock::lifecycle(Init)`.
//!
//! `migrations_001.rs` already exercises `migrations::apply` directly. The
//! point of this test is the WIRING — that the AuthBlock's lifecycle hook
//! actually triggers it.

use solobase_core::blocks::auth::AuthBlock;
use wafer_core::clients::database as db;
use wafer_run::{
    block::Block,
    types::{LifecycleEvent, LifecycleType},
};

use crate::common::MigrationTestCtx;

/// Sanity: a bare context (no migrations applied) does not have the Plan A2
/// tables. Establishes the precondition for the `init_lifecycle_*` tests.
#[tokio::test]
async fn bare_context_has_no_plan_a2_tables() {
    let ctx = MigrationTestCtx::new();
    let res = db::query_raw(
        &ctx,
        "SELECT name FROM sqlite_master WHERE type='table' AND name = 'suppers_ai__auth__local_credentials'",
        &[],
    )
    .await
    .expect("probe sqlite_master");
    assert!(res.is_empty(), "expected no plan A2 table on bare ctx");
}

#[tokio::test]
async fn init_lifecycle_creates_local_credentials_table() {
    let ctx = MigrationTestCtx::new();
    let block = AuthBlock::default();
    block
        .lifecycle(
            &ctx,
            LifecycleEvent {
                event_type: LifecycleType::Init,
                data: vec![],
            },
        )
        .await
        .expect("lifecycle Init");

    let res = db::query_raw(
        &ctx,
        "SELECT name FROM sqlite_master WHERE type='table' AND name = 'suppers_ai__auth__local_credentials'",
        &[],
    )
    .await
    .expect("probe sqlite_master");
    assert_eq!(
        res.len(),
        1,
        "local_credentials table missing after Init lifecycle"
    );
}

#[tokio::test]
async fn init_lifecycle_creates_bootstrap_tokens_table() {
    let ctx = MigrationTestCtx::new();
    let block = AuthBlock::default();
    block
        .lifecycle(
            &ctx,
            LifecycleEvent {
                event_type: LifecycleType::Init,
                data: vec![],
            },
        )
        .await
        .expect("lifecycle Init");

    let res = db::query_raw(
        &ctx,
        "SELECT name FROM sqlite_master WHERE type='table' AND name = 'suppers_ai__auth__bootstrap_tokens'",
        &[],
    )
    .await
    .expect("probe sqlite_master");
    assert_eq!(
        res.len(),
        1,
        "bootstrap_tokens table missing after Init lifecycle"
    );
}

/// Re-running Init must be a no-op rather than an error — boot sequences
/// often call lifecycle hooks more than once (e.g. when blocks reload).
/// Migration 001 uses `CREATE TABLE IF NOT EXISTS`; this test guards that
/// the lifecycle wiring respects that idempotence.
#[tokio::test]
async fn init_lifecycle_is_idempotent() {
    let ctx = MigrationTestCtx::new();
    let block = AuthBlock::default();
    let event = LifecycleEvent {
        event_type: LifecycleType::Init,
        data: vec![],
    };
    block
        .lifecycle(&ctx, event.clone())
        .await
        .expect("first init");
    block
        .lifecycle(&ctx, event)
        .await
        .expect("second init must succeed (idempotent)");
}
