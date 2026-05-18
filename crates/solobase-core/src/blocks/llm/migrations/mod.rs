//! LLM block migrations. Applied from the block's `Init` lifecycle.
//!
//! Mirrors the files block's migration pattern (see `files/migrations/mod.rs`).
//! SQL files are embedded with `include_str!`. Backend selection reads the
//! `SOLOBASE_SHARED__DATABASE__BACKEND` config key (`sqlite` | `postgres`).
//! Falls back to `sqlite` when the config block is not registered.
//!
//! Application is gated by [`crate::migration_helper::apply_if_blessed`]:
//! the helper handles statement splitting + the `current_hash` /
//! `blessed_hash` / `SOLOBASE_RUN_MIGRATIONS` gate, and stamps a row in
//! `suppers_ai__admin__block_settings` once applied. Replaces the implicit
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

use wafer_core::clients::config;
use wafer_run::context::Context;

use crate::migration_helper;

const LLM_BLOCK_NAME: &str = "suppers-ai/llm";

const SQL_001_SQLITE: &str = include_str!("001_llm_schema.sqlite.sql");
const SQL_001_POSTGRES: &str = include_str!("001_llm_schema.postgres.sql");

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

fn sql_for(b: Backend) -> &'static str {
    match b {
        Backend::Sqlite => SQL_001_SQLITE,
        Backend::Postgres => SQL_001_POSTGRES,
    }
}

/// Apply LLM schema migrations through the shared migration-state gate.
/// Idempotent across cold starts: once the gate stamps a `current_hash`
/// row in `block_settings`, subsequent boots short-circuit before issuing
/// any DDL. Schema changes require a `--run-migrations` redeploy (see
/// `migration-state-workflow` in user memory).
pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    let b = backend(ctx).await;
    migration_helper::apply_if_blessed(ctx, LLM_BLOCK_NAME, sql_for(b)).await
}

#[cfg(test)]
mod tests {
    use super::{sql_for, Backend};

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
        let count = split_statements(sql_for(Backend::Sqlite));
        assert_eq!(count, 5, "sqlite llm migration: expected 5 statements");
    }

    #[test]
    fn postgres_sql_splits_into_expected_chunks() {
        let count = split_statements(sql_for(Backend::Postgres));
        assert_eq!(count, 5, "postgres llm migration: expected 5 statements");
    }

    #[test]
    fn sqlite_creates_both_tables() {
        let sql = sql_for(Backend::Sqlite);
        assert!(
            sql.contains("suppers_ai__llm__settings"),
            "missing settings table"
        );
        assert!(
            sql.contains("suppers_ai__llm__providers"),
            "missing providers table"
        );
    }

    #[test]
    fn sqlite_declares_required_indexes() {
        let sql = sql_for(Backend::Sqlite);
        assert!(sql.contains("suppers_ai__llm__settings_thread_id_idx"));
        assert!(sql.contains("suppers_ai__llm__providers_name_uniq"));
        assert!(sql.contains("suppers_ai__llm__providers_enabled_idx"));
    }

    #[test]
    fn postgres_creates_both_tables() {
        let sql = sql_for(Backend::Postgres);
        assert!(sql.contains("suppers_ai__llm__settings"));
        assert!(sql.contains("suppers_ai__llm__providers"));
    }
}
