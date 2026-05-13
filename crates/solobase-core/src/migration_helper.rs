//! Shared migration helper.
//!
//! Each block's `migrations::apply()` calls [`apply_if_blessed`], which:
//!
//! 1. Fast-path: checks a per-isolate `AtomicBool` — if true, returns.
//! 2. Reads the block's `MigrationState` from the cached `BlockSettings`.
//! 3. Computes the SQL's SHA-256.
//! 4. If `current_hash` matches the code's hash → already applied, return.
//! 5. If `blessed_hash` matches OR the `SOLOBASE_RUN_MIGRATIONS` env var is
//!    set to `"1"` → apply all statements via `db::ddl`, then upsert the
//!    block's row in `suppers_ai__admin__block_settings` with the new hash.
//! 6. Otherwise → log warning and return (operator must redeploy with
//!    `--run-migrations` to bless this schema).
//!
//! The statement splitter handles `;` outside `--` comments. Block comments
//! `/* ... */` and `;` inside string literals are not supported — the
//! canonical .sql files don't use either.

use std::sync::atomic::{AtomicBool, Ordering};

use sha2::{Digest, Sha256};
use wafer_core::clients::database as db;
use wafer_run::context::Context;

use crate::features::{BlockSettings, MigrationState, BLOCK_SETTINGS_CONFIG_KEY};

/// Env-var name set by `solobase --run-migrations` (native) or
/// `deploy-cloudflare.sh deploy --run-migrations` (CF).
pub const RUN_MIGRATIONS_KEY: &str = "SOLOBASE_RUN_MIGRATIONS";

/// Full table name for the block_settings collection. Hardcoded here to
/// avoid a circular dep on `crate::blocks::admin`.
const BLOCK_SETTINGS_TABLE: &str = "suppers_ai__admin__block_settings";

/// Apply `sql` against `db::ddl` iff the operator has blessed it or
/// `SOLOBASE_RUN_MIGRATIONS=1`. Idempotent within an isolate (the
/// `applied` flag short-circuits subsequent calls).
///
/// `block_name` is the full block name (e.g. `"suppers-ai/files"`).
/// `sql` is the embedded migration SQL (usually `include_str!(...)`).
/// `applied` is a `&'static AtomicBool` owned by the caller block.
pub async fn apply_if_blessed(
    ctx: &dyn Context,
    block_name: &str,
    sql: &str,
    applied: &AtomicBool,
) -> Result<(), String> {
    if applied.load(Ordering::Relaxed) {
        return Ok(());
    }

    let code_hash = sha256_hex(sql);
    let state = read_state(ctx, block_name);
    // Read directly from the config snapshot (env-var sourced key). The
    // config block service is not involved — `SOLOBASE_RUN_MIGRATIONS` is
    // an infra env var that populates the wafer config snapshot at boot,
    // never written to the DB. Tests populate it via `ctx.set_config(...)`.
    let run_requested = ctx.config_get(RUN_MIGRATIONS_KEY) == Some("1");

    if state.current_hash == code_hash {
        applied.store(true, Ordering::Relaxed);
        return Ok(());
    }

    // Fresh install (no previous apply) bootstraps without operator consent —
    // there's no prior schema to protect, and dev/test/browser-WASM modes
    // can't pass `--run-migrations`. Operator gating still applies to
    // SCHEMA CHANGES (current_hash non-empty + different code_hash below).
    let is_fresh = state.current_hash.is_empty();
    let should_apply = is_fresh || run_requested || state.blessed_hash == code_hash;
    if !should_apply {
        tracing::warn!(
            block = %block_name,
            current = %state.current_hash,
            blessed = %state.blessed_hash,
            code = %code_hash,
            "schema drift; redeploy with --run-migrations to apply"
        );
        applied.store(true, Ordering::Relaxed);
        return Ok(());
    }

    for stmt in split_statements(sql) {
        if !has_executable_content(&stmt) {
            continue;
        }
        let trimmed = stmt.trim();
        db::ddl(ctx, trimmed)
            .await
            .map_err(|e| format!("ddl failed on `{trimmed}`: {e}"))?;
    }

    let new_state = MigrationState {
        current_hash: code_hash.clone(),
        blessed_hash: code_hash,
    };
    write_state(ctx, block_name, &new_state).await?;

    applied.store(true, Ordering::Relaxed);
    Ok(())
}

/// Read the cached `BlockSettings` from the wafer config and look up the
/// migration state for `block_name`. Returns an empty `MigrationState` when
/// no row exists yet.
///
/// Reads directly from the config snapshot — `BLOCK_SETTINGS_CONFIG_KEY` is
/// a synthetic key set at boot by the loader (not a DB-backed config var),
/// so it lives in `ctx.config_get`, not behind the config block service.
fn read_state(ctx: &dyn Context, block_name: &str) -> MigrationState {
    let json = ctx.config_get(BLOCK_SETTINGS_CONFIG_KEY).unwrap_or("{}");
    let settings = BlockSettings::from_config_json(json);
    settings.state(block_name).migration
}

/// Upsert the block's row in `suppers_ai__admin__block_settings` with the
/// new migration state. Preserves the `enabled` flag if the row already exists.
async fn write_state(
    ctx: &dyn Context,
    block_name: &str,
    state: &MigrationState,
) -> Result<(), String> {
    use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};

    let opts = ListOptions {
        filters: vec![Filter {
            field: "block_name".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(block_name.to_string()),
        }],
        sort: vec![SortField {
            field: "created_at".into(),
            desc: false,
        }],
        limit: 1,
        offset: 0,
        skip_count: true,
    };

    match db::list(ctx, BLOCK_SETTINGS_TABLE, &opts).await {
        Ok(result) if !result.records.is_empty() => {
            let id = result.records[0].id.clone();
            let mut patch = std::collections::HashMap::new();
            patch.insert(
                "current_hash".to_string(),
                serde_json::json!(state.current_hash),
            );
            patch.insert(
                "blessed_hash".to_string(),
                serde_json::json!(state.blessed_hash),
            );
            db::update(ctx, BLOCK_SETTINGS_TABLE, &id, patch)
                .await
                .map_err(|e| format!("write_state update: {e}"))?;
        }
        Ok(_) => {
            let mut data = std::collections::HashMap::new();
            data.insert("block_name".to_string(), serde_json::json!(block_name));
            data.insert("enabled".to_string(), serde_json::json!(true));
            data.insert(
                "current_hash".to_string(),
                serde_json::json!(state.current_hash),
            );
            data.insert(
                "blessed_hash".to_string(),
                serde_json::json!(state.blessed_hash),
            );
            db::create(ctx, BLOCK_SETTINGS_TABLE, data)
                .await
                .map_err(|e| format!("write_state create: {e}"))?;
        }
        Err(e) => return Err(format!("write_state lookup: {e}")),
    }
    Ok(())
}

/// Split `sql` into statements for direct use by test fixtures.
/// Identical to the private `split_statements` — exposed for `apply_direct`
/// helpers in block migration modules.
#[cfg(test)]
pub(crate) fn split_statements_for_test(sql: &str) -> Vec<String> {
    split_statements(sql)
}

/// Returns true if `stmt` has executable content — exposed for `apply_direct`
/// helpers in block migration modules.
#[cfg(test)]
pub(crate) fn has_executable_content_for_test(stmt: &str) -> bool {
    has_executable_content(stmt)
}

fn sha256_hex(sql: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(sql.as_bytes());
    hex::encode(hasher.finalize())
}

/// Split `sql` on `;` outside `--` line comments.
fn split_statements(sql: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_line_comment = false;
    for ch in sql.chars() {
        if in_line_comment {
            current.push(ch);
            if ch == '\n' {
                in_line_comment = false;
            }
            continue;
        }
        if ch == '-' && current.ends_with('-') {
            in_line_comment = true;
            current.push(ch);
            continue;
        }
        if ch == ';' {
            out.push(std::mem::take(&mut current));
            continue;
        }
        current.push(ch);
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn has_executable_content(stmt: &str) -> bool {
    stmt.lines().any(|line| {
        let l = line.trim();
        !l.is_empty() && !l.starts_with("--")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_deterministic() {
        let a = sha256_hex("CREATE TABLE foo (id TEXT);");
        let b = sha256_hex("CREATE TABLE foo (id TEXT);");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn sha256_hex_differs_for_different_input() {
        let a = sha256_hex("CREATE TABLE foo (id TEXT);");
        let b = sha256_hex("CREATE TABLE bar (id TEXT);");
        assert_ne!(a, b);
    }

    #[test]
    fn empty_chunk_has_no_executable_content() {
        assert!(!has_executable_content(""));
        assert!(!has_executable_content("   \n  "));
    }

    #[test]
    fn comment_only_chunk_is_skipped() {
        assert!(!has_executable_content("-- one\n-- two\n"));
    }

    #[test]
    fn ddl_with_leading_comment_is_executed() {
        assert!(has_executable_content(
            "-- header\nCREATE TABLE foo (id TEXT)"
        ));
    }

    #[test]
    fn split_ignores_semicolons_inside_line_comments() {
        let sql = "-- Placeholder; text\nSELECT 1;";
        let parts = split_statements(sql);
        assert_eq!(parts.len(), 1);
        assert!(parts[0].contains("SELECT 1"));
    }

    #[test]
    fn split_handles_multiple_statements() {
        let sql = "DROP TABLE foo;\nCREATE TABLE bar (id TEXT);\n";
        let count = split_statements(sql)
            .into_iter()
            .filter(|s| has_executable_content(s))
            .count();
        assert_eq!(count, 2);
    }

    #[test]
    fn files_sql_splits_into_expected_chunks() {
        // Counts the executable statements in the files block SQL files.
        // Fails if the SQL file is edited and the helper's splitter is broken
        // or a new statement is added without updating this count.
        let sql_sqlite = include_str!("blocks/files/migrations/001_initial_schema.sqlite.sql");
        let sqlite_count = split_statements(sql_sqlite)
            .into_iter()
            .filter(|s| has_executable_content(s))
            .count();
        assert_eq!(
            sqlite_count, 14,
            "files sqlite migration: expected 14 statements, got {sqlite_count}"
        );

        let sql_postgres = include_str!("blocks/files/migrations/001_initial_schema.postgres.sql");
        let postgres_count = split_statements(sql_postgres)
            .into_iter()
            .filter(|s| has_executable_content(s))
            .count();
        assert_eq!(
            postgres_count, 14,
            "files postgres migration: expected 14 statements, got {postgres_count}"
        );
    }
}
