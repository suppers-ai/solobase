//! Auth block migrations. Delegated to `crate::migration_helper`.
//!
//! Hash-gated apply — runs at most once per deploy (per block, per isolate).
//! Concatenated SQL of both migration scripts is hashed and tracked in
//! `suppers_ai__admin__block_settings`.

use std::sync::atomic::AtomicBool;

use wafer_core::clients::config;
use wafer_run::context::Context;

use crate::migration_helper;

const SQL_001_SQLITE: &str = include_str!("001_auth_schema.sqlite.sql");
const SQL_001_POSTGRES: &str = include_str!("001_auth_schema.postgres.sql");
const SQL_002_SQLITE: &str = include_str!("002_reserved_orgs.sqlite.sql");
const SQL_002_POSTGRES: &str = include_str!("002_reserved_orgs.postgres.sql");

static APPLIED: AtomicBool = AtomicBool::new(false);

pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    let backend = config::get_default(ctx, "SOLOBASE_SHARED__DATABASE__BACKEND", "sqlite")
        .await
        .to_ascii_lowercase();
    let sql = if backend == "postgres" {
        format!("{SQL_001_POSTGRES}\n{SQL_002_POSTGRES}")
    } else {
        format!("{SQL_001_SQLITE}\n{SQL_002_SQLITE}")
    };
    migration_helper::apply_if_blessed(ctx, "suppers-ai/auth", &sql, &APPLIED).await
}

/// Apply migrations directly against the provided context, bypassing the
/// hash-gate and `APPLIED` guard. Used by test fixtures that create a fresh
/// in-memory SQLite DB per test — the per-process `APPLIED` guard cannot be
/// shared across concurrent tests that each start with an empty schema.
#[cfg(test)]
pub(crate) async fn apply_direct(ctx: &dyn Context) -> Result<(), String> {
    use wafer_core::clients::database as db;

    let sql = format!("{SQL_001_SQLITE}\n{SQL_002_SQLITE}");
    for stmt in migration_helper::split_statements_for_test(&sql) {
        if !migration_helper::has_executable_content_for_test(&stmt) {
            continue;
        }
        let trimmed = stmt.trim();
        db::ddl(ctx, trimmed)
            .await
            .map_err(|e| format!("ddl failed on `{trimmed}`: {e}"))?;
    }
    Ok(())
}
