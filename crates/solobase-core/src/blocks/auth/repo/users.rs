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
    pub disabled: bool,
    pub email_verified: bool,
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
    // Defensive bool decode — handles JSON bool, integer (TEXT-int via sqlite),
    // and string ('true'/'1') so the row reads cleanly across all backends.
    let b = |k: &str| match m.get(k) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        Some(Value::String(s)) => s == "1" || s.eq_ignore_ascii_case("true"),
        _ => false,
    };
    Ok(UserRow {
        id: s("id").ok_or_else(|| RepoError::Db("missing id".into()))?,
        email: s("email").ok_or_else(|| RepoError::Db("missing email".into()))?,
        display_name: s("display_name").unwrap_or_default(),
        avatar_url: s("avatar_url"),
        role: s("role").unwrap_or_else(|| "user".into()),
        disabled: b("disabled"),
        email_verified: b("email_verified"),
        created_at: s("created_at").unwrap_or_default(),
        updated_at: s("updated_at").unwrap_or_default(),
    })
}

pub async fn insert(ctx: &dyn Context, new: NewUser) -> Result<UserRow, RepoError> {
    let id = Uuid::now_v7().to_string();
    let now = now_iso();
    let mut data: HashMap<String, Value> = HashMap::new();
    data.insert("id".into(), json!(id));
    data.insert("email".into(), json!(new.email));
    data.insert("display_name".into(), json!(new.display_name));
    if let Some(a) = new.avatar_url.as_deref() {
        data.insert("avatar_url".into(), json!(a));
    }
    data.insert("role".into(), json!(new.role));
    data.insert("created_at".into(), json!(now));
    data.insert("updated_at".into(), json!(now));

    let rec = db::create(ctx, TABLE, data)
        .await
        .map_err(|e| RepoError::Db(format!("insert: {e}")))?;
    row_from_map(&rec.data)
}

pub async fn find_by_email(ctx: &dyn Context, email: &str) -> Result<Option<UserRow>, RepoError> {
    use wafer_block::ErrorCode;
    match db::get_by_field(ctx, TABLE, "email", json!(email)).await {
        Ok(rec) => Ok(Some(row_from_map(&rec.data)?)),
        Err(e) if e.code == ErrorCode::NOT_FOUND => Ok(None),
        Err(e) => Err(RepoError::Db(format!("select by email: {e}"))),
    }
}

/// Returns the number of rows currently in `suppers_ai__auth__users`.
///
/// Used by the block's bootstrap logic to decide whether to create the first
/// admin user. A non-zero count means "already bootstrapped — no-op".
pub async fn count(ctx: &dyn Context) -> Result<u64, RepoError> {
    let n = db::count(ctx, TABLE, &[])
        .await
        .map_err(|e| RepoError::Db(format!("users count: {e}")))?;
    Ok(n.max(0) as u64)
}

pub async fn find_by_id(ctx: &dyn Context, id: &str) -> Result<Option<UserRow>, RepoError> {
    use wafer_block::ErrorCode;
    match db::get(ctx, TABLE, id).await {
        Ok(rec) => Ok(Some(row_from_map(&rec.data)?)),
        Err(e) if e.code == ErrorCode::NOT_FOUND => Ok(None),
        Err(e) => Err(RepoError::Db(format!("select by id: {e}"))),
    }
}

/// Read the `email_verified` flag for a user. Returns `Ok(false)` when the
/// user row is missing (the doc-claim path) and propagates other DB errors.
///
/// Accepts both SQLite TEXT-int (`'0'`/`'1'`), Postgres BOOLEAN, JSON `bool`,
/// and string `'true'`/`'false'` via `RecordExt::bool_field`.
pub async fn is_email_verified(ctx: &dyn Context, user_id: &str) -> Result<bool, RepoError> {
    use wafer_block::ErrorCode;

    use crate::blocks::helpers::RecordExt;

    match db::get(ctx, TABLE, user_id).await {
        Ok(r) => Ok(r.bool_field("email_verified")),
        Err(e) if e.code == ErrorCode::NOT_FOUND => Ok(false),
        Err(e) => Err(RepoError::Db(format!("get user {user_id}: {e}"))),
    }
}

/// Set the `email_verified` flag for a user. Stamps `updated_at` so admin
/// auditing reflects the change.
///
/// Stores the value as the JSON boolean — both `wafer-block-sqlite`
/// (TEXT-everything via JSON serialization) and `wafer-block-postgres`
/// (typed BOOLEAN) accept it. `RecordExt::bool_field` round-trips both.
pub async fn set_email_verified(
    ctx: &dyn Context,
    user_id: &str,
    verified: bool,
) -> Result<(), RepoError> {
    let mut data = std::collections::HashMap::new();
    data.insert("email_verified".to_string(), json!(verified));
    crate::blocks::helpers::stamp_updated(&mut data);

    db::update(ctx, TABLE, user_id, data)
        .await
        .map_err(|e| RepoError::Db(format!("set email_verified for {user_id}: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod email_verified_tests {
    use super::*;
    use crate::test_support::TestContext;

    /// Seeds a user with `email_verified` defaulted to 0 (unverified). Mirrors
    /// the migration's `INTEGER NOT NULL DEFAULT 0` shape.
    async fn seed_user(ctx: &TestContext, user_id: &str) {
        db::exec_raw(
            ctx,
            "INSERT INTO suppers_ai__auth__users \
             (id, email, display_name, role, email_verified, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            &[
                json!(user_id),
                json!(format!("{user_id}@example.com")),
                json!(user_id),
                json!("user"),
                json!(0),
                json!("2026-01-01T00:00:00Z"),
                json!("2026-01-01T00:00:00Z"),
            ],
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn unverified_by_default_after_seed() {
        let ctx = TestContext::with_auth().await;
        seed_user(&ctx, "user-a").await;
        assert!(!is_email_verified(&ctx, "user-a").await.unwrap());
    }

    #[tokio::test]
    async fn set_then_read_round_trips_true() {
        let ctx = TestContext::with_auth().await;
        seed_user(&ctx, "user-a").await;
        set_email_verified(&ctx, "user-a", true).await.unwrap();
        assert!(is_email_verified(&ctx, "user-a").await.unwrap());
    }

    #[tokio::test]
    async fn set_then_read_round_trips_false() {
        let ctx = TestContext::with_auth().await;
        seed_user(&ctx, "user-a").await;
        // First flip to true so the false-write isn't a no-op against the
        // default value.
        set_email_verified(&ctx, "user-a", true).await.unwrap();
        set_email_verified(&ctx, "user-a", false).await.unwrap();
        assert!(!is_email_verified(&ctx, "user-a").await.unwrap());
    }

    #[tokio::test]
    async fn missing_user_returns_false() {
        let ctx = TestContext::with_auth().await;
        // Doc-claim: missing user → Ok(false). Real DB errors still propagate
        // (verified separately by the per-backend integration tests).
        assert!(!is_email_verified(&ctx, "nonexistent").await.unwrap());
    }
}

#[cfg(test)]
mod typed_client_tests {
    use super::*;
    use crate::test_support::TestContext;

    /// `repo::users::insert` must succeed when the auth block calls it under
    /// WRAP enforcement (own-resource access). Today's `exec_raw` path
    /// requires admin and would fail; this test guards the typed-client
    /// rewrite.
    #[tokio::test]
    async fn insert_succeeds_under_wrap_for_auth_block() {
        let ctx =
            TestContext::with_auth()
                .await
                .with_wrap("suppers-ai/auth", vec![], "suppers-ai/admin");
        let user = insert(
            &ctx,
            NewUser {
                email: "a@b.c".into(),
                display_name: "A".into(),
                avatar_url: None,
                role: "user".into(),
            },
        )
        .await
        .expect("insert under wrap");
        assert_eq!(user.email, "a@b.c");
        assert!(!user.id.is_empty());
    }

    #[tokio::test]
    async fn find_by_email_returns_inserted_row_under_wrap() {
        let ctx =
            TestContext::with_auth()
                .await
                .with_wrap("suppers-ai/auth", vec![], "suppers-ai/admin");
        insert(
            &ctx,
            NewUser {
                email: "x@y.z".into(),
                display_name: "X".into(),
                avatar_url: Some("https://example.com/a.png".into()),
                role: "admin".into(),
            },
        )
        .await
        .unwrap();
        let got = find_by_email(&ctx, "x@y.z").await.unwrap().unwrap();
        assert_eq!(got.role, "admin");
        assert_eq!(got.avatar_url.as_deref(), Some("https://example.com/a.png"));
    }

    #[tokio::test]
    async fn count_reports_zero_then_one() {
        let ctx =
            TestContext::with_auth()
                .await
                .with_wrap("suppers-ai/auth", vec![], "suppers-ai/admin");
        assert_eq!(count(&ctx).await.unwrap(), 0);
        insert(
            &ctx,
            NewUser {
                email: "c@d.e".into(),
                display_name: "C".into(),
                avatar_url: None,
                role: "user".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(count(&ctx).await.unwrap(), 1);
    }
}
