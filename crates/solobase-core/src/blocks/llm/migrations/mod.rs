//! LLM block migrations. Applied from the block's `Init` lifecycle.
//!
//! SQL files are embedded with `include_str!`. Backend dispatch + the
//! `current_hash` / `blessed_hash` / `SOLOBASE_RUN_MIGRATIONS` gate live
//! in [`crate::migration_helper::apply_migrations`]. Replaces the implicit
//! `ensure_table` materialisation that previously created these tables on
//! first insert (TEXT-only columns, no indexes — see solobase
//! `ensure-table-removal-in-progress`).
//!
//! The sibling [`legacy_providers`] module hosts the one-shot row-copy
//! migration from the retired `suppers_ai__provider_llm__providers` table
//! into the new `suppers_ai__llm__providers` table. It is invoked
//! separately from `LlmBlock::lifecycle(Init)` because it needs a handle
//! to the in-memory `ProviderLlmService` (to refresh it post-copy).

pub(in crate::blocks::llm) mod legacy_providers;

use wafer_run::context::Context;

use crate::migration_helper;

const LLM_BLOCK_NAME: &str = "suppers-ai/llm";

const SQL_001_SQLITE: &str = include_str!("001_llm_schema.sqlite.sql");
const SQL_001_POSTGRES: &str = include_str!("001_llm_schema.postgres.sql");

/// Apply LLM schema migrations through the shared migration-state gate.
/// Idempotent across cold starts: once the gate stamps a `current_hash`
/// row in `block_settings`, subsequent boots short-circuit before issuing
/// any DDL. Schema changes require a `--run-migrations` redeploy (see
/// `migration-state-workflow` in user memory).
pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    migration_helper::apply_migrations(ctx, LLM_BLOCK_NAME, &[SQL_001_SQLITE], &[SQL_001_POSTGRES])
        .await
}

#[cfg(test)]
mod tests {
    use super::{SQL_001_POSTGRES, SQL_001_SQLITE};

    fn split_statements(sql: &str) -> usize {
        // Inline mirror of `migration_helper::split_statements`'s
        // semicolon-on-newline split, filtering empty/comment-only chunks.
        // Kept here so the parser test guards regressions against the
        // committed SQL without depending on the helper's pub surface.
        sql.split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter(|s| {
                s.lines().any(|line| {
                    let l = line.trim();
                    !l.is_empty() && !l.starts_with("--")
                })
            })
            .count()
    }

    #[test]
    fn sqlite_sql_splits_into_expected_chunks() {
        // 2 tables + 3 indexes = 5 executable statements.
        assert_eq!(
            split_statements(SQL_001_SQLITE),
            5,
            "sqlite llm migration: expected 5 statements"
        );
    }

    #[test]
    fn postgres_sql_splits_into_expected_chunks() {
        assert_eq!(
            split_statements(SQL_001_POSTGRES),
            5,
            "postgres llm migration: expected 5 statements"
        );
    }

    #[test]
    fn sqlite_creates_both_tables() {
        assert!(SQL_001_SQLITE.contains("suppers_ai__llm__settings"));
        assert!(SQL_001_SQLITE.contains("suppers_ai__llm__providers"));
    }

    #[test]
    fn sqlite_declares_required_indexes() {
        assert!(SQL_001_SQLITE.contains("suppers_ai__llm__settings_thread_id_idx"));
        assert!(SQL_001_SQLITE.contains("suppers_ai__llm__providers_name_uniq"));
        assert!(SQL_001_SQLITE.contains("suppers_ai__llm__providers_enabled_idx"));
    }

    #[test]
    fn postgres_creates_both_tables() {
        assert!(SQL_001_POSTGRES.contains("suppers_ai__llm__settings"));
        assert!(SQL_001_POSTGRES.contains("suppers_ai__llm__providers"));
    }
}
