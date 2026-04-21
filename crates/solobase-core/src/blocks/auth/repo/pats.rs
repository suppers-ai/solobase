//! Row-level access over `suppers_ai__auth__personal_access_tokens`.

use std::collections::HashMap;

use serde_json::{json, Value};
use wafer_core::clients::database as db;
use wafer_run::context::Context;

use super::RepoError;

pub const TABLE: &str = "suppers_ai__auth__personal_access_tokens";

#[derive(Debug, Clone)]
pub struct PatRow {
    pub token_hash: Vec<u8>,
    pub user_id: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewPat {
    pub token_hash: Vec<u8>,
    pub user_id: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<String>,
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

/// Decode scopes from whatever shape the backend returned.
///
/// We stored a JSON-encoded array as a string. The sqlite service helpfully
/// auto-parses text starting with `[`/`{` back into a `Value::Array`, so the
/// repo layer accepts both the post-parsing array and the raw JSON string.
fn decode_scopes(v: &Value) -> Result<Vec<String>, RepoError> {
    match v {
        Value::Array(arr) => Ok(arr
            .iter()
            .filter_map(|x| x.as_str().map(str::to_owned))
            .collect()),
        Value::String(s) => serde_json::from_str::<Vec<String>>(s)
            .map_err(|e| RepoError::Db(format!("scopes json: {e}"))),
        Value::Null => Ok(Vec::new()),
        other => Err(RepoError::Db(format!(
            "scopes has unexpected shape: {other}"
        ))),
    }
}

fn row_from_map(m: &HashMap<String, Value>) -> Result<PatRow, RepoError> {
    let s = |k: &str| m.get(k).and_then(Value::as_str).map(str::to_owned);
    let token_hash = m
        .get("token_hash")
        .and_then(decode_bytes)
        .ok_or_else(|| RepoError::Db("missing token_hash".into()))?;
    let scopes = match m.get("scopes") {
        Some(v) => decode_scopes(v)?,
        None => Vec::new(),
    };
    Ok(PatRow {
        token_hash,
        user_id: s("user_id").ok_or_else(|| RepoError::Db("missing user_id".into()))?,
        name: s("name").unwrap_or_default(),
        scopes,
        created_at: s("created_at").unwrap_or_default(),
        last_used_at: s("last_used_at"),
        expires_at: s("expires_at"),
    })
}

pub async fn insert(ctx: &dyn Context, new: NewPat) -> Result<(), RepoError> {
    let now = now_iso();
    let scopes_json = serde_json::to_string(&new.scopes)
        .map_err(|e| RepoError::Db(format!("scopes ser: {e}")))?;
    db::exec_raw(
        ctx,
        &format!(
            "INSERT INTO {TABLE} (token_hash, user_id, name, scopes, created_at, last_used_at, expires_at) VALUES (?, ?, ?, ?, ?, NULL, ?)",
        ),
        &[
            json!(new.token_hash),
            json!(new.user_id),
            json!(new.name),
            json!(scopes_json),
            json!(now),
            match new.expires_at.as_deref() {
                Some(s) => json!(s),
                None => Value::Null,
            },
        ],
    )
    .await
    .map_err(|e| RepoError::Db(format!("pat insert: {e}")))?;
    Ok(())
}

/// List every PAT belonging to `user_id`, newest first.
///
/// Ordering is by `created_at DESC` so the UI can render "most recent at the
/// top". `token_hash` is returned on the row but API callers are expected to
/// strip it before serialising to the client.
pub async fn list_for_user(ctx: &dyn Context, user_id: &str) -> Result<Vec<PatRow>, RepoError> {
    let rows = db::query_raw(
        ctx,
        &format!("SELECT * FROM {TABLE} WHERE user_id = ? ORDER BY created_at DESC"),
        &[json!(user_id)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("pat list: {e}")))?;
    rows.iter().map(|r| row_from_map(&r.data)).collect()
}

pub async fn find_by_token_hash(
    ctx: &dyn Context,
    hash: &[u8],
) -> Result<Option<PatRow>, RepoError> {
    let rows = db::query_raw(
        ctx,
        &format!("SELECT * FROM {TABLE} WHERE token_hash = ?"),
        &[json!(hash)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("pat select: {e}")))?;
    match rows.first() {
        Some(r) => Ok(Some(row_from_map(&r.data)?)),
        None => Ok(None),
    }
}

/// Bumps `last_used_at` for the row identified by `hash`. Silently no-ops if
/// the row is missing.
pub async fn touch_last_used(ctx: &dyn Context, hash: &[u8]) -> Result<(), RepoError> {
    let now = now_iso();
    db::exec_raw(
        ctx,
        &format!("UPDATE {TABLE} SET last_used_at = ? WHERE token_hash = ?"),
        &[json!(now), json!(hash)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("pat touch: {e}")))?;
    Ok(())
}
