//! Admin block migrations. Applied from the block's `Init` lifecycle.
//!
//! Mirrors the auth block's migration pattern (see `auth/migrations/mod.rs`).
//! SQL files are embedded with `include_str!`. Backend selection reads the
//! `SOLOBASE_SHARED__DATABASE__BACKEND` config key (`sqlite` | `postgres`).
//! Falls back to `sqlite` when the config block is not registered.
//!
//! Application is gated by [`crate::migration_helper::apply_if_blessed`]:
//! the helper handles statement splitting + the `current_hash` /
//! `blessed_hash` / `SOLOBASE_RUN_MIGRATIONS` gate, and stamps a row in
//! `suppers_ai__admin__block_settings` once applied. Earlier versions of
//! this module called `db::ddl` directly in a loop, bypassing the gate
//! and re-running every DDL on every cold isolate (~2,800 D1 queries/day
//! on wafer.run — see the 2026-05-14 config-snapshot spec).

use wafer_core::clients::config;
use wafer_run::context::Context;

const ADMIN_BLOCK_NAME: &str = "suppers-ai/admin";

const SQL_001_SQLITE: &str = include_str!("001_admin_schema.sqlite.sql");
const SQL_001_POSTGRES: &str = include_str!("001_admin_schema.postgres.sql");
const SQL_002_SQLITE: &str = include_str!("002_variables_block_column.sqlite.sql");
const SQL_002_POSTGRES: &str = include_str!("002_variables_block_column.postgres.sql");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Sqlite,
    Postgres,
}

async fn backend(ctx: &dyn Context) -> Backend {
    let raw = config::get_default(ctx, "SOLOBASE_SHARED__DATABASE__BACKEND", "sqlite").await;
    match raw.to_ascii_lowercase().as_str() {
        "postgres" => Backend::Postgres,
        _ => Backend::Sqlite,
    }
}

/// Concatenate the per-backend migration scripts into a single blob so
/// `apply_if_blessed` records one `current_hash` covering both. Mirrors the
/// auth block's concatenation pattern; `apply_if_blessed`'s splitter handles
/// the `;\n…` boundary between files.
fn concatenated_sql(b: Backend) -> String {
    match b {
        Backend::Sqlite => format!("{SQL_001_SQLITE}\n{SQL_002_SQLITE}"),
        Backend::Postgres => format!("{SQL_001_POSTGRES}\n{SQL_002_POSTGRES}"),
    }
}

/// Apply all admin migrations through the shared migration-state gate.
/// Idempotent across cold starts: once the gate stamps a `current_hash` row
/// in `block_settings`, subsequent boots short-circuit before issuing any
/// DDL. Schema changes require a `--run-migrations` redeploy (see
/// `migration-state-workflow` in user memory).
pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    let b = backend(ctx).await;
    let sql = concatenated_sql(b);
    crate::migration_helper::apply_if_blessed(ctx, ADMIN_BLOCK_NAME, &sql).await
}

#[cfg(test)]
mod tests {
    use super::{concatenated_sql, Backend};

    #[test]
    fn concatenated_sqlite_contains_both_migrations() {
        let sql = concatenated_sql(Backend::Sqlite);
        // Spot-check the 001 schema (the variables UNIQUE INDEX) and the
        // 002 follow-up (ALTER TABLE ADD COLUMN block) are both present.
        assert!(
            sql.contains("suppers_ai__admin__variables_key_uniq"),
            "missing 001 marker; got len={}",
            sql.len()
        );
        assert!(
            sql.contains("ADD COLUMN block"),
            "missing 002 ALTER COLUMN; got len={}",
            sql.len()
        );
        assert!(
            sql.contains("suppers_ai__admin__variables_block_idx"),
            "missing 002 index; got len={}",
            sql.len()
        );
    }

    #[test]
    fn concatenated_postgres_contains_both_migrations() {
        let sql = concatenated_sql(Backend::Postgres);
        assert!(
            sql.contains("suppers_ai__admin__variables_key_uniq"),
            "missing 001 marker; got len={}",
            sql.len()
        );
        assert!(
            sql.contains("ADD COLUMN"),
            "missing 002 ALTER COLUMN; got len={}",
            sql.len()
        );
    }
}
