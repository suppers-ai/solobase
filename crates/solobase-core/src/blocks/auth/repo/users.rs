//! Row-level access over `suppers_ai__auth__users`.

use std::collections::HashMap;

use serde_json::{json, Value};
use uuid::Uuid;
use wafer_core::clients::database as db;
use wafer_run::context::Context;

use super::RepoError;

pub const TABLE: &str = "suppers_ai__auth__users";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserRow {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct NewUser {
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub role: String,
}

fn now_iso() -> String {
    // Matches the plain `...Z` style already used by the migration tests; the
    // exact formatting is not load-bearing beyond being comparable and ISO-8601.
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn row_from_map(m: &HashMap<String, Value>) -> Result<UserRow, RepoError> {
    let s = |k: &str| m.get(k).and_then(Value::as_str).map(str::to_owned);
    Ok(UserRow {
        id: s("id").ok_or_else(|| RepoError::Db("missing id".into()))?,
        email: s("email").ok_or_else(|| RepoError::Db("missing email".into()))?,
        display_name: s("display_name").unwrap_or_default(),
        avatar_url: s("avatar_url"),
        role: s("role").unwrap_or_else(|| "user".into()),
        created_at: s("created_at").unwrap_or_default(),
        updated_at: s("updated_at").unwrap_or_default(),
    })
}

pub async fn insert(ctx: &dyn Context, new: NewUser) -> Result<UserRow, RepoError> {
    let id = Uuid::now_v7().to_string();
    let now = now_iso();
    db::exec_raw(
        ctx,
        &format!(
            "INSERT INTO {TABLE} (id, email, display_name, avatar_url, role, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        ),
        &[
            json!(id),
            json!(new.email),
            json!(new.display_name),
            match new.avatar_url.as_deref() {
                Some(a) => json!(a),
                None => Value::Null,
            },
            json!(new.role),
            json!(now),
            json!(now),
        ],
    )
    .await
    .map_err(|e| RepoError::Db(format!("insert: {e}")))?;

    find_by_id(ctx, &id).await?.ok_or(RepoError::NotFound)
}

pub async fn find_by_email(ctx: &dyn Context, email: &str) -> Result<Option<UserRow>, RepoError> {
    let rows = db::query_raw(
        ctx,
        &format!("SELECT * FROM {TABLE} WHERE email = ?"),
        &[json!(email)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("select by email: {e}")))?;
    match rows.first() {
        Some(r) => Ok(Some(row_from_map(&r.data)?)),
        None => Ok(None),
    }
}

/// Returns the number of rows currently in `suppers_ai__auth__users`.
///
/// Used by the block's bootstrap logic to decide whether to create the first
/// admin user. A non-zero count means "already bootstrapped — no-op".
pub async fn count(ctx: &dyn Context) -> Result<u64, RepoError> {
    let rows = db::query_raw(ctx, &format!("SELECT COUNT(*) AS n FROM {TABLE}"), &[])
        .await
        .map_err(|e| RepoError::Db(format!("users count: {e}")))?;
    let n = rows
        .first()
        .and_then(|r| r.data.get("n"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| RepoError::Db("count returned no rows".into()))?;
    Ok(n.max(0) as u64)
}

pub async fn find_by_id(ctx: &dyn Context, id: &str) -> Result<Option<UserRow>, RepoError> {
    let rows = db::query_raw(
        ctx,
        &format!("SELECT * FROM {TABLE} WHERE id = ?"),
        &[json!(id)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("select by id: {e}")))?;
    match rows.first() {
        Some(r) => Ok(Some(row_from_map(&r.data)?)),
        None => Ok(None),
    }
}
