//! Row-level access over `suppers_ai__auth__cli_exchange_codes`.
//!
//! Codes are short-lived (default 15 min) and single-use. [`take`] performs
//! select-then-delete so a given code can only be redeemed once, and expired
//! rows are dropped on lookup as a side effect — a background sweeper (spec
//! §6) is additive, not load-bearing for correctness.

use std::collections::HashMap;

use serde_json::{json, Value};
use wafer_core::clients::database as db;
use wafer_run::context::Context;

use super::RepoError;

pub const TABLE: &str = "suppers_ai__auth__cli_exchange_codes";

/// Payload for [`insert`].
#[derive(Debug, Clone)]
pub struct NewCode<'a> {
    pub code_hash: &'a [u8],
    pub user_id: &'a str,
    /// Absolute expiry time as ISO-8601 (`%Y-%m-%dT%H:%M:%SZ`).
    pub expires_at: &'a str,
}

/// Row returned by [`take`]: the user the code was issued to, plus the
/// original expiry so callers can surface "this code expired 12 s ago".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliCodeRow {
    pub user_id: String,
    pub expires_at: String,
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

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

fn row_from_map(m: &HashMap<String, Value>) -> Result<CliCodeRow, RepoError> {
    let s = |k: &str| m.get(k).and_then(Value::as_str).map(str::to_owned);
    Ok(CliCodeRow {
        user_id: s("user_id").ok_or_else(|| RepoError::Db("missing user_id".into()))?,
        expires_at: s("expires_at").unwrap_or_default(),
    })
}

/// Insert a new exchange code. Hash collisions with an existing row trip the
/// PRIMARY KEY constraint — callers are expected to generate fresh random
/// codes, so a collision is a programmer error, not an expected case.
pub async fn insert(ctx: &dyn Context, new: NewCode<'_>) -> Result<(), RepoError> {
    let now = now_iso();
    db::exec_raw(
        ctx,
        &format!(
            "INSERT INTO {TABLE} (code_hash, user_id, created_at, expires_at) VALUES (?, ?, ?, ?)"
        ),
        &[
            json!(new.code_hash),
            json!(new.user_id),
            json!(now),
            json!(new.expires_at),
        ],
    )
    .await
    .map_err(|e| RepoError::Db(format!("cli_codes insert: {e}")))?;
    Ok(())
}

/// Look up a code by its sha256 hash and simultaneously delete it. Returns
/// `Ok(None)` if missing, or if the row was present but expired (the expired
/// row is still deleted as a side effect — single-use even on timeout).
///
/// Uses `DELETE … RETURNING` (sqlite 3.35+, postgres) so the read and delete
/// are atomic in a single statement. Falls back to a no-op if the backend
/// returns an empty row (e.g. `DELETE` matched nothing).
pub async fn take(ctx: &dyn Context, code_hash: &[u8]) -> Result<Option<CliCodeRow>, RepoError> {
    let rows = db::query_raw(
        ctx,
        &format!("DELETE FROM {TABLE} WHERE code_hash = ? RETURNING user_id, expires_at"),
        &[json!(code_hash)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("cli_codes take: {e}")))?;
    let Some(r) = rows.first() else {
        return Ok(None);
    };
    let row = row_from_map(&r.data)?;
    if row.expires_at.as_str() < now_iso().as_str() {
        // Already deleted by the RETURNING clause — just surface "not found".
        return Ok(None);
    }
    Ok(Some(row))
}

/// Deletes all rows whose `expires_at < cutoff`. Returns the number deleted.
/// Called by the background sweeper (spec §6) — not required for correctness
/// since [`take`] also drops expired rows on read.
pub async fn delete_expired(ctx: &dyn Context, cutoff: &str) -> Result<u64, RepoError> {
    let affected = db::exec_raw(
        ctx,
        &format!("DELETE FROM {TABLE} WHERE expires_at < ?"),
        &[json!(cutoff)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("cli_codes delete_expired: {e}")))?;
    Ok(affected.max(0) as u64)
}

// Re-expose the unused `decode_bytes` helper so future inspection of the
// binary `code_hash` column (e.g. test assertions) can reuse the same
// tolerant decoder as the sessions repo.
#[allow(dead_code)]
pub(crate) fn decode_code_hash(v: &Value) -> Option<Vec<u8>> {
    decode_bytes(v)
}
