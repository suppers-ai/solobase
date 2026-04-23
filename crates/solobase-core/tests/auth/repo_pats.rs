//! Personal access tokens repo — insert / find / touch against in-memory
//! SQLite after applying migration 001.

use solobase_core::blocks::auth::{
    migrations,
    repo::{pats, users},
};

use crate::common::MigrationTestCtx;

#[tokio::test]
async fn pat_insert_find_with_scopes() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migration apply");

    let u = users::insert(
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

    let hash = [3u8; 32];
    pats::insert(
        &ctx,
        pats::NewPat {
            token_hash: hash.to_vec(),
            user_id: u.id.clone(),
            name: "ci".into(),
            scopes: vec!["publish".into()],
            expires_at: None,
        },
    )
    .await
    .expect("insert pat");

    let found = pats::find_by_token_hash(&ctx, &hash)
        .await
        .expect("find pat")
        .expect("pat present");
    assert_eq!(found.user_id, u.id);
    assert_eq!(found.name, "ci");
    assert_eq!(found.scopes, vec!["publish".to_string()]);
    assert_eq!(found.token_hash, hash.to_vec());
    assert!(found.expires_at.is_none());
    assert!(found.last_used_at.is_none());

    pats::touch_last_used(&ctx, &hash).await.expect("touch pat");
    let after_touch = pats::find_by_token_hash(&ctx, &hash)
        .await
        .expect("find after touch")
        .expect("still present");
    assert!(after_touch.last_used_at.is_some());
}

#[tokio::test]
async fn pat_insert_with_multi_scope_and_expiry() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migration apply");

    let u = users::insert(
        &ctx,
        users::NewUser {
            email: "m@example.com".into(),
            display_name: "M".into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .expect("seed user");

    let hash = [4u8; 32];
    pats::insert(
        &ctx,
        pats::NewPat {
            token_hash: hash.to_vec(),
            user_id: u.id.clone(),
            name: "release".into(),
            scopes: vec!["publish".into(), "read".into()],
            expires_at: Some("2099-12-31T23:59:59Z".into()),
        },
    )
    .await
    .expect("insert pat");

    let found = pats::find_by_token_hash(&ctx, &hash)
        .await
        .expect("find pat")
        .expect("pat present");
    assert_eq!(
        found.scopes,
        vec!["publish".to_string(), "read".to_string()]
    );
    assert_eq!(found.expires_at.as_deref(), Some("2099-12-31T23:59:59Z"));
}

#[tokio::test]
async fn pat_find_missing_returns_none() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migration apply");

    let none_hash = [0u8; 32];
    let hit = pats::find_by_token_hash(&ctx, &none_hash)
        .await
        .expect("lookup");
    assert!(hit.is_none());
}
