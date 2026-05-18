//! `session::issue_for` — generates a prefixed random token, hashes it, and
//! inserts a row into `suppers_ai__auth__sessions`.

use solobase_core::blocks::auth::{
    migrations,
    repo::{sessions, users},
    service::hash_token,
    session,
};

use crate::common::MigrationTestCtx;

#[tokio::test]
async fn issue_for_inserts_row_and_returns_prefixed_token() {
    let ctx = MigrationTestCtx::new().await;
    migrations::apply(&ctx).await.expect("migrations");

    let user = users::insert(
        &ctx,
        users::NewUser {
            email: "s@example.com".into(),
            display_name: "S".into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .expect("seed user");

    let issued = session::issue_for(&ctx, &user.id, 30)
        .await
        .expect("issue_for");

    assert!(
        issued.raw_token.starts_with("wafer_session_"),
        "raw token must be prefixed for operator grep-ability, got {}",
        issued.raw_token
    );

    let row = sessions::find_by_token_hash(&ctx, &hash_token(&issued.raw_token))
        .await
        .expect("find session")
        .expect("session row present");
    assert_eq!(row.user_id, user.id);

    // Expiry is approximately 30 days from now — allow a generous window
    // to accommodate wall-clock jitter between the helper and the assertion.
    let now = chrono::Utc::now();
    let delta = issued.expires_at - now;
    assert!(
        delta.num_days() >= 29 && delta.num_days() <= 30,
        "expected ~30 day lifetime, got {} days",
        delta.num_days()
    );
}

#[tokio::test]
async fn two_issues_produce_distinct_tokens() {
    let ctx = MigrationTestCtx::new().await;
    migrations::apply(&ctx).await.expect("migrations");

    let user = users::insert(
        &ctx,
        users::NewUser {
            email: "t@example.com".into(),
            display_name: "T".into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .expect("seed user");

    let a = session::issue_for(&ctx, &user.id, 7).await.expect("a");
    let b = session::issue_for(&ctx, &user.id, 7).await.expect("b");
    assert_ne!(
        a.raw_token, b.raw_token,
        "random bytes must differ across issues"
    );
}
