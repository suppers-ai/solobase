//! Auth block migrations. Delegated to `crate::migration_helper`.
//!
//! Hash-gated apply — runs only when the SQL hash differs from the recorded
//! `current_hash` in `suppers_ai__admin__block_settings`. Concatenated SQL of
//! both migration scripts is hashed and tracked.

use wafer_core::clients::config;
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

pub async fn apply(ctx: &dyn Context) -> Result<(), String> {
    let backend = config::get_default(ctx, "SOLOBASE_SHARED__DATABASE__BACKEND", "sqlite")
        .await
        .to_ascii_lowercase();
    let sql = if backend == "postgres" {
        format!("{SQL_001_POSTGRES}\n{SQL_002_POSTGRES}\n{SQL_003_POSTGRES}\n{SQL_004_POSTGRES}")
    } else {
        format!("{SQL_001_SQLITE}\n{SQL_002_SQLITE}\n{SQL_003_SQLITE}\n{SQL_004_SQLITE}")
    };
    migration_helper::apply_if_blessed(ctx, "suppers-ai/auth", &sql).await
}
