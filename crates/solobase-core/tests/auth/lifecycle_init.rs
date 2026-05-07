//! `AuthBlock::lifecycle(Init)` must apply the auth-block migrations.
//!
//! Drives `AuthBlock` directly against a bare context that does NOT
//! pre-apply migrations. After `lifecycle(Init)` returns, the §3 schema
//! tables must exist — exercising the `db::ddl` path through the same
//! WRAP-typed client production uses (the regression that closed PR #90
//! was that migrations went through the admin-only `__raw_sql__` path,
//! tripping browser-WASM boot under WRAP).

use solobase_core::blocks::auth::AuthBlock;
use wafer_core::clients::database as db;
use wafer_run::{
    block::Block,
    types::{LifecycleEvent, LifecycleType},
};

use crate::common::MigrationTestCtx;

const EXPECTED_TABLES: &[&str] = &[
    "suppers_ai__auth__users",
    "suppers_ai__auth__local_credentials",
    "suppers_ai__auth__provider_links",
    "suppers_ai__auth__orgs",
    "suppers_ai__auth__sessions",
    "suppers_ai__auth__personal_access_tokens",
    "suppers_ai__auth__cli_exchange_codes",
    "suppers_ai__auth__bootstrap_tokens",
];

#[tokio::test]
async fn init_lifecycle_applies_auth_migrations() {
    // Bare context — no pre-applied migrations. `MigrationTestCtx::new()`
    // routes only `wafer-run/database` and `wafer-run/crypto`, leaving
    // `wafer-run/config` to fall back, which is what production also does
    // when no config block is registered for the auth backend key.
    let ctx = MigrationTestCtx::new();

    let block = AuthBlock::default();
    block
        .lifecycle(
            &ctx,
            LifecycleEvent {
                event_type: LifecycleType::Init,
                data: Vec::new(),
            },
        )
        .await
        .expect("Init lifecycle must succeed");

    let rows = db::query_raw(
        &ctx,
        "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'suppers_ai__auth__%'",
        &[],
    )
    .await
    .expect("query sqlite_master after Init");

    let names: Vec<String> = rows
        .iter()
        .filter_map(|r| {
            r.data
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect();

    for t in EXPECTED_TABLES {
        assert!(
            names.contains(&t.to_string()),
            "table missing after Init: {t} (got {names:?})"
        );
    }
}

#[tokio::test]
async fn init_lifecycle_is_idempotent() {
    let ctx = MigrationTestCtx::new();
    let block = AuthBlock::default();

    for _ in 0..2 {
        block
            .lifecycle(
                &ctx,
                LifecycleEvent {
                    event_type: LifecycleType::Init,
                    data: Vec::new(),
                },
            )
            .await
            .expect("Init lifecycle must be idempotent across re-boots");
    }
}
