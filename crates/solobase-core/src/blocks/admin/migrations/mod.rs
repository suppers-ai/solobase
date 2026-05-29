//! Admin block migrations. Applied from the block's `Init` lifecycle.
//!
//! SQL files are embedded with `include_str!`. Backend dispatch + concat +
//! the `current_hash` / `blessed_hash` / `SOLOBASE_RUN_MIGRATIONS` gate
//! all live in [`crate::migration_helper::apply_migrations`]. Earlier
//! versions of this module called `db::ddl` directly in a loop, bypassing
//! the gate and re-running every DDL on every cold isolate (~2,800 D1
//! queries/day on wafer.run — see the 2026-05-14 config-snapshot spec).

use wafer_run::context::Context;

const ADMIN_BLOCK_NAME: &str = "suppers-ai/admin";

const SQL_001_SQLITE: &str = include_str!("001_admin_schema.sqlite.sql");
const SQL_001_POSTGRES: &str = include_str!("001_admin_schema.postgres.sql");
const SQL_002_SQLITE: &str = include_str!("002_variables_block_column.sqlite.sql");
const SQL_002_POSTGRES: &str = include_str!("002_variables_block_column.postgres.sql");
const SQL_003_SQLITE: &str = include_str!("003_block_settings_seed_hash.sqlite.sql");
const SQL_003_POSTGRES: &str = include_str!("003_block_settings_seed_hash.postgres.sql");

/// Apply all admin migrations through the shared migration-state gate.
/// Idempotent across cold starts: once the gate stamps a `current_hash` row
/// in `block_settings`, subsequent boots short-circuit before issuing any
/// DDL. Schema changes require a `--run-migrations` redeploy (see
/// `migration-state-workflow` in user memory).
pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    crate::migration_helper::apply_migrations(
        ctx,
        ADMIN_BLOCK_NAME,
        &[SQL_001_SQLITE, SQL_002_SQLITE, SQL_003_SQLITE],
        &[SQL_001_POSTGRES, SQL_002_POSTGRES, SQL_003_POSTGRES],
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::{
        SQL_001_POSTGRES, SQL_001_SQLITE, SQL_002_POSTGRES, SQL_002_SQLITE, SQL_003_POSTGRES,
        SQL_003_SQLITE,
    };

    #[test]
    fn sqlite_migrations_contain_expected_ddl() {
        // 001 schema (variables UNIQUE INDEX)
        assert!(SQL_001_SQLITE.contains("suppers_ai__admin__variables_key_uniq"));
        // 002 follow-up (ALTER TABLE ADD COLUMN block + index)
        assert!(SQL_002_SQLITE.contains("ADD COLUMN block"));
        assert!(SQL_002_SQLITE.contains("suppers_ai__admin__variables_block_idx"));
        // 003 follow-up (ADD COLUMN seed_defaults_hash)
        assert!(SQL_003_SQLITE.contains("ADD COLUMN seed_defaults_hash"));
    }

    #[test]
    fn postgres_migrations_contain_expected_ddl() {
        assert!(SQL_001_POSTGRES.contains("suppers_ai__admin__variables_key_uniq"));
        assert!(SQL_002_POSTGRES.contains("ADD COLUMN"));
        assert!(SQL_003_POSTGRES.contains("seed_defaults_hash"));
    }
}
