//! Row-level access over `suppers_ai__auth__local_credentials`.
//!
//! Holds the Argon2id `password_hash` for users who authenticate with
//! email + password. OAuth-only users have no row here. The `user_id` column
//! is the primary key and references `suppers_ai__auth__users(id)` with
//! `ON DELETE CASCADE`.

use std::collections::HashMap;

use serde_json::{json, Value};
use wafer_core::clients::database as db;
use wafer_run::context::Context;

use super::RepoError;

pub const TABLE: &str = "suppers_ai__auth__local_credentials";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalCredentialRow {
    pub user_id: String,
    pub password_hash: String,
    pub must_reset: bool,
    pub created_at: String,
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn row_from_map(m: &HashMap<String, Value>) -> Result<LocalCredentialRow, RepoError> {
    let s = |k: &str| m.get(k).and_then(Value::as_str).map(str::to_owned);
    let must_reset = m
        .get("must_reset")
        .and_then(|v| v.as_i64())
        .map(|n| n != 0)
        .unwrap_or(false);
    Ok(LocalCredentialRow {
        user_id: s("user_id").ok_or_else(|| RepoError::Db("missing user_id".into()))?,
        password_hash: s("password_hash")
            .ok_or_else(|| RepoError::Db("missing password_hash".into()))?,
        must_reset,
        created_at: s("created_at").unwrap_or_default(),
    })
}

/// Insert a local-credentials row for `user_id`. Fails if a row already
/// exists for that user (PK collision).
pub async fn insert(
    ctx: &dyn Context,
    user_id: &str,
    password_hash: &str,
    must_reset: bool,
) -> Result<(), RepoError> {
    let now = now_iso();
    db::exec_raw(
        ctx,
        &format!(
            "INSERT INTO {TABLE} (user_id, password_hash, must_reset, created_at) VALUES (?, ?, ?, ?)",
        ),
        &[
            json!(user_id),
            json!(password_hash),
            json!(if must_reset { 1 } else { 0 }),
            json!(now),
        ],
    )
    .await
    .map_err(|e| RepoError::Db(format!("local_credentials insert: {e}")))?;
    Ok(())
}

pub async fn find_by_user_id(
    ctx: &dyn Context,
    user_id: &str,
) -> Result<Option<LocalCredentialRow>, RepoError> {
    let rows = db::query_raw(
        ctx,
        &format!("SELECT * FROM {TABLE} WHERE user_id = ?"),
        &[json!(user_id)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("local_credentials select: {e}")))?;
    match rows.first() {
        Some(r) => Ok(Some(row_from_map(&r.data)?)),
        None => Ok(None),
    }
}
