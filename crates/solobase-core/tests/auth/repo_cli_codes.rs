//! cli_exchange_codes repo — insert + atomic take.

use solobase_core::blocks::auth::{
    migrations,
    repo::{cli_codes, users},
};

use crate::common::MigrationTestCtx;

async fn mk_user(ctx: &MigrationTestCtx, email: &str) -> String {
    users::insert(
        ctx,
        users::NewUser {
            email: email.into(),
            display_name: email.into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .expect("insert user")
    .id
}

fn hash(n: u8) -> [u8; 32] {
    let mut h = [0u8; 32];
    h[0] = n;
    h
}

fn iso_plus_secs(secs: i64) -> String {
    use chrono::{Duration, Utc};
    (Utc::now() + Duration::seconds(secs))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

#[tokio::test]
async fn insert_then_take_returns_row_and_deletes() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migration apply");
    let uid = mk_user(&ctx, "a@x.com").await;

    let h = hash(1);
    cli_codes::insert(
        &ctx,
        cli_codes::NewCode {
            code_hash: &h,
            user_id: &uid,
            expires_at: &iso_plus_secs(900),
        },
    )
    .await
    .expect("insert");

    let row = cli_codes::take(&ctx, &h)
        .await
        .expect("take")
        .expect("row present");
    assert_eq!(row.user_id, uid);

    // Second take → None (row was deleted by the first call).
    let gone = cli_codes::take(&ctx, &h).await.expect("take-2");
    assert!(gone.is_none(), "row should be single-use");
}

#[tokio::test]
async fn take_unknown_returns_none() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migration apply");
    let row = cli_codes::take(&ctx, &hash(99)).await.expect("take");
    assert!(row.is_none());
}

#[tokio::test]
async fn take_expired_returns_none_and_deletes() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migration apply");
    let uid = mk_user(&ctx, "a@x.com").await;

    let h = hash(2);
    cli_codes::insert(
        &ctx,
        cli_codes::NewCode {
            code_hash: &h,
            user_id: &uid,
            // expired 60 s ago
            expires_at: &iso_plus_secs(-60),
        },
    )
    .await
    .expect("insert");

    let row = cli_codes::take(&ctx, &h).await.expect("take");
    assert!(row.is_none(), "expired row should not be returned");

    // Expired row is also gone from DB (RETURNING fired the delete).
    let again = cli_codes::take(&ctx, &h).await.expect("take-2");
    assert!(again.is_none());
}

#[tokio::test]
async fn delete_expired_drops_only_expired_rows() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migration apply");
    let uid = mk_user(&ctx, "a@x.com").await;
    cli_codes::insert(
        &ctx,
        cli_codes::NewCode {
            code_hash: &hash(3),
            user_id: &uid,
            expires_at: &iso_plus_secs(-60),
        },
    )
    .await
    .expect("insert expired");
    cli_codes::insert(
        &ctx,
        cli_codes::NewCode {
            code_hash: &hash(4),
            user_id: &uid,
            expires_at: &iso_plus_secs(900),
        },
    )
    .await
    .expect("insert fresh");

    let deleted = cli_codes::delete_expired(&ctx, &iso_plus_secs(0))
        .await
        .expect("delete_expired");
    assert_eq!(deleted, 1);

    // Fresh row still present.
    let row = cli_codes::take(&ctx, &hash(4))
        .await
        .expect("take")
        .expect("fresh row still there");
    assert_eq!(row.user_id, uid);
}
