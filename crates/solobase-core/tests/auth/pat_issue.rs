//! `pat::issue` — generates a prefixed random PAT, hashes it, and inserts a
//! row into `suppers_ai__auth__personal_access_tokens`.

use solobase_core::blocks::auth::{
    migrations, pat,
    repo::{pats, users},
    service::hash_token,
};
use wafer_core::interfaces::auth::service::TokenScope;

use crate::common::MigrationTestCtx;

#[tokio::test]
async fn issue_inserts_row_and_token_is_prefixed() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");

    let user = users::insert(
        &ctx,
        users::NewUser {
            email: "p@example.com".into(),
            display_name: "P".into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .expect("seed user");

    let issued = pat::issue(&ctx, &user.id, "cli", &[TokenScope::Publish], None)
        .await
        .expect("issue");

    assert!(
        issued.raw_token.starts_with("wafer_pat_"),
        "raw token must be prefixed, got {}",
        issued.raw_token
    );
    assert!(issued.expires_at.is_none());

    let row = pats::find_by_token_hash(&ctx, &hash_token(&issued.raw_token))
        .await
        .expect("find pat")
        .expect("pat row present");
    assert_eq!(row.user_id, user.id);
    assert_eq!(row.name, "cli");
    assert_eq!(row.scopes, vec!["publish".to_string()]);
    assert!(row.expires_at.is_none());
}

#[tokio::test]
async fn issue_with_expiry_stores_expires_at() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");

    let user = users::insert(
        &ctx,
        users::NewUser {
            email: "q@example.com".into(),
            display_name: "Q".into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .expect("seed user");

    let exp = chrono::Utc::now() + chrono::Duration::days(7);
    let issued = pat::issue(&ctx, &user.id, "deploy", &[TokenScope::Publish], Some(exp))
        .await
        .expect("issue");

    let row = pats::find_by_token_hash(&ctx, &hash_token(&issued.raw_token))
        .await
        .expect("find")
        .expect("row present");
    assert!(row.expires_at.is_some(), "expires_at should be persisted");
    assert_eq!(issued.expires_at.unwrap().timestamp(), exp.timestamp());
}

#[tokio::test]
async fn two_issues_produce_distinct_tokens() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");

    let user = users::insert(
        &ctx,
        users::NewUser {
            email: "r@example.com".into(),
            display_name: "R".into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .expect("seed user");

    let a = pat::issue(&ctx, &user.id, "a", &[TokenScope::Publish], None)
        .await
        .expect("a");
    let b = pat::issue(&ctx, &user.id, "b", &[TokenScope::Publish], None)
        .await
        .expect("b");
    assert_ne!(a.raw_token, b.raw_token);
}
