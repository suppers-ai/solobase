//! Admin block migrations. Applied from the block's `Init` lifecycle.
//!
//! Mirrors the auth block's migration pattern (see `auth/migrations/mod.rs`).
//! SQL files are embedded with `include_str!`. Backend selection reads the
//! `SOLOBASE_SHARED__DATABASE__BACKEND` config key (`sqlite` | `postgres`).
//! Falls back to `sqlite` when the config block is not registered.
//!
//! Statements are executed one-by-one through `wafer-run/database`'s typed
//! `db::ddl` message contract — the WRAP-aware path that lets any
//! attributable caller run `CREATE TABLE` / `CREATE INDEX` against its own
//! (`{org}__{block}__*`) tables without an admin grant. The parser splits
//! on bare `;` outside of `--` line comments.

use wafer_core::clients::{config, database as db};
use wafer_run::context::Context;

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

/// Apply all admin migrations in order. Idempotent: every `CREATE TABLE` /
/// `CREATE INDEX` uses `IF NOT EXISTS`. Migration 002 is *not* fully
/// idempotent — `ALTER TABLE … ADD COLUMN` errors if the column already
/// exists (SQLite) — but the column-existence check happens before the
/// migration runner picks the script up. The runner guards against
/// re-application via the migration-state hash (see
/// `solobase-cloud-target` runbook + `SOLOBASE_RUN_MIGRATIONS` flag).
pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    let b = backend(ctx).await;
    let (sql_001, sql_002) = match b {
        Backend::Sqlite => (SQL_001_SQLITE, SQL_002_SQLITE),
        Backend::Postgres => (SQL_001_POSTGRES, SQL_002_POSTGRES),
    };
    apply_script(ctx, sql_001)
        .await
        .map_err(|e| format!("migration 001: {e}"))?;
    apply_script(ctx, sql_002)
        .await
        .map_err(|e| format!("migration 002: {e}"))?;
    Ok(())
}

async fn apply_script(ctx: &dyn Context, sql: &str) -> Result<(), String> {
    for stmt in split_statements(sql) {
        if !has_executable_content(&stmt) {
            continue;
        }
        let trimmed = stmt.trim();
        db::ddl(ctx, trimmed)
            .await
            .map_err(|e| format!("ddl failed on `{trimmed}`: {e}"))?;
    }
    Ok(())
}

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
    use super::{has_executable_content, split_statements, SQL_001_SQLITE, SQL_002_SQLITE};

    #[test]
    fn embedded_sqlite_script_parses_into_statements() {
        let parts: Vec<_> = split_statements(SQL_001_SQLITE)
            .into_iter()
            .filter(|s| has_executable_content(s))
            .collect();
        // 9 tables + their CREATE INDEX statements (some tables have 1 index,
        // user_roles/audit_logs/request_logs/storage_access_logs/wrap_grants).
        assert!(
            parts.len() >= 10,
            "expected at least 10 statements, got {}: {:?}",
            parts.len(),
            parts
        );
        // Spot-check that the variables UNIQUE INDEX is present.
        assert!(parts
            .iter()
            .any(|s| s.contains("suppers_ai__admin__variables_key_uniq")));
    }

    #[test]
    fn embedded_sqlite_002_parses_into_three_statements() {
        // ALTER TABLE + UPDATE … SET block = CASE … END + CREATE INDEX.
        let parts: Vec<_> = split_statements(SQL_002_SQLITE)
            .into_iter()
            .filter(|s| has_executable_content(s))
            .collect();
        assert_eq!(
            parts.len(),
            3,
            "expected 3 statements (ALTER + UPDATE + CREATE INDEX), got {}: {:?}",
            parts.len(),
            parts
        );
        assert!(parts.iter().any(|s| s.contains("ADD COLUMN block")));
        assert!(parts
            .iter()
            .any(|s| s.contains("suppers_ai__admin__variables_block_idx")));
    }

    #[test]
    fn split_handles_line_comments_and_semicolons() {
        let sql = "-- header\nCREATE TABLE foo (id TEXT);\n-- ; in comment\nCREATE INDEX bar ON foo (id);";
        let parts: Vec<_> = split_statements(sql)
            .into_iter()
            .filter(|s| has_executable_content(s))
            .collect();
        assert_eq!(parts.len(), 2);
    }
}
