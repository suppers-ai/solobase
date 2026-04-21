//! Narrow access over `suppers_ai__auth__bootstrap_tokens`.
//!
//! Only the subset needed by `AuthServiceImpl::require_role` plus a test
//! insert. The full bootstrap-admin lifecycle (issuance, consumption,
//! single-use semantics) lands in Plan A2.

use serde_json::json;
use wafer_core::clients::database as db;
use wafer_run::context::Context;

use super::RepoError;

pub const TABLE: &str = "suppers_ai__auth__bootstrap_tokens";

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Insert a bootstrap token row. Used by Plan A2's bootstrap-admin init;
/// exposed here so the `require_role` integration tests can seed a row
/// directly without re-implementing the SQL.
pub async fn insert(
    ctx: &dyn Context,
    token_hash: Vec<u8>,
    expires_at: &str,
) -> Result<(), RepoError> {
    let now = now_iso();
    db::exec_raw(
        ctx,
        &format!("INSERT INTO {TABLE} (token_hash, created_at, expires_at) VALUES (?, ?, ?)"),
        &[json!(token_hash), json!(now), json!(expires_at)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("bootstrap_tokens insert: {e}")))?;
    Ok(())
}

/// Returns true iff an unexpired row exists with the given hash.
///
/// Compared as ISO-8601 strings to match the text format the migration
/// schema stores.
pub async fn is_valid(ctx: &dyn Context, token_hash: &[u8]) -> Result<bool, RepoError> {
    let now = now_iso();
    let rows = db::query_raw(
        ctx,
        &format!("SELECT 1 AS one FROM {TABLE} WHERE token_hash = ? AND expires_at >= ?"),
        &[json!(token_hash), json!(now)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("bootstrap_tokens lookup: {e}")))?;
    Ok(!rows.is_empty())
}
