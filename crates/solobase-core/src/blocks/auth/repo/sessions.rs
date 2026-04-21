//! Row-level access over `suppers_ai__auth__sessions`.
//!
//! `token_hash` is the sha256 of the raw session token — the raw token never
//! lives past its issuance helper (see AuthService in a later task).

use std::collections::HashMap;

use serde_json::{json, Value};
use wafer_core::clients::database as db;
use wafer_run::context::Context;

use super::RepoError;

pub const TABLE: &str = "suppers_ai__auth__sessions";

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub token_hash: Vec<u8>,
    pub user_id: String,
    pub created_at: String,
    pub last_used_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone)]
pub struct NewSession {
    pub token_hash: Vec<u8>,
    pub user_id: String,
    pub expires_at: String,
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Decode a `Value` returned by the database client into a byte vector.
///
/// The sqlite service stores byte arrays passed in as `json!(&[u8])` as the
/// JSON array text representation and returns them parsed back into
/// `Value::Array` on read. The helper tolerates a couple of other shapes so
/// the repo layer stays forgiving if a backend round-trips them differently.
fn decode_bytes(v: &Value) -> Option<Vec<u8>> {
    match v {
        Value::Array(arr) => Some(
            arr.iter()
                .filter_map(|x| x.as_u64().map(|n| n as u8))
                .collect(),
        ),
        Value::String(s) => Some(s.as_bytes().to_vec()),
        _ => None,
    }
}

fn row_from_map(m: &HashMap<String, Value>) -> Result<SessionRow, RepoError> {
    let s = |k: &str| m.get(k).and_then(Value::as_str).map(str::to_owned);
    let token_hash = m
        .get("token_hash")
        .and_then(decode_bytes)
        .ok_or_else(|| RepoError::Db("missing token_hash".into()))?;
    Ok(SessionRow {
        token_hash,
        user_id: s("user_id").ok_or_else(|| RepoError::Db("missing user_id".into()))?,
        created_at: s("created_at").unwrap_or_default(),
        last_used_at: s("last_used_at").unwrap_or_default(),
        expires_at: s("expires_at").unwrap_or_default(),
    })
}

pub async fn insert(ctx: &dyn Context, new: NewSession) -> Result<(), RepoError> {
    let now = now_iso();
    db::exec_raw(
        ctx,
        &format!(
            "INSERT INTO {TABLE} (token_hash, user_id, created_at, last_used_at, expires_at) VALUES (?, ?, ?, ?, ?)",
        ),
        &[
            json!(new.token_hash),
            json!(new.user_id),
            json!(now),
            json!(now),
            json!(new.expires_at),
        ],
    )
    .await
    .map_err(|e| RepoError::Db(format!("insert session: {e}")))?;
    Ok(())
}

pub async fn find_by_token_hash(
    ctx: &dyn Context,
    hash: &[u8],
) -> Result<Option<SessionRow>, RepoError> {
    let rows = db::query_raw(
        ctx,
        &format!("SELECT * FROM {TABLE} WHERE token_hash = ?"),
        &[json!(hash)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("session select: {e}")))?;
    match rows.first() {
        Some(r) => Ok(Some(row_from_map(&r.data)?)),
        None => Ok(None),
    }
}

/// Bumps `last_used_at` to the current time for the row identified by
/// `hash`. Silently no-ops if the row is missing — callers treat liveness
/// as a best-effort signal.
pub async fn touch_last_used(ctx: &dyn Context, hash: &[u8]) -> Result<(), RepoError> {
    let now = now_iso();
    db::exec_raw(
        ctx,
        &format!("UPDATE {TABLE} SET last_used_at = ? WHERE token_hash = ?"),
        &[json!(now), json!(hash)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("session touch: {e}")))?;
    Ok(())
}

/// Deletes rows whose `expires_at < cutoff`. Returns the number deleted.
///
/// `cutoff` is compared as an ISO-8601 string; the migration schema stores
/// timestamps in the same text format.
pub async fn delete_expired(ctx: &dyn Context, cutoff: &str) -> Result<u64, RepoError> {
    let affected = db::exec_raw(
        ctx,
        &format!("DELETE FROM {TABLE} WHERE expires_at < ?"),
        &[json!(cutoff)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("delete expired: {e}")))?;
    Ok(affected.max(0) as u64)
}
