//! PR 1: confirm `migrations::apply` runs on `AuthBlock::lifecycle(Init)`.
//!
//! Builds an `AuthBlock` against a fresh `MigrationTestCtx` (which has NO
//! migrations pre-applied — only the database + crypto service blocks
//! routed) and dispatches a `LifecycleType::Init` event directly. The
//! lifecycle handler must apply the Plan A2 migrations before the
//! Plan A2 tables physically exist in `sqlite_master`. We probe
//! `sqlite_master` directly (not `db::list`) because the SQLite service's
//! `list` returns an empty `RecordList` for missing tables instead of
//! erroring — which would mask a missing migration.

use solobase_core::blocks::auth::AuthBlock;
use wafer_core::clients::database as db;
use wafer_run::{
    block::Block,
    types::{LifecycleEvent, LifecycleType},
};

use crate::common::MigrationTestCtx;

async fn run_init(ctx: &MigrationTestCtx) {
    let block = AuthBlock::default();
    let event = LifecycleEvent {
        event_type: LifecycleType::Init,
        data: Vec::new(),
    };
    block
        .lifecycle(ctx, event)
        .await
        .expect("lifecycle(Init) must succeed");
}

async fn auth_table_names(ctx: &MigrationTestCtx) -> Vec<String> {
    let rows = db::query_raw(
        ctx,
        "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'suppers_ai__auth__%'",
        &[],
    )
    .await
    .expect("query sqlite_master");
    rows.iter()
        .filter_map(|r| {
            r.data
                .get("name")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .collect()
}

/// `lifecycle(Init)` must create the `local_credentials` table — one of
/// the Plan A2 tables that doesn't exist in the legacy `users`-only
/// schema. Without `migrations::apply` wired into Init this assertion
/// fails because nothing else in the production lifecycle materialises
/// the table.
#[tokio::test]
async fn init_lifecycle_creates_local_credentials_table() {
    let ctx = MigrationTestCtx::new();

    let before = auth_table_names(&ctx).await;
    assert!(
        !before.contains(&"suppers_ai__auth__local_credentials".to_string()),
        "preconditions: local_credentials must NOT exist before Init: {before:?}",
    );

    run_init(&ctx).await;

    let after = auth_table_names(&ctx).await;
    assert!(
        after.contains(&"suppers_ai__auth__local_credentials".to_string()),
        "local_credentials missing after Init (got {after:?})",
    );
}

/// `lifecycle(Init)` must create the `bootstrap_tokens` table — the
/// other Plan A2 table without a legacy counterpart.
#[tokio::test]
async fn init_lifecycle_creates_bootstrap_tokens_table() {
    let ctx = MigrationTestCtx::new();
    run_init(&ctx).await;

    let after = auth_table_names(&ctx).await;
    assert!(
        after.contains(&"suppers_ai__auth__bootstrap_tokens".to_string()),
        "bootstrap_tokens missing after Init (got {after:?})",
    );
}
