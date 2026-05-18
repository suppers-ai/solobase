//! Shared migration helper.
//!
//! Each block's `migrations::apply()` calls [`apply_if_blessed`], which:
//!
//! 1. Reads the block's `MigrationState` from the cached `BlockSettings`.
//! 2. Computes the SQL's SHA-256.
//! 3. If `current_hash` matches the code's hash → already applied, return.
//! 4. If `blessed_hash` matches OR the `SOLOBASE_RUN_MIGRATIONS` env var is
//!    set to `"1"` → apply all statements via `db::ddl`, then upsert the
//!    block's row in `suppers_ai__admin__block_settings` with the new hash.
//! 5. Otherwise → log warning and return (operator must redeploy with
//!    `--run-migrations` to bless this schema).
//!
//! The statement splitter handles `;` outside `--` comments. Block comments
//! `/* ... */` and `;` inside string literals are not supported — the
//! canonical .sql files don't use either.

use sha2::{Digest, Sha256};
use wafer_core::clients::database as db;
use wafer_run::context::Context;

use crate::features::{BlockSettings, MigrationState, BLOCK_SETTINGS_CONFIG_KEY};
// NOTE: `BlockSettings::state_for` parses only the requested block's entry
// out of the JSON map, avoiding the full-map materialization that
// `from_config_json` would do on every `apply_if_blessed` call.

/// Env-var name set by `solobase --run-migrations` (native) or
/// `deploy-cloudflare.sh deploy --run-migrations` (CF).
pub const RUN_MIGRATIONS_KEY: &str = "SOLOBASE_RUN_MIGRATIONS";

/// Full table name for the block_settings collection. Hardcoded here to
/// avoid a circular dep on `crate::blocks::admin`.
const BLOCK_SETTINGS_TABLE: &str = "suppers_ai__admin__block_settings";

/// Apply `sql` against `db::ddl` iff the operator has blessed it or
/// `SOLOBASE_RUN_MIGRATIONS=1`. Idempotent across calls: returns early
/// once `current_hash` in the cached `BlockSettings` matches the SQL's hash.
///
/// `block_name` is the full block name (e.g. `"suppers-ai/files"`).
/// `sql` is the embedded migration SQL (usually `include_str!(...)`).
pub async fn apply_if_blessed(
    ctx: &dyn Context,
    block_name: &str,
    sql: &str,
) -> Result<(), String> {
    let code_hash = sha256_hex(sql);
    let state = read_state(ctx, block_name);
    // Read directly from the config snapshot (env-var sourced key). The
    // config block service is not involved — `SOLOBASE_RUN_MIGRATIONS` is
    // an infra env var that populates the wafer config snapshot at boot,
    // never written to the DB. Tests populate it via `ctx.set_config(...)`.
    let run_requested = ctx.config_get(RUN_MIGRATIONS_KEY) == Some("1");

    if state.current_hash == code_hash {
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
        return Ok(());
    }

    for stmt in split_statements(sql) {
        if !has_executable_content(stmt) {
            continue;
        }
        let trimmed = stmt.trim();
        db::ddl(ctx, trimmed)
            .await
            .inspect_err(|e| {
                tracing::warn!(
                    block = %block_name,
                    stmt = %trimmed,
                    err = %e,
                    "ddl failed",
                )
            })
            .map_err(|e| format!("ddl failed on `{trimmed}`: {e}"))?;
    }

    let new_state = MigrationState {
        current_hash: code_hash.clone(),
        blessed_hash: code_hash,
    };
    write_state(ctx, block_name, &new_state).await?;

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
    BlockSettings::state_for(json, block_name).migration
}

/// Upsert the block's row in `suppers_ai__admin__block_settings` with the
/// new migration state. Preserves the `enabled` flag if the row already exists.
async fn write_state(
    ctx: &dyn Context,
    block_name: &str,
    state: &MigrationState,
) -> Result<(), String> {
    let mut patch = std::collections::HashMap::new();
    patch.insert(
        "current_hash".to_string(),
        serde_json::json!(state.current_hash),
    );
    patch.insert(
        "blessed_hash".to_string(),
        serde_json::json!(state.blessed_hash),
    );
    upsert_block_settings_fields(ctx, block_name, patch).await
}

/// Upsert a subset of columns on the `suppers_ai__admin__block_settings` row
/// keyed by `block_name`. Creates the row with `enabled=true` if absent,
/// preserves every column not present in `patch` otherwise.
///
/// Shared by `migration_helper::write_state` (migration hash columns) and
/// `admin::settings::seed_defaults` (seed_defaults_hash column) so both
/// hash-gates write through the same single-row-per-block primitive.
pub(crate) async fn upsert_block_settings_fields(
    ctx: &dyn Context,
    block_name: &str,
    patch: std::collections::HashMap<String, serde_json::Value>,
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

    let existing = db::list(ctx, BLOCK_SETTINGS_TABLE, &opts)
        .await
        .map_err(|e| format!("block_settings lookup: {e}"))?;

    if !existing.records.is_empty() {
        let id = existing.records[0].id.clone();
        db::update(ctx, BLOCK_SETTINGS_TABLE, &id, patch)
            .await
            .map_err(|e| format!("block_settings update: {e}"))?;
    } else {
        let mut data = patch;
        data.insert("block_name".to_string(), serde_json::json!(block_name));
        data.entry("enabled".to_string())
            .or_insert(serde_json::json!(true));
        db::create(ctx, BLOCK_SETTINGS_TABLE, data)
            .await
            .map_err(|e| format!("block_settings create: {e}"))?;
    }
    Ok(())
}

/// Compute a SHA-256 hex digest. Re-exported for callers (e.g.
/// `admin::settings::seed_defaults`) that hash-gate against a payload other
/// than SQL bytes but want to share the same digest algorithm with
/// `apply_if_blessed`.
pub(crate) fn sha256_hex_bytes(payload: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    hex::encode(hasher.finalize())
}

fn sha256_hex(sql: &str) -> String {
    sha256_hex_bytes(sql.as_bytes())
}

/// Split `sql` on `;` outside `--` line comments. Returns byte-range slices
/// into the original `sql` — no per-statement allocation.
fn split_statements(sql: &str) -> Vec<&str> {
    let bytes = sql.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut in_line_comment = false;
    let mut prev_was_dash = false;
    for (i, &b) in bytes.iter().enumerate() {
        if in_line_comment {
            if b == b'\n' {
                in_line_comment = false;
            }
            prev_was_dash = false;
            continue;
        }
        if b == b'-' && prev_was_dash {
            in_line_comment = true;
            prev_was_dash = false;
            continue;
        }
        if b == b';' {
            out.push(&sql[start..i]);
            start = i + 1;
            prev_was_dash = false;
            continue;
        }
        prev_was_dash = b == b'-';
    }
    if start < bytes.len() {
        out.push(&sql[start..]);
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
    fn legalpages_sql_splits_into_expected_chunks() {
        let sql_sqlite =
            include_str!("blocks/legalpages/migrations/001_legalpages_schema.sqlite.sql");
        let sqlite_count = split_statements(sql_sqlite)
            .into_iter()
            .filter(|s| has_executable_content(s))
            .count();
        assert_eq!(
            sqlite_count, 2,
            "legalpages sqlite migration: expected 2 statements, got {sqlite_count}"
        );

        let sql_postgres =
            include_str!("blocks/legalpages/migrations/001_legalpages_schema.postgres.sql");
        let postgres_count = split_statements(sql_postgres)
            .into_iter()
            .filter(|s| has_executable_content(s))
            .count();
        assert_eq!(
            postgres_count, 2,
            "legalpages postgres migration: expected 2 statements, got {postgres_count}"
        );
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

    #[test]
    fn products_sql_splits_into_expected_chunks() {
        // Counts the executable statements in the products block SQL files.
        // 10 CREATE TABLE + 9 CREATE INDEX = 19 statements per backend.
        let sql_sqlite = include_str!("blocks/products/migrations/001_products_schema.sqlite.sql");
        let sqlite_count = split_statements(sql_sqlite)
            .into_iter()
            .filter(|s| has_executable_content(s))
            .count();
        assert_eq!(
            sqlite_count, 19,
            "products sqlite migration: expected 19 statements, got {sqlite_count}"
        );

        let sql_postgres =
            include_str!("blocks/products/migrations/001_products_schema.postgres.sql");
        let postgres_count = split_statements(sql_postgres)
            .into_iter()
            .filter(|s| has_executable_content(s))
            .count();
        assert_eq!(
            postgres_count, 19,
            "products postgres migration: expected 19 statements, got {postgres_count}"
        );
    }
}
