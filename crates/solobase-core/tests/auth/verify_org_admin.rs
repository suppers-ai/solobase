//! `AuthService::verify_org_admin` — covers the surviving dispatch matrix
//! after the legacy `OAuthProvider` registry was removed.
//!
//! Cases covered:
//!   1. Reserved org + admin user                → true
//!   2. Reserved org + plain user                → false
//!   3. Non-reserved org owner (provider match)  → true
//!   4. Non-reserved org non-owner (provider match) → false
//!   5. Non-matching provider on claimed org     → false
//!   6. Unknown org                              → false
//!
//! Without an `OAuthProvider` registry there is no upstream membership call,
//! so non-owner members of a claimed org always return false. Multi-admin
//! orgs need to either grow real "members" rows in the DB or grant the
//! site-admin role.

use std::sync::Arc;

use solobase_core::blocks::auth::{
    cache::OrgAdminCache,
    migrations,
    repo::{
        orgs::{self, NewClaim},
        users,
    },
    service::{AuthServiceImpl, BlockState},
};
use wafer_core::interfaces::auth::service::{AuthService, UserId};
use wafer_run::context::Context;

use crate::common::MigrationTestCtx;

struct Harness {
    ctx: Arc<dyn Context>,
    svc: AuthServiceImpl,
}

async fn boot() -> Harness {
    let ctx: Arc<dyn Context> = Arc::new(MigrationTestCtx::new());
    migrations::apply(ctx.as_ref()).await.expect("migrations");
    let state = BlockState::for_test(ctx.clone()).with_org_admin_cache(OrgAdminCache::default());
    Harness {
        ctx,
        svc: AuthServiceImpl::new(state),
    }
}

async fn mk_user(ctx: &dyn Context, email: &str, role: &str) -> UserId {
    let row = users::insert(
        ctx,
        users::NewUser {
            email: email.into(),
            display_name: email.into(),
            avatar_url: None,
            role: role.into(),
        },
    )
    .await
    .expect("user insert");
    UserId(row.id)
}

#[tokio::test]
async fn reserved_org_admin_user_is_admin() {
    let h = boot().await;
    let admin = mk_user(h.ctx.as_ref(), "admin@example.com", "admin").await;
    let ok = h
        .svc
        .verify_org_admin(admin, "github", "wafer-run")
        .await
        .unwrap();
    assert!(ok);
}

#[tokio::test]
async fn reserved_org_plain_user_is_not_admin() {
    let h = boot().await;
    let user = mk_user(h.ctx.as_ref(), "user@example.com", "user").await;
    let ok = h
        .svc
        .verify_org_admin(user, "github", "wafer-run")
        .await
        .unwrap();
    assert!(!ok);
}

#[tokio::test]
async fn claimed_org_owner_is_admin() {
    let h = boot().await;
    let owner = mk_user(h.ctx.as_ref(), "owner@example.com", "user").await;
    orgs::upsert_claimed(
        h.ctx.as_ref(),
        NewClaim {
            name: "acme",
            owner_user_id: &owner.0,
            verified_via: "github",
            verified_ref: "acme-org",
        },
    )
    .await
    .unwrap();
    let ok = h
        .svc
        .verify_org_admin(owner, "github", "acme")
        .await
        .unwrap();
    assert!(ok);
}

#[tokio::test]
async fn claimed_org_non_owner_is_not_admin() {
    let h = boot().await;
    let owner = mk_user(h.ctx.as_ref(), "owner@example.com", "user").await;
    let other = mk_user(h.ctx.as_ref(), "other@example.com", "user").await;
    orgs::upsert_claimed(
        h.ctx.as_ref(),
        NewClaim {
            name: "acme",
            owner_user_id: &owner.0,
            verified_via: "github",
            verified_ref: "acme-org",
        },
    )
    .await
    .unwrap();
    let ok = h
        .svc
        .verify_org_admin(other, "github", "acme")
        .await
        .unwrap();
    assert!(!ok);
}

#[tokio::test]
async fn provider_mismatch_on_claimed_org_is_not_admin() {
    let h = boot().await;
    let owner = mk_user(h.ctx.as_ref(), "owner@example.com", "user").await;
    orgs::upsert_claimed(
        h.ctx.as_ref(),
        NewClaim {
            name: "acme",
            owner_user_id: &owner.0,
            verified_via: "github",
            verified_ref: "acme-org",
        },
    )
    .await
    .unwrap();
    let ok = h
        .svc
        .verify_org_admin(owner, "google", "acme")
        .await
        .unwrap();
    assert!(!ok);
}

#[tokio::test]
async fn unknown_org_is_not_admin() {
    let h = boot().await;
    let user = mk_user(h.ctx.as_ref(), "user@example.com", "user").await;
    let ok = h
        .svc
        .verify_org_admin(user, "github", "never-claimed")
        .await
        .unwrap();
    assert!(!ok);
}
