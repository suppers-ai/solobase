//! Row-level access over `suppers_ai__auth__orgs`.
//!
//! Migration 001 creates the table with `UNIQUE(name)` and a partial unique
//! index over `(verified_via, verified_ref) WHERE is_reserved = 0` — this repo
//! surfaces the two distinct conflicts as two distinct error variants so the
//! HTTP layer (Plan C Cluster 2) can map each to its own 409.

use std::collections::HashMap;

use serde_json::{json, Value};
use uuid::Uuid;
use wafer_core::clients::database as db;
use wafer_run::context::Context;

pub const TABLE: &str = "suppers_ai__auth__orgs";

/// Full row shape returned by [`find_by_name`] and [`upsert_claimed`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgRow {
    pub id: String,
    pub name: String,
    pub owner_user_id: Option<String>,
    pub verified_via: Option<String>,
    pub verified_ref: Option<String>,
    pub is_reserved: bool,
    pub created_at: String,
}

/// Errors surfaced by this repo. Two distinct unique-constraint violations
/// produce two distinct variants — callers in Plan C Cluster 2 need to map
/// each to its proper 409 message.
#[derive(thiserror::Error, Debug)]
pub enum OrgsRepoError {
    #[error("org with that name already exists")]
    NameTaken,
    #[error("that provider org is already claimed by another user")]
    AlreadyClaimed,
    #[error("db: {0}")]
    Db(String),
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn row_from_map(m: &HashMap<String, Value>) -> Result<OrgRow, OrgsRepoError> {
    let s = |k: &str| m.get(k).and_then(Value::as_str).map(str::to_owned);
    let is_reserved = match m.get("is_reserved") {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        Some(Value::String(s)) => s == "1" || s.eq_ignore_ascii_case("true"),
        _ => false,
    };
    Ok(OrgRow {
        id: s("id").ok_or_else(|| OrgsRepoError::Db("missing id".into()))?,
        name: s("name").ok_or_else(|| OrgsRepoError::Db("missing name".into()))?,
        owner_user_id: s("owner_user_id"),
        verified_via: s("verified_via"),
        verified_ref: s("verified_ref"),
        is_reserved,
        created_at: s("created_at").unwrap_or_default(),
    })
}

/// Look up a single org by its `name` column (UNIQUE). Returns `Ok(None)` if
/// no such row exists.
pub async fn find_by_name(ctx: &dyn Context, name: &str) -> Result<Option<OrgRow>, OrgsRepoError> {
    let rows = db::query_raw(
        ctx,
        &format!("SELECT * FROM {TABLE} WHERE name = ?"),
        &[json!(name)],
    )
    .await
    .map_err(|e| OrgsRepoError::Db(format!("orgs find_by_name: {e}")))?;
    match rows.first() {
        Some(r) => Ok(Some(row_from_map(&r.data)?)),
        None => Ok(None),
    }
}

/// Return all orgs owned by `user_id`, ordered by `created_at` ASC for
/// stable rendering. Empty Vec if the user owns none.
pub async fn list_for_user(ctx: &dyn Context, user_id: &str) -> Result<Vec<OrgRow>, OrgsRepoError> {
    let rows = db::query_raw(
        ctx,
        &format!("SELECT * FROM {TABLE} WHERE owner_user_id = ? ORDER BY created_at ASC"),
        &[json!(user_id)],
    )
    .await
    .map_err(|e| OrgsRepoError::Db(format!("orgs list_for_user: {e}")))?;
    rows.iter().map(|r| row_from_map(&r.data)).collect()
}

/// Payload for [`upsert_claimed`]. Borrowed fields — caller keeps ownership.
#[derive(Debug, Clone, Copy)]
pub struct NewClaim<'a> {
    pub name: &'a str,
    pub owner_user_id: &'a str,
    pub verified_via: &'a str,
    pub verified_ref: &'a str,
}

/// Insert a new claimed (non-reserved) org row. The two unique-constraint
/// conflicts are distinguished by running pre-checks against the table before
/// the INSERT: the `wafer-run/database` surface doesn't currently let us
/// inspect the constraint name that fired, so we fail fast with the right
/// variant instead of relying on the underlying error string shape.
///
/// Returns the inserted [`OrgRow`].
pub async fn upsert_claimed(
    ctx: &dyn Context,
    claim: NewClaim<'_>,
) -> Result<OrgRow, OrgsRepoError> {
    // 1) (verified_via, verified_ref) already claimed → AlreadyClaimed.
    let existing = db::query_raw(
        ctx,
        &format!(
            "SELECT id FROM {TABLE} WHERE verified_via = ? AND verified_ref = ? AND is_reserved = 0"
        ),
        &[json!(claim.verified_via), json!(claim.verified_ref)],
    )
    .await
    .map_err(|e| OrgsRepoError::Db(format!("orgs claim-check: {e}")))?;
    if !existing.is_empty() {
        return Err(OrgsRepoError::AlreadyClaimed);
    }

    // 2) `name` already taken (by any row, reserved or claimed) → NameTaken.
    if find_by_name(ctx, claim.name).await?.is_some() {
        return Err(OrgsRepoError::NameTaken);
    }

    // 3) Insert.
    let id = Uuid::now_v7().to_string();
    let now = now_iso();
    db::exec_raw(
        ctx,
        &format!(
            "INSERT INTO {TABLE} (id, name, owner_user_id, verified_via, verified_ref, is_reserved, created_at) \
             VALUES (?, ?, ?, ?, ?, 0, ?)"
        ),
        &[
            json!(id),
            json!(claim.name),
            json!(claim.owner_user_id),
            json!(claim.verified_via),
            json!(claim.verified_ref),
            json!(now),
        ],
    )
    .await
    .map_err(|e| OrgsRepoError::Db(format!("orgs insert: {e}")))?;

    find_by_name(ctx, claim.name)
        .await?
        .ok_or_else(|| OrgsRepoError::Db("insert returned no row".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestContext;

    #[tokio::test]
    async fn find_by_name_returns_none_for_missing_org() {
        let ctx = TestContext::with_auth().await;
        let result = find_by_name(&ctx, "nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn find_by_name_returns_inserted_org() {
        let ctx = TestContext::with_auth().await;

        // Create a user first (foreign key constraint on owner_user_id)
        db::exec_raw(
            &ctx,
            "INSERT INTO suppers_ai__auth__users (id, email, display_name, role, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            &[
                json!("user-a"),
                json!("alice@example.com"),
                json!("Alice"),
                json!("user"),
                json!("2026-01-01T00:00:00Z"),
                json!("2026-01-01T00:00:00Z"),
            ],
        )
        .await
        .unwrap();

        upsert_claimed(
            &ctx,
            NewClaim {
                name: "acme",
                owner_user_id: "user-a",
                verified_via: "github",
                verified_ref: "gh-1",
            },
        )
        .await
        .unwrap();

        let row = find_by_name(&ctx, "acme").await.unwrap().unwrap();
        assert_eq!(row.name, "acme");
        assert_eq!(row.owner_user_id.as_deref(), Some("user-a"));
        assert_eq!(row.verified_via.as_deref(), Some("github"));
        assert_eq!(row.verified_ref.as_deref(), Some("gh-1"));
    }

    #[tokio::test]
    async fn list_for_user_returns_only_caller_orgs_ordered_by_created_at() {
        let ctx = TestContext::with_auth().await;

        // Seed users (FK constraint on owner_user_id).
        for user_id in ["user-a", "user-b"] {
            db::exec_raw(
                &ctx,
                "INSERT INTO suppers_ai__auth__users (id, email, display_name, role, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?)",
                &[
                    json!(user_id),
                    json!(format!("{user_id}@example.com")),
                    json!(user_id),
                    json!("user"),
                    json!("2026-01-01T00:00:00Z"),
                    json!("2026-01-01T00:00:00Z"),
                ],
            )
            .await
            .unwrap();
        }

        // user-a claims two orgs; user-b claims one.
        upsert_claimed(
            &ctx,
            NewClaim {
                name: "alpha",
                owner_user_id: "user-a",
                verified_via: "github",
                verified_ref: "gh-1",
            },
        )
        .await
        .unwrap();
        upsert_claimed(
            &ctx,
            NewClaim {
                name: "beta",
                owner_user_id: "user-a",
                verified_via: "google",
                verified_ref: "gg-2",
            },
        )
        .await
        .unwrap();
        upsert_claimed(
            &ctx,
            NewClaim {
                name: "gamma",
                owner_user_id: "user-b",
                verified_via: "github",
                verified_ref: "gh-3",
            },
        )
        .await
        .unwrap();

        let a = list_for_user(&ctx, "user-a").await.unwrap();
        let b = list_for_user(&ctx, "user-b").await.unwrap();
        let c = list_for_user(&ctx, "user-c").await.unwrap();

        let names_a: Vec<&str> = a.iter().map(|o| o.name.as_str()).collect();
        let names_b: Vec<&str> = b.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names_a, vec!["alpha", "beta"]);
        assert_eq!(names_b, vec!["gamma"]);
        assert!(c.is_empty());
    }
}
