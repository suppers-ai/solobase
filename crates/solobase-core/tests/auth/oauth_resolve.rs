//! Callback "resolve user" rule — spec §5.
//!
//! Four branches, one test each:
//!   1. Existing link → reuse, even if email has changed upstream.
//!   2. No link, verified email matches a `users` row → link to it.
//!   3. No link, verified email, no match → create a new user.
//!   4. Missing or unverified email → `AuthError::Forbidden`.

use solobase_core::blocks::auth::{
    handlers::oauth_resolve::{resolve_user_for_profile, ResolveOutcome},
    migrations,
    providers::ProviderProfile,
    repo::{provider_links, users},
};
use wafer_core::interfaces::auth::service::AuthError;

use crate::common::MigrationTestCtx;

fn profile(provider_ref: &str, email: Option<&str>, email_verified: bool) -> ProviderProfile {
    ProviderProfile {
        provider_ref: provider_ref.into(),
        login: "alice".into(),
        email: email.map(str::to_string),
        email_verified,
        display_name: "Alice".into(),
        avatar_url: None,
        access_token: "tok".into(),
    }
}

async fn seed_user(ctx: &MigrationTestCtx, email: &str) -> String {
    users::insert(
        ctx,
        users::NewUser {
            email: email.into(),
            display_name: "Seed".into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .expect("insert user")
    .id
}

#[tokio::test]
async fn existing_link_returns_that_user_even_if_email_changed() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");
    let uid = seed_user(&ctx, "alice@old.example").await;
    provider_links::upsert(
        &ctx,
        provider_links::NewLink {
            provider: "github",
            provider_ref: "42",
            user_id: &uid,
            provider_login: "alice",
            access_token: "tok",
        },
    )
    .await
    .expect("upsert link");

    let outcome = resolve_user_for_profile(
        &ctx,
        "github",
        &profile("42", Some("alice@new.example"), true),
    )
    .await
    .expect("resolve ok");
    match outcome {
        ResolveOutcome::Existing(got) => assert_eq!(got, uid),
        other => panic!("expected Existing, got {other:?}"),
    }
}

#[tokio::test]
async fn no_link_verified_email_matches_existing_user_links_to_it() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");
    let uid = seed_user(&ctx, "alice@example.com").await;

    let outcome = resolve_user_for_profile(
        &ctx,
        "github",
        &profile("42", Some("alice@example.com"), true),
    )
    .await
    .expect("resolve ok");
    match outcome {
        ResolveOutcome::LinkedToExisting(got) => assert_eq!(got, uid),
        other => panic!("expected LinkedToExisting, got {other:?}"),
    }
}

#[tokio::test]
async fn no_link_verified_email_no_match_creates_new_user() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");

    let outcome = resolve_user_for_profile(
        &ctx,
        "github",
        &profile("99", Some("new@example.com"), true),
    )
    .await
    .expect("resolve ok");
    match outcome {
        ResolveOutcome::Created(uid) => {
            let row = users::find_by_id(&ctx, &uid)
                .await
                .expect("find")
                .expect("row");
            assert_eq!(row.email, "new@example.com");
            assert_eq!(row.display_name, "Alice");
        }
        other => panic!("expected Created, got {other:?}"),
    }
}

#[tokio::test]
async fn unverified_email_is_forbidden() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");
    let err = resolve_user_for_profile(
        &ctx,
        "github",
        &profile("99", Some("shady@example.com"), false),
    )
    .await
    .expect_err("must be forbidden");
    assert!(matches!(err, AuthError::Forbidden));
}

#[tokio::test]
async fn missing_email_is_forbidden() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");
    let err = resolve_user_for_profile(&ctx, "github", &profile("99", None, true))
        .await
        .expect_err("must be forbidden");
    assert!(matches!(err, AuthError::Forbidden));
}
