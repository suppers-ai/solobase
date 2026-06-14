//! Messages block migrations. Backend dispatch + apply gating live in
//! [`crate::migration_helper::apply_migrations`].

use wafer_run::context::Context;

use crate::migration_helper;

const SQL_001_SQLITE: &str = include_str!("001_messages_schema.sqlite.sql");
const SQL_001_POSTGRES: &str = include_str!("001_messages_schema.postgres.sql");

/// Ordered SQLite migration scripts for this block, as `(basename, content)`
/// pairs. Single source for both the runtime `apply()` below and the
/// Cloudflare-build D1 migration registry (`crate::migrations`).
pub(crate) const SQLITE_MIGRATIONS: &[(&str, &str)] = &[("001_messages_schema", SQL_001_SQLITE)];

pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    let sqlite: Vec<&str> = SQLITE_MIGRATIONS.iter().map(|(_, sql)| *sql).collect();
    migration_helper::apply_migrations(ctx, "suppers-ai/messages", &sqlite, &[SQL_001_POSTGRES])
        .await
}

#[cfg(test)]
mod tests {
    use super::{SQL_001_POSTGRES, SQL_001_SQLITE};

    /// The migration_helper statement splitter splits on bare `;` outside
    /// `--` line comments. Make sure every embedded statement parses into
    /// at least the table count we expect — protects against a stray
    /// `;` inside a comment / string literal silently dropping DDL.
    // Match against the canonical DDL prefix, not bare "CREATE TABLE" — the
    // header comment in the SQL file mentions "CREATE TABLE IF NOT EXISTS"
    // descriptively, which would otherwise inflate the count.
    fn count_create_table(sql: &str) -> usize {
        sql.match_indices("CREATE TABLE IF NOT EXISTS ").count()
    }
    fn count_create_index(sql: &str) -> usize {
        sql.match_indices("CREATE INDEX IF NOT EXISTS ").count()
    }

    #[test]
    fn sqlite_script_has_expected_tables_and_indexes() {
        // 2 tables: contexts + entries
        assert_eq!(count_create_table(SQL_001_SQLITE), 2);
        // 9 indexes: 5 on contexts (updated_at, type, status, sender_id,
        // parent_id) + 4 on entries (context_id+created_at, context_id,
        // context_id+kind, kind)
        assert_eq!(count_create_index(SQL_001_SQLITE), 9);
        // Spot-check a few key names so a rename here breaks the test.
        assert!(SQL_001_SQLITE.contains("suppers_ai__messages__contexts"));
        assert!(SQL_001_SQLITE.contains("suppers_ai__messages__entries"));
        assert!(SQL_001_SQLITE.contains("idx_messages_contexts_updated_at"));
        assert!(SQL_001_SQLITE.contains("idx_messages_entries_context_id_created_at"));
    }

    #[test]
    fn postgres_script_has_expected_tables_and_indexes() {
        assert_eq!(count_create_table(SQL_001_POSTGRES), 2);
        assert_eq!(count_create_index(SQL_001_POSTGRES), 9);
        assert!(SQL_001_POSTGRES.contains("suppers_ai__messages__contexts"));
        assert!(SQL_001_POSTGRES.contains("suppers_ai__messages__entries"));
    }
}
