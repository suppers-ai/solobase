//! User portal block migrations. Applied from the block's `Init` lifecycle.
//!
//! SQL files are embedded with `include_str!`. Backend dispatch + the
//! `current_hash` / `blessed_hash` / `SOLOBASE_RUN_MIGRATIONS` gate live
//! in [`crate::migration_helper::apply_migrations`].

use wafer_run::context::Context;

use crate::migration_helper;

const USERPORTAL_BLOCK_NAME: &str = "suppers-ai/userportal";

const SQL_001_SQLITE: &str = include_str!("001_userportal_schema.sqlite.sql");
const SQL_001_POSTGRES: &str = include_str!("001_userportal_schema.postgres.sql");

/// Ordered SQLite migration scripts for this block, as `(basename, content)`
/// pairs. Single source for both the runtime `apply()` below and the
/// Cloudflare-build D1 migration registry (`crate::migrations`).
pub(crate) const SQLITE_MIGRATIONS: &[(&str, &str)] = &[("001_userportal_schema", SQL_001_SQLITE)];

pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    let sqlite: Vec<&str> = SQLITE_MIGRATIONS.iter().map(|(_, sql)| *sql).collect();
    migration_helper::apply_migrations(ctx, USERPORTAL_BLOCK_NAME, &sqlite, &[SQL_001_POSTGRES])
        .await
}

#[cfg(test)]
mod tests {
    use super::{SQL_001_POSTGRES, SQL_001_SQLITE};

    #[test]
    fn sqlite_script_creates_buttons_table_and_sort_order_index() {
        assert!(
            SQL_001_SQLITE.contains("CREATE TABLE IF NOT EXISTS suppers_ai__userportal__buttons")
        );
        assert!(SQL_001_SQLITE.contains("suppers_ai__userportal__buttons_sort_order_idx"));
    }

    #[test]
    fn postgres_script_creates_buttons_table_and_sort_order_index() {
        assert!(
            SQL_001_POSTGRES.contains("CREATE TABLE IF NOT EXISTS suppers_ai__userportal__buttons")
        );
        assert!(SQL_001_POSTGRES.contains("suppers_ai__userportal__buttons_sort_order_idx"));
    }
}
