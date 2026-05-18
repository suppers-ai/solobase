//! User portal block migrations. Applied from the block's `Init` lifecycle.
//!
//! Mirrors the admin/auth/files migration pattern. SQL files are embedded
//! with `include_str!`. Backend selection reads the
//! `SOLOBASE_SHARED__DATABASE__BACKEND` config key (`sqlite` | `postgres`).
//! Falls back to `sqlite` when unset.
//!
//! Application is gated by [`crate::migration_helper::apply_if_blessed`]:
//! the helper handles statement splitting + the `current_hash` /
//! `blessed_hash` / `SOLOBASE_RUN_MIGRATIONS` gate, and stamps a row in
//! `suppers_ai__admin__block_settings` once applied.

use wafer_core::clients::config;
use wafer_run::context::Context;

use crate::migration_helper;

const USERPORTAL_BLOCK_NAME: &str = "suppers-ai/userportal";

const SQL_001_SQLITE: &str = include_str!("001_userportal_schema.sqlite.sql");
const SQL_001_POSTGRES: &str = include_str!("001_userportal_schema.postgres.sql");

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

fn concatenated_sql(b: Backend) -> String {
    match b {
        Backend::Sqlite => SQL_001_SQLITE.to_string(),
        Backend::Postgres => SQL_001_POSTGRES.to_string(),
    }
}

pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    let b = backend(ctx).await;
    let sql = concatenated_sql(b);
    migration_helper::apply_if_blessed(ctx, USERPORTAL_BLOCK_NAME, &sql).await
}

#[cfg(test)]
mod tests {
    use super::{concatenated_sql, Backend};

    #[test]
    fn sqlite_script_creates_buttons_table_and_sort_order_index() {
        let sql = concatenated_sql(Backend::Sqlite);
        assert!(
            sql.contains("CREATE TABLE IF NOT EXISTS suppers_ai__userportal__buttons"),
            "missing buttons CREATE TABLE; got len={}",
            sql.len()
        );
        assert!(
            sql.contains("suppers_ai__userportal__buttons_sort_order_idx"),
            "missing sort_order index; got len={}",
            sql.len()
        );
    }

    #[test]
    fn postgres_script_creates_buttons_table_and_sort_order_index() {
        let sql = concatenated_sql(Backend::Postgres);
        assert!(
            sql.contains("CREATE TABLE IF NOT EXISTS suppers_ai__userportal__buttons"),
            "missing buttons CREATE TABLE; got len={}",
            sql.len()
        );
        assert!(
            sql.contains("suppers_ai__userportal__buttons_sort_order_idx"),
            "missing sort_order index; got len={}",
            sql.len()
        );
    }
}
