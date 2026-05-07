//! Narrow access over `suppers_ai__auth__bootstrap_tokens`.
//!
//! Only the subset needed by `AuthServiceImpl::require_role` plus a test
//! insert. The full bootstrap-admin lifecycle (issuance, consumption,
//! single-use semantics) lands in Plan A2.

use serde_json::json;
use wafer_core::clients::database as db;
use wafer_run::context::Context;

use super::RepoError;

pub const TABLE: &str = "suppers_ai__auth__bootstrap_tokens";

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Insert a bootstrap token row. Used by Plan A2's bootstrap-admin init;
/// exposed here so the `require_role` integration tests can seed a row
/// directly without re-implementing the SQL.
pub async fn insert(
    ctx: &dyn Context,
    token_hash: Vec<u8>,
    expires_at: &str,
) -> Result<(), RepoError> {
    use std::collections::HashMap;

    use serde_json::Value;
    let id = uuid::Uuid::now_v7().to_string();
    let now = now_iso();
    let hex = hex_encode(&token_hash);
    let mut data: HashMap<String, Value> = HashMap::new();
    data.insert("id".into(), json!(id));
    data.insert("token_hash".into(), json!(hex));
    data.insert("created_at".into(), json!(now));
    data.insert("expires_at".into(), json!(expires_at));

    db::create(ctx, TABLE, data)
        .await
        .map_err(|e| RepoError::Db(format!("bootstrap_tokens insert: {e}")))?;
    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Returns true iff an unexpired row exists with the given hash.
///
/// Compared as ISO-8601 strings to match the text format the migration
/// schema stores.
pub async fn is_valid(ctx: &dyn Context, token_hash: &[u8]) -> Result<bool, RepoError> {
    let now = now_iso();
    let hex = hex_encode(token_hash);
    let opts = db::ListOptions {
        filters: vec![
            db::Filter {
                field: "token_hash".into(),
                operator: db::FilterOp::Equal,
                value: json!(hex),
            },
            db::Filter {
                field: "expires_at".into(),
                operator: db::FilterOp::GreaterEqual,
                value: json!(now),
            },
        ],
        limit: 1,
        ..Default::default()
    };
    let res = db::list(ctx, TABLE, &opts)
        .await
        .map_err(|e| RepoError::Db(format!("bootstrap_tokens lookup: {e}")))?;
    Ok(!res.records.is_empty())
}

#[cfg(test)]
mod typed_client_tests {
    use super::*;
    use crate::test_support::TestContext;

    fn future_iso(secs: i64) -> String {
        (chrono::Utc::now() + chrono::Duration::seconds(secs))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string()
    }

    fn past_iso(secs: i64) -> String {
        (chrono::Utc::now() - chrono::Duration::seconds(secs))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string()
    }

    #[tokio::test]
    async fn insert_then_validate_round_trips_under_wrap() {
        let ctx =
            TestContext::with_auth()
                .await
                .with_wrap("suppers-ai/auth", vec![], "suppers-ai/admin");
        let hash = vec![0xab_u8; 32];
        insert(&ctx, hash.clone(), &future_iso(3600)).await.unwrap();
        assert!(is_valid(&ctx, &hash).await.unwrap());
    }

    #[tokio::test]
    async fn unknown_hash_is_invalid() {
        let ctx =
            TestContext::with_auth()
                .await
                .with_wrap("suppers-ai/auth", vec![], "suppers-ai/admin");
        let hash = vec![0xcd_u8; 32];
        assert!(!is_valid(&ctx, &hash).await.unwrap());
    }

    #[tokio::test]
    async fn expired_hash_is_invalid() {
        let ctx =
            TestContext::with_auth()
                .await
                .with_wrap("suppers-ai/auth", vec![], "suppers-ai/admin");
        let hash = vec![0xef_u8; 32];
        insert(&ctx, hash.clone(), &past_iso(3600)).await.unwrap();
        assert!(!is_valid(&ctx, &hash).await.unwrap());
    }
}
