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

/// Deletes the session row identified by `token_hash`. Returns the number of
/// affected rows (0 if no such row exists).
pub async fn delete_by_token_hash(ctx: &dyn Context, hash: &[u8]) -> Result<u64, RepoError> {
    let affected = db::exec_raw(
        ctx,
        &format!("DELETE FROM {TABLE} WHERE token_hash = ?"),
        &[json!(hash)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("session delete: {e}")))?;
    Ok(affected.max(0) as u64)
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

/// Return all active sessions for `user_id`, ordered by `last_used_at` DESC
/// so the most recently active session sorts first.
pub async fn list_for_user(ctx: &dyn Context, user_id: &str) -> Result<Vec<SessionRow>, RepoError> {
    let rows = db::query_raw(
        ctx,
        &format!("SELECT * FROM {TABLE} WHERE user_id = ? ORDER BY last_used_at DESC"),
        &[json!(user_id)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("session list_for_user: {e}")))?;
    rows.iter().map(|r| row_from_map(&r.data)).collect()
}

/// Delete a session row, but only if it belongs to `user_id`. Returns 0 if
/// the row doesn't exist OR belongs to a different user — never reveals
/// which.
pub async fn delete_for_user(
    ctx: &dyn Context,
    user_id: &str,
    hash: &[u8],
) -> Result<u64, RepoError> {
    let affected = db::exec_raw(
        ctx,
        &format!("DELETE FROM {TABLE} WHERE token_hash = ? AND user_id = ?"),
        &[json!(hash), json!(user_id)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("session delete_for_user: {e}")))?;
    Ok(affected.max(0) as u64)
}

#[cfg(test)]
mod tests_phase_4 {
    use super::*;
    use crate::test_support::TestContext;

    fn fake_session(user_id: &str, hash_byte: u8) -> NewSession {
        NewSession {
            token_hash: vec![hash_byte; 32],
            user_id: user_id.into(),
            expires_at: "2099-01-01T00:00:00Z".into(),
        }
    }

    #[tokio::test]
    async fn list_for_user_returns_only_caller_sessions() {
        let ctx = TestContext::with_auth().await;
        // Seed users (FK constraint).
        for user_id in ["user-a", "user-b"] {
            wafer_core::clients::database::exec_raw(
                &ctx,
                "INSERT INTO suppers_ai__auth__users (id, email, display_name, role, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?)",
                &[
                    serde_json::json!(user_id),
                    serde_json::json!(format!("{user_id}@example.com")),
                    serde_json::json!(user_id),
                    serde_json::json!("user"),
                    serde_json::json!("2026-01-01T00:00:00Z"),
                    serde_json::json!("2026-01-01T00:00:00Z"),
                ],
            )
            .await
            .unwrap();
        }
        insert(&ctx, fake_session("user-a", 0x01)).await.unwrap();
        insert(&ctx, fake_session("user-a", 0x02)).await.unwrap();
        insert(&ctx, fake_session("user-b", 0x03)).await.unwrap();

        let a = list_for_user(&ctx, "user-a").await.unwrap();
        let b = list_for_user(&ctx, "user-b").await.unwrap();
        assert_eq!(a.len(), 2);
        assert_eq!(b.len(), 1);
    }

    #[tokio::test]
    async fn delete_for_user_refuses_other_users_session() {
        let ctx = TestContext::with_auth().await;
        for user_id in ["user-a", "user-b"] {
            wafer_core::clients::database::exec_raw(
                &ctx,
                "INSERT INTO suppers_ai__auth__users (id, email, display_name, role, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?)",
                &[
                    serde_json::json!(user_id),
                    serde_json::json!(format!("{user_id}@example.com")),
                    serde_json::json!(user_id),
                    serde_json::json!("user"),
                    serde_json::json!("2026-01-01T00:00:00Z"),
                    serde_json::json!("2026-01-01T00:00:00Z"),
                ],
            )
            .await
            .unwrap();
        }
        insert(&ctx, fake_session("user-a", 0x01)).await.unwrap();
        insert(&ctx, fake_session("user-b", 0x02)).await.unwrap();

        // user-a tries to revoke user-b's session — should affect 0 rows.
        let affected = delete_for_user(&ctx, "user-a", &vec![0x02; 32])
            .await
            .unwrap();
        assert_eq!(affected, 0);
        // user-b's session is still there.
        assert_eq!(list_for_user(&ctx, "user-b").await.unwrap().len(), 1);

        // user-a can revoke their own.
        let affected = delete_for_user(&ctx, "user-a", &vec![0x01; 32])
            .await
            .unwrap();
        assert_eq!(affected, 1);
        assert_eq!(list_for_user(&ctx, "user-a").await.unwrap().len(), 0);
    }
}
