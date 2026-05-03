//! Row-level access over `suppers_ai__auth__provider_links`.
//!
//! One row per `(provider, provider_ref)` pair. The primary key matches the
//! spec §3 definition; `upsert` relies on `ON CONFLICT` against that PK so
//! an OAuth login by the same user from the same provider deterministically
//! refreshes `access_token`, `provider_login`, `user_id`, and `linked_at`.

use std::collections::HashMap;

use serde_json::{json, Value};
use wafer_core::clients::database as db;
use wafer_run::context::Context;

use super::RepoError;

pub const TABLE: &str = "suppers_ai__auth__provider_links";

/// Full row shape returned by [`find_by_provider_ref`]. `linked_at` is
/// included so higher layers can surface "last login via …" strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderLink {
    pub provider: String,
    pub provider_ref: String,
    pub user_id: String,
    pub provider_login: String,
    pub access_token: String,
    pub linked_at: String,
}

/// Insert/update payload for [`upsert`]. All fields are borrowed so the
/// caller avoids allocating clones of handler-owned strings.
#[derive(Debug, Clone, Copy)]
pub struct NewLink<'a> {
    pub provider: &'a str,
    pub provider_ref: &'a str,
    pub user_id: &'a str,
    pub provider_login: &'a str,
    pub access_token: &'a str,
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn row_from_map(m: &HashMap<String, Value>) -> Result<ProviderLink, RepoError> {
    let s = |k: &str| m.get(k).and_then(Value::as_str).map(str::to_owned);
    Ok(ProviderLink {
        provider: s("provider").ok_or_else(|| RepoError::Db("missing provider".into()))?,
        provider_ref: s("provider_ref")
            .ok_or_else(|| RepoError::Db("missing provider_ref".into()))?,
        user_id: s("user_id").ok_or_else(|| RepoError::Db("missing user_id".into()))?,
        provider_login: s("provider_login").unwrap_or_default(),
        access_token: s("access_token").unwrap_or_default(),
        linked_at: s("linked_at").unwrap_or_default(),
    })
}

/// Insert a link row, or update `user_id`, `provider_login`, `access_token`,
/// `linked_at` in place when a row with the same `(provider, provider_ref)`
/// already exists. Single SQL statement per spec §5 (OAuth callback).
pub async fn upsert(ctx: &dyn Context, new: NewLink<'_>) -> Result<(), RepoError> {
    let now = now_iso();
    db::exec_raw(
        ctx,
        &format!(
            "INSERT INTO {TABLE} (provider, provider_ref, user_id, provider_login, access_token, linked_at) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT (provider, provider_ref) DO UPDATE SET \
                 user_id = excluded.user_id, \
                 provider_login = excluded.provider_login, \
                 access_token = excluded.access_token, \
                 linked_at = excluded.linked_at",
        ),
        &[
            json!(new.provider),
            json!(new.provider_ref),
            json!(new.user_id),
            json!(new.provider_login),
            json!(new.access_token),
            json!(now),
        ],
    )
    .await
    .map_err(|e| RepoError::Db(format!("provider_links upsert: {e}")))?;
    Ok(())
}

/// Look up a link by `(provider, provider_ref)`. Returns `Ok(None)` if no
/// matching row exists.
pub async fn find_by_provider_ref(
    ctx: &dyn Context,
    provider: &str,
    provider_ref: &str,
) -> Result<Option<ProviderLink>, RepoError> {
    let rows = db::query_raw(
        ctx,
        &format!("SELECT * FROM {TABLE} WHERE provider = ? AND provider_ref = ?"),
        &[json!(provider), json!(provider_ref)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("provider_links find: {e}")))?;
    match rows.first() {
        Some(r) => Ok(Some(row_from_map(&r.data)?)),
        None => Ok(None),
    }
}

/// Look up the most-recently-linked row for `(user_id, provider)`. A given
/// user can only have at most one link per provider (enforced by the
/// OAuth callback upserting on provider+provider_ref and users being
/// unique per provider account), so there's at most one row in practice;
/// the `ORDER BY linked_at DESC LIMIT 1` just makes that explicit.
///
/// Used by `AuthService::verify_org_admin` to retrieve the access token it
/// needs to call into the provider's org-membership endpoint.
pub async fn find_by_user_provider(
    ctx: &dyn Context,
    user_id: &str,
    provider: &str,
) -> Result<Option<ProviderLink>, RepoError> {
    let rows = db::query_raw(
        ctx,
        &format!(
            "SELECT * FROM {TABLE} WHERE user_id = ? AND provider = ? ORDER BY linked_at DESC LIMIT 1"
        ),
        &[json!(user_id), json!(provider)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("provider_links find_by_user_provider: {e}")))?;
    match rows.first() {
        Some(r) => Ok(Some(row_from_map(&r.data)?)),
        None => Ok(None),
    }
}

/// Return all OAuth provider links owned by `user_id`, ordered by
/// `linked_at` ASC for stable rendering on the security page.
pub async fn list_for_user(
    ctx: &dyn Context,
    user_id: &str,
) -> Result<Vec<ProviderLink>, RepoError> {
    let rows = db::query_raw(
        ctx,
        &format!("SELECT * FROM {TABLE} WHERE user_id = ? ORDER BY linked_at ASC"),
        &[json!(user_id)],
    )
    .await
    .map_err(|e| RepoError::Db(format!("provider_links list_for_user: {e}")))?;
    rows.iter().map(|r| row_from_map(&r.data)).collect()
}

#[cfg(test)]
mod tests_phase_4 {
    use super::*;
    use crate::test_support::TestContext;

    async fn seed_user(ctx: &TestContext, user_id: &str) {
        wafer_core::clients::database::exec_raw(
            ctx,
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

    #[tokio::test]
    async fn list_for_user_returns_only_caller_links() {
        let ctx = TestContext::with_auth().await;
        for u in ["user-a", "user-b"] {
            seed_user(&ctx, u).await;
        }
        upsert(&ctx, NewLink {
            provider: "github", provider_ref: "gh-1", user_id: "user-a",
            provider_login: "alice", access_token: "tok-a",
        }).await.unwrap();
        upsert(&ctx, NewLink {
            provider: "google", provider_ref: "gg-1", user_id: "user-a",
            provider_login: "alice@example.com", access_token: "tok-b",
        }).await.unwrap();
        upsert(&ctx, NewLink {
            provider: "github", provider_ref: "gh-2", user_id: "user-b",
            provider_login: "bob", access_token: "tok-c",
        }).await.unwrap();

        let a = list_for_user(&ctx, "user-a").await.unwrap();
        let providers: Vec<&str> = a.iter().map(|l| l.provider.as_str()).collect();
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&"github"));
        assert!(providers.contains(&"google"));

        let b = list_for_user(&ctx, "user-b").await.unwrap();
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].provider, "github");
        assert_eq!(b[0].provider_login, "bob");

        let c = list_for_user(&ctx, "user-c").await.unwrap();
        assert!(c.is_empty());
    }
}
