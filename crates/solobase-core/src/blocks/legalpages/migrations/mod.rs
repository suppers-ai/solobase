//! Legalpages block migrations. Delegated to `crate::migration_helper`.

use wafer_run::context::Context;

use crate::migration_helper;

const SQL_001_SQLITE: &str = include_str!("001_legalpages_schema.sqlite.sql");
const SQL_001_POSTGRES: &str = include_str!("001_legalpages_schema.postgres.sql");

/// Ordered SQLite migration scripts for this block, as `(basename, content)`
/// pairs. Single source for both the runtime `apply()` below and the
/// Cloudflare-build D1 migration registry (`crate::migrations`).
pub(crate) const SQLITE_MIGRATIONS: &[(&str, &str)] = &[("001_legalpages_schema", SQL_001_SQLITE)];

pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    let sqlite: Vec<&str> = SQLITE_MIGRATIONS.iter().map(|(_, sql)| *sql).collect();
    migration_helper::apply_migrations(ctx, "suppers-ai/legalpages", &sqlite, &[SQL_001_POSTGRES])
        .await
}
