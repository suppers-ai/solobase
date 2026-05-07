//! PR 1+2: AuthBlock lifecycle(Init) applies migrations + runs bootstrap.
//!
//! The two changes ship together because PR 1 (migrations alone) is unsafe:
//! migrations create the Plan A2 `users` schema (no `password_hash` column),
//! but the legacy `seed_admin_user` writes a row with `password_hash`. The
//! schema conflict broke the admin login row. This PR retires
//! `seed_admin_user` and wires `bootstrap::run` (which writes the new schema
//! plus transitional legacy compat fields) on the same Init event.
//!
//! Coverage:
//! - Migrations create `local_credentials`/`bootstrap_tokens` (Plan A2 tables
//!   that did not exist before this PR).
//! - With env vars set, bootstrap seeds an admin user with a populated
//!   `local_credentials` row + `display_name`/`role=admin` on `users`.

use std::sync::Arc;

use solobase_core::blocks::auth::{
    config::AuthConfig,
    repo::{local_credentials, users},
    AuthBlock,
};
use wafer_block::core_types::{LifecycleEvent, LifecycleType};
use wafer_core::clients::database as db;
use wafer_run::block::Block;

use crate::common::MigrationTestCtx;

/// Drive `AuthBlock::lifecycle(Init)` against a freshly-built test context.
async fn run_init(ctx: &MigrationTestCtx) {
    let block = Arc::new(AuthBlock::new());
    let event = LifecycleEvent {
        event_type: LifecycleType::Init,
        data: Vec::new(),
    };
    block
        .lifecycle(ctx, event)
        .await
        .expect("lifecycle Init should succeed");
}

#[tokio::test]
async fn init_creates_local_credentials_and_bootstrap_tokens_tables() {
    let ctx = MigrationTestCtx::new();

    // Sanity: tables don't exist before lifecycle.
    let pre = db::query_raw(
        &ctx,
        "SELECT name FROM sqlite_master WHERE type='table' AND name='suppers_ai__auth__local_credentials'",
        &[],
    )
    .await
    .expect("query sqlite_master");
    assert!(
        pre.is_empty(),
        "local_credentials table must not exist pre-Init"
    );

    run_init(&ctx).await;

    // Migrations created the tables — querying them via `db::query_raw`
    // succeeds (would error with "no such table" otherwise).
    let lc = db::query_raw(
        &ctx,
        "SELECT COUNT(*) AS n FROM suppers_ai__auth__local_credentials",
        &[],
    )
    .await;
    assert!(lc.is_ok(), "local_credentials missing: {:?}", lc);

    let bt = db::query_raw(
        &ctx,
        "SELECT COUNT(*) AS n FROM suppers_ai__auth__bootstrap_tokens",
        &[],
    )
    .await;
    assert!(bt.is_ok(), "bootstrap_tokens missing: {:?}", bt);
}

#[tokio::test]
async fn init_with_no_bootstrap_env_seeds_no_admin() {
    // No env vars → bootstrap is a no-op even after migrations.
    let ctx = MigrationTestCtx::new();
    run_init(&ctx).await;

    assert_eq!(
        users::count(&ctx).await.expect("count"),
        0,
        "no env → no seeded admin"
    );
}

#[tokio::test]
async fn bootstrap_seeds_admin_dual_writes_legacy_and_new_schema() {
    // This test bypasses lifecycle's `AuthConfig::from_ctx` (which would
    // require a config block in the test ctx) and calls bootstrap directly
    // with an explicit AuthConfig — same code path the lifecycle takes after
    // config loading.
    let ctx = MigrationTestCtx::new();
    solobase_core::blocks::auth::migrations::apply(&ctx)
        .await
        .expect("migrations");

    let cfg = AuthConfig::from_env_for_test(&[
        (
            "SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_EMAIL",
            "boot@example.com",
        ),
        (
            "SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_PASSWORD",
            "bootpass123",
        ),
    ]);
    solobase_core::blocks::auth::bootstrap::run(&ctx, &cfg)
        .await
        .expect("bootstrap run");

    // New-schema repo lookup sees the row.
    let user = users::find_by_email(&ctx, "boot@example.com")
        .await
        .expect("repo call")
        .expect("admin user inserted");
    assert_eq!(user.role, "admin");
    assert_eq!(user.display_name, "Admin");

    // local_credentials row populated for new login path.
    let cred = local_credentials::find_by_user_id(&ctx, &user.id)
        .await
        .expect("repo call")
        .expect("local_credentials inserted");
    assert!(!cred.password_hash.is_empty());

    // Legacy compat fields written via dual-write so login.rs (which reads
    // `password_hash` directly off `users`) keeps working until PR 3.
    let legacy = db::query_raw(
        &ctx,
        "SELECT name, password_hash, disabled FROM suppers_ai__auth__users WHERE id = ?",
        &[serde_json::json!(user.id)],
    )
    .await
    .expect("legacy field query")
    .into_iter()
    .next()
    .expect("user row");
    assert_eq!(
        legacy.data.get("name").and_then(|v| v.as_str()),
        Some("Admin"),
        "legacy name must be set for login.rs"
    );
    assert!(
        legacy
            .data
            .get("password_hash")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "legacy password_hash must be set"
    );

    // Legacy admin role row in `suppers_ai__admin__user_roles` for the
    // `helpers::get_user_roles` lookup login.rs uses.
    let role_rows = db::query_raw(
        &ctx,
        "SELECT role FROM suppers_ai__admin__user_roles WHERE user_id = ?",
        &[serde_json::json!(user.id)],
    )
    .await
    .expect("role query");
    assert!(
        role_rows
            .iter()
            .any(|r| r.data.get("role").and_then(|v| v.as_str()) == Some("admin")),
        "must seed legacy admin role row"
    );
}
