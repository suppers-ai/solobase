//! Auth block migrations. Delegated to `crate::migration_helper`.
//!
//! Hash-gated apply — runs only when the SQL hash differs from the recorded
//! `current_hash` in `suppers_ai__admin__block_settings`. Concatenated SQL of
//! all migration scripts is hashed and tracked.

use wafer_run::context::Context;

use crate::migration_helper;

const SQL_001_SQLITE: &str = include_str!("001_auth_schema.sqlite.sql");
const SQL_001_POSTGRES: &str = include_str!("001_auth_schema.postgres.sql");
const SQL_002_SQLITE: &str = include_str!("002_reserved_orgs.sqlite.sql");
const SQL_002_POSTGRES: &str = include_str!("002_reserved_orgs.postgres.sql");
const SQL_003_SQLITE: &str = include_str!("003_oauth_pkce_states.sqlite.sql");
const SQL_003_POSTGRES: &str = include_str!("003_oauth_pkce_states.postgres.sql");
const SQL_004_SQLITE: &str = include_str!("004_refresh_tokens.sqlite.sql");
const SQL_004_POSTGRES: &str = include_str!("004_refresh_tokens.postgres.sql");
const SQL_005_SQLITE: &str = include_str!("005_jwt_blocklist.sqlite.sql");
const SQL_005_POSTGRES: &str = include_str!("005_jwt_blocklist.postgres.sql");
const SQL_006_SQLITE: &str = include_str!("006_user_extended_fields.sqlite.sql");
const SQL_006_POSTGRES: &str = include_str!("006_user_extended_fields.postgres.sql");
const SQL_007_SQLITE: &str = include_str!("007_api_keys.sqlite.sql");
const SQL_007_POSTGRES: &str = include_str!("007_api_keys.postgres.sql");

/// Ordered SQLite migration scripts for this block, as `(basename, content)`
/// pairs. Single source for both the runtime `apply()` below and the
/// Cloudflare-build D1 migration registry (`crate::migrations`). Order here
/// is the apply order — keep it identical to `apply()`'s sqlite slice.
pub(crate) const SQLITE_MIGRATIONS: &[(&str, &str)] = &[
    ("001_auth_schema", SQL_001_SQLITE),
    ("002_reserved_orgs", SQL_002_SQLITE),
    ("003_oauth_pkce_states", SQL_003_SQLITE),
    ("004_refresh_tokens", SQL_004_SQLITE),
    ("005_jwt_blocklist", SQL_005_SQLITE),
    ("006_user_extended_fields", SQL_006_SQLITE),
    ("007_api_keys", SQL_007_SQLITE),
];

pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    let sqlite: Vec<&str> = SQLITE_MIGRATIONS.iter().map(|(_, sql)| *sql).collect();
    migration_helper::apply_migrations(
        ctx,
        "suppers-ai/auth",
        &sqlite,
        &[
            SQL_001_POSTGRES,
            SQL_002_POSTGRES,
            SQL_003_POSTGRES,
            SQL_004_POSTGRES,
            SQL_005_POSTGRES,
            SQL_006_POSTGRES,
            SQL_007_POSTGRES,
        ],
    )
    .await
}
