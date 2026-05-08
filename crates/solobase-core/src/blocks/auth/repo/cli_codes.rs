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
    let mut data: HashMap<String, Value> = HashMap::new();
    data.insert("code_hash".into(), json!(new.code_hash));
    data.insert("user_id".into(), json!(new.user_id));
    data.insert("created_at".into(), json!(now));
    data.insert("expires_at".into(), json!(new.expires_at));
    db::create(ctx, TABLE, data)
        .await
        .map_err(|e| RepoError::Db(format!("cli_codes insert: {e}")))?;
    Ok(())
}

/// Look up a code by its sha256 hash and simultaneously delete it. Returns
/// `Ok(None)` if missing, or if the row was present but expired (the expired
/// row is still deleted as a side effect — single-use even on timeout).
///
/// Uses `db::take_by_filters` which dispatches to `DELETE … WHERE … RETURNING *`
/// (sqlite 3.35+, postgres) so the read and delete are atomic in a single
/// statement. Falls back to list-then-delete on backends that lack native
/// RETURNING support (the default trait impl).
pub async fn take(ctx: &dyn Context, code_hash: &[u8]) -> Result<Option<CliCodeRow>, RepoError> {
    let rows = db::take_by_filters(
        ctx,
        TABLE,
        vec![db::Filter {
            field: "code_hash".into(),
            operator: db::FilterOp::Equal,
            value: json!(code_hash),
        }],
    )
    .await
    .map_err(|e| RepoError::Db(format!("cli_codes take: {e}")))?;
    let Some(r) = rows.into_iter().next() else {
        return Ok(None);
    };
    let row = row_from_map(&r.data)?;
    if row.expires_at.as_str() < now_iso().as_str() {
        // Row was present but expired — already deleted as a side effect.
        // Surface "not found" so callers treat it the same as missing.
        return Ok(None);
    }
    Ok(Some(row))
}

/// Deletes all rows whose `expires_at < cutoff`. Returns the number deleted.
/// Called by the background sweeper (spec §6) — not required for correctness
/// since [`take`] also drops expired rows on read.
pub async fn delete_expired(ctx: &dyn Context, cutoff: &str) -> Result<u64, RepoError> {
    let n = db::delete_by_filters_count(
        ctx,
        TABLE,
        vec![db::Filter {
            field: "expires_at".into(),
            operator: db::FilterOp::LessThan,
            value: json!(cutoff),
        }],
    )
    .await
    .map_err(|e| RepoError::Db(format!("cli_codes delete_expired: {e}")))?;
    Ok(n.max(0) as u64)
}

// Re-expose the unused `decode_bytes` helper so future inspection of the
// binary `code_hash` column (e.g. test assertions) can reuse the same
// tolerant decoder as the sessions repo.
#[allow(dead_code)]
pub(crate) fn decode_code_hash(v: &Value) -> Option<Vec<u8>> {
    decode_bytes(v)
}
