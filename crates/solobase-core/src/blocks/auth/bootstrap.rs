//! First-run bootstrap for the `suppers-ai/auth` block.
//!
//! On `Init`, if the `suppers_ai__auth__users` table is empty, [`run`] picks
//! one of three paths based on which bootstrap env vars are set:
//!
//! 1. `BOOTSTRAP_ADMIN_EMAIL` + `BOOTSTRAP_ADMIN_PASSWORD` — hash the password
//!    via `wafer-run/crypto` and create an admin user + `local_credentials`.
//! 2. `BOOTSTRAP_ADMIN_TOKEN` — store sha256(token) in `bootstrap_tokens`
//!    with a 24h expiry. The holder later redeems it by presenting the raw
//!    token as a `Bearer` header (see `AuthServiceImpl::require_role`).
//! 3. None set — log and do nothing; leaves the operator to provision via UI
//!    / CLI.
//!
//! If the `users` table is non-empty, [`run`] is a no-op regardless of env —
//! bootstrap is a first-run mechanism only, never a "re-seed" trigger.
//!
//! ## Transitional dual-write (PR 1+2 → PR 3)
//!
//! Until PR 3 migrates `login.rs`/`oauth.rs` to read credentials from
//! `repo::local_credentials` and roles from the new schema, the legacy login
//! handler still expects:
//! - `users.password_hash` (legacy column on the same table)
//! - `users.name`, `users.disabled`, `users.email_verified` (legacy fields)
//! - a row in `suppers_ai__admin__user_roles` granting `admin`
//!
//! The Plan A2 migration creates `users` with the new schema only
//! (no `password_hash`). To keep the existing login path working during the
//! transition, the email+password bootstrap inserts the user row through
//! `db::create()` — which auto-`ALTER`s the table to add any missing legacy
//! columns via `ensure_table` — with BOTH the new (`display_name`, `role`,
//! `email_verified` boolean) and legacy (`password_hash`, `name`, `disabled`)
//! fields populated. It then writes the matching `local_credentials` row for
//! the new schema and a role row in `suppers_ai__admin__user_roles` for the
//! legacy code path.
//!
//! These dual writes are removed in PR 3 once login/oauth read from the new
//! schema.

use std::collections::HashMap;

use sha2::{Digest, Sha256};
use uuid::Uuid;
use wafer_core::clients::{crypto, database as db};
use wafer_run::{
    context::Context,
    types::{ErrorCode, WaferError},
};

use super::{
    config::AuthConfig,
    repo::{bootstrap_tokens, local_credentials, users},
};

/// SHA-256 of `s`. Shared with bootstrap-token verification in
/// `require_role`: the raw token is never stored.
pub fn sha256(s: &str) -> Vec<u8> {
    Sha256::digest(s.as_bytes()).to_vec()
}

/// Run the bootstrap step. Idempotent: returns `Ok(())` without side-effects
/// when the `users` table is already populated.
pub async fn run(ctx: &dyn Context, cfg: &AuthConfig) -> Result<(), WaferError> {
    let user_count = users::count(ctx).await.map_err(internal)?;
    if user_count > 0 {
        tracing::debug!("auth: bootstrap skipped, users table already has {user_count} row(s)");
        return Ok(());
    }

    match (
        cfg.bootstrap_admin_email.as_deref(),
        cfg.bootstrap_admin_password.as_deref(),
        cfg.bootstrap_admin_token.as_deref(),
    ) {
        (Some(email), Some(password), _) => {
            bootstrap_with_email_password(ctx, email, password).await?;
            tracing::info!("auth: bootstrapped admin user: {email}");
        }
        (_, _, Some(token)) => {
            bootstrap_with_token(ctx, token).await?;
            tracing::info!("auth: bootstrap token installed; expires in 24h");
        }
        _ => {
            tracing::info!("auth: no bootstrap admin configured (users table empty)");
        }
    }
    Ok(())
}

async fn bootstrap_with_email_password(
    ctx: &dyn Context,
    email: &str,
    password: &str,
) -> Result<(), WaferError> {
    let hash = crypto::hash(ctx, password).await?;
    let id = Uuid::now_v7().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // Insert the user row through `db::create()` so `ensure_table` ALTERs in
    // any missing legacy columns (transitional — see module doc). Populating
    // both the new (`display_name`, `role`, `email_verified` as a bool) and
    // legacy (`name`, `password_hash`, `disabled`, `email_verified` as the
    // string "true") fields keeps the new repo and the legacy login.rs both
    // working until PR 3 retires the legacy reads.
    let mut data: HashMap<String, serde_json::Value> = HashMap::new();
    data.insert("id".to_string(), serde_json::json!(id));
    data.insert("email".to_string(), serde_json::json!(email));
    // New schema (Plan A2)
    data.insert("display_name".to_string(), serde_json::json!("Admin"));
    data.insert("role".to_string(), serde_json::json!("admin"));
    data.insert("email_verified".to_string(), serde_json::json!(true));
    data.insert("created_at".to_string(), serde_json::json!(now.clone()));
    data.insert("updated_at".to_string(), serde_json::json!(now.clone()));
    // Legacy compat (removed in PR 3 once login.rs migrates to repo::*)
    data.insert("name".to_string(), serde_json::json!("Admin"));
    data.insert("password_hash".to_string(), serde_json::json!(hash.clone()));
    data.insert("disabled".to_string(), serde_json::json!(false));
    db::create(ctx, users::TABLE, data)
        .await
        .map_err(|e| internal(format!("insert bootstrap admin: {e}")))?;

    // New-schema local_credentials row.
    local_credentials::insert(ctx, &id, &hash, false)
        .await
        .map_err(internal)?;

    // Legacy role row in `suppers_ai__admin__user_roles` so the legacy
    // login.rs/oauth.rs `get_user_roles` lookup returns ["admin"]. Removed in
    // PR 3 once those handlers read role from `users.role`.
    let role_data: HashMap<String, serde_json::Value> = HashMap::from_iter([
        ("user_id".to_string(), serde_json::json!(id)),
        ("role".to_string(), serde_json::json!("admin")),
        ("assigned_at".to_string(), serde_json::json!(now)),
    ]);
    db::create(ctx, crate::blocks::admin::USER_ROLES_COLLECTION, role_data)
        .await
        .map_err(|e| internal(format!("insert bootstrap admin role: {e}")))?;

    Ok(())
}

async fn bootstrap_with_token(ctx: &dyn Context, token: &str) -> Result<(), WaferError> {
    let expires = chrono::Utc::now() + chrono::Duration::hours(24);
    let expires_iso = expires.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    bootstrap_tokens::insert(ctx, sha256(token), &expires_iso)
        .await
        .map_err(internal)?;
    Ok(())
}

fn internal<E: std::fmt::Display>(e: E) -> WaferError {
    WaferError::new(ErrorCode::INTERNAL, format!("auth bootstrap: {e}"))
}
