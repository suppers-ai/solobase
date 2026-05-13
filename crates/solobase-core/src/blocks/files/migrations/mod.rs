//! Files block migrations. Delegated to `crate::migration_helper`.

use wafer_core::clients::config;
use wafer_run::context::Context;

use crate::migration_helper;

const SQL_001_SQLITE: &str = include_str!("001_initial_schema.sqlite.sql");
const SQL_001_POSTGRES: &str = include_str!("001_initial_schema.postgres.sql");

pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    let backend = config::get_default(ctx, "SOLOBASE_SHARED__DATABASE__BACKEND", "sqlite")
        .await
        .to_ascii_lowercase();
    let sql = if backend == "postgres" {
        SQL_001_POSTGRES
    } else {
        SQL_001_SQLITE
    };
    migration_helper::apply_if_blessed(ctx, "suppers-ai/files", sql).await
}
