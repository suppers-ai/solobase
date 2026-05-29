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

pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    migration_helper::apply_migrations(
        ctx,
        "suppers-ai/auth",
        &[
            SQL_001_SQLITE,
            SQL_002_SQLITE,
            SQL_003_SQLITE,
            SQL_004_SQLITE,
            SQL_005_SQLITE,
        ],
        &[
            SQL_001_POSTGRES,
            SQL_002_POSTGRES,
            SQL_003_POSTGRES,
            SQL_004_POSTGRES,
            SQL_005_POSTGRES,
        ],
    )
    .await
}
