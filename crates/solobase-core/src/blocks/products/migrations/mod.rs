//! Products block migrations. Delegated to `crate::migration_helper`.
//!
//! Mirrors the auth/files migration pattern. SQL is embedded via
//! `include_str!`. Backend selection reads the
//! `SOLOBASE_SHARED__DATABASE__BACKEND` config key
//! (`sqlite` | `postgres`). Falls back to `sqlite` when the config block
//! is not registered.
//!
//! Application is gated by [`crate::migration_helper::apply_if_blessed`]:
//! the helper handles statement splitting + the `current_hash` /
//! `blessed_hash` / `SOLOBASE_RUN_MIGRATIONS` gate, and stamps a row in
//! `suppers_ai__admin__block_settings` once applied.

use wafer_core::clients::config;
use wafer_run::context::Context;

use crate::migration_helper;

const PRODUCTS_BLOCK_NAME: &str = "suppers-ai/products";

const SQL_001_SQLITE: &str = include_str!("001_products_schema.sqlite.sql");
const SQL_001_POSTGRES: &str = include_str!("001_products_schema.postgres.sql");

pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    let backend = config::get_default(ctx, "SOLOBASE_SHARED__DATABASE__BACKEND", "sqlite")
        .await
        .to_ascii_lowercase();
    let sql = if backend == "postgres" {
        SQL_001_POSTGRES
    } else {
        SQL_001_SQLITE
    };
    migration_helper::apply_if_blessed(ctx, PRODUCTS_BLOCK_NAME, sql).await
}
