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

use std::sync::Arc;

use solobase_core::blocks::auth::migrations;
use wafer_core::clients::database as db;
use wafer_run::{
    block::Block,
    context::Context,
    types::{Message, WaferError},
    InputStream, OutputStream,
};

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

/// Minimal `Context` that routes `call_block("wafer-run/database", ...)` to a
/// real `DatabaseBlock` wrapping an in-memory SQLite service. Any other block
/// call returns `NotFound` — including `wafer-run/config`, which makes
/// `config::get_default(..., "sqlite")` fall back to the default.
struct MigrationTestCtx {
    db_block: Arc<dyn Block>,
}

impl MigrationTestCtx {
    fn new() -> Self {
        let svc = Arc::new(
            wafer_block_sqlite::service::SQLiteDatabaseService::open_in_memory()
                .expect("open in-memory sqlite"),
        );
        let db_block: Arc<dyn Block> = Arc::new(
            wafer_core::service_blocks::database::DatabaseBlock::new(svc),
        );
        Self { db_block }
    }
}

#[async_trait::async_trait]
impl Context for MigrationTestCtx {
    async fn call_block(&self, block_name: &str, msg: Message, input: InputStream) -> OutputStream {
        if block_name == "wafer-run/database" {
            self.db_block.handle(self, msg, input).await
        } else {
            OutputStream::error(WaferError::new(
                wafer_run::types::ErrorCode::NOT_FOUND,
                format!("block '{block_name}' not registered in test ctx"),
            ))
        }
    }

    fn is_cancelled(&self) -> bool {
        false
    }

    fn config_get(&self, _key: &str) -> Option<&str> {
        None
    }
}

#[tokio::test]
async fn migration_001_creates_all_tables() {
    let ctx = MigrationTestCtx::new();
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
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("first apply");
    migrations::apply(&ctx)
        .await
        .expect("second apply must succeed (idempotent)");
}
