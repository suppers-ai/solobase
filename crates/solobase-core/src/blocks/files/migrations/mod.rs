//! Files block migrations. Delegated to `crate::migration_helper`.

use wafer_run::context::Context;

use crate::migration_helper;

const SQL_001_SQLITE: &str = include_str!("001_initial_schema.sqlite.sql");
const SQL_001_POSTGRES: &str = include_str!("001_initial_schema.postgres.sql");

pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    migration_helper::apply_migrations(
        ctx,
        "suppers-ai/files",
        &[SQL_001_SQLITE],
        &[SQL_001_POSTGRES],
    )
    .await
}
