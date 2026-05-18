//! Apply migration 001 against in-memory SQLite; verify all §3 tables exist.
//!
//! Uses a minimal `Context` test harness that dispatches
//! `call_block("wafer-run/database", ...)` to a real `DatabaseBlock` wrapping
//! an in-memory `SQLiteDatabaseService`. This exercises the same message
//! contract (`exec_raw` / `query_raw`) the block uses at runtime.
//!
//! `config::get_default(ctx, "SOLOBASE_SHARED__DATABASE__BACKEND", "sqlite")`
//! falls back to `"sqlite"` because we intentionally don't register
//! `wafer-run/config` — the fallback keeps the test self-contained.

use solobase_core::blocks::auth::migrations;
use wafer_core::clients::database as db;

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
async fn migration_001_creates_all_tables() {
    let ctx = MigrationTestCtx::new().await;
    migrations::apply(&ctx).await.expect("migration 001 apply");

    let rows = db::query_raw(
        &ctx,
        "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'suppers_ai__auth__%'",
        &[],
    )
    .await
    .expect("query sqlite_master");

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
            "table missing: {t} (got {names:?})"
        );
    }
}

#[tokio::test]
async fn migration_001_is_idempotent() {
    let ctx = MigrationTestCtx::new().await;
    migrations::apply(&ctx).await.expect("first apply");
    migrations::apply(&ctx)
        .await
        .expect("second apply must succeed (idempotent)");
}
