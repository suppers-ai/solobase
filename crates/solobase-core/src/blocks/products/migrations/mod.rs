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

use wafer_run::context::Context;

use crate::migration_helper;

const PRODUCTS_BLOCK_NAME: &str = "suppers-ai/products";

const SQL_001_SQLITE: &str = include_str!("001_products_schema.sqlite.sql");
const SQL_001_POSTGRES: &str = include_str!("001_products_schema.postgres.sql");
const SQL_002_SQLITE: &str = include_str!("002_default_templates.sqlite.sql");
const SQL_002_POSTGRES: &str = include_str!("002_default_templates.postgres.sql");

/// Ordered SQLite migration scripts for this block, as `(basename, content)`
/// pairs. Single source for both the runtime `apply()` below and the
/// Cloudflare-build D1 migration registry (`crate::migrations`). Order here
/// is the apply order.
pub(crate) const SQLITE_MIGRATIONS: &[(&str, &str)] = &[
    ("001_products_schema", SQL_001_SQLITE),
    ("002_default_templates", SQL_002_SQLITE),
];

pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    let sqlite: Vec<&str> = SQLITE_MIGRATIONS.iter().map(|(_, sql)| *sql).collect();
    migration_helper::apply_migrations(
        ctx,
        PRODUCTS_BLOCK_NAME,
        &sqlite,
        &[SQL_001_POSTGRES, SQL_002_POSTGRES],
    )
    .await
}
