//! OAuth callback "resolve user" rule per auth-block-design spec §5.
//!
//! Given a normalised [`ProviderProfile`] from a provider's `exchange_code`,
//! decide which local user the callback should attach the session to:
//!
//! 1. If a `provider_links` row already exists for `(provider, provider_ref)`,
//!    reuse its `user_id` — even if the email on the profile has since
//!    changed upstream.
//! 2. Otherwise, require a verified email. If the profile has one and it
//!    matches an existing `users.email` row, link the new identity to that
//!    user.
//! 3. Otherwise, with a verified email and no match, create a brand-new user
//!    row (uuid v7 id) and return its id.
//! 4. Otherwise — missing or unverified email — refuse with [`AuthError::Forbidden`].
//!
//! The caller is responsible for invoking `provider_links::upsert` with the
//! returned `user_id`; this function only picks WHICH user to attach.

use wafer_core::interfaces::auth::service::AuthError;
use wafer_run::context::Context;

use crate::blocks::auth::{
    providers::ProviderProfile,
    repo::{provider_links, users},
};

/// Outcome of [`resolve_user_for_profile`].
///
/// Three success paths (all carry the resolved `user_id`) corresponding to
/// spec §5 rules 1–3. The [`AuthError::Forbidden`] path is surfaced as an
/// `Err`, not as a variant here.
#[derive(Debug)]
pub enum ResolveOutcome {
    /// An existing `provider_links` row already points at this user.
    Existing(String),
    /// No link existed; matched an existing user by verified email and
    /// will now be linked.
    LinkedToExisting(String),
    /// No link existed and no email match; a fresh user row was created.
    Created(String),
}

/// Spec §5 OAuth callback "resolve user" rule.
///
/// See the module docstring for the branch order. `AuthError::Forbidden` is
/// returned both for missing emails and for present-but-unverified emails,
/// matching the spec's "verified email" gating. All other failures surface
/// as `AuthError::Internal`.
pub async fn resolve_user_for_profile(
    ctx: &dyn Context,
    provider: &str,
    profile: &ProviderProfile,
) -> Result<ResolveOutcome, AuthError> {
    // 1. Existing link?
    if let Some(link) = provider_links::find_by_provider_ref(ctx, provider, &profile.provider_ref)
        .await
        .map_err(|e| AuthError::Internal(format!("provider_links find: {e}")))?
    {
        return Ok(ResolveOutcome::Existing(link.user_id));
    }

    // 2. Need a verified email to link or create.
    let Some(email) = profile.email.as_deref() else {
        return Err(AuthError::Forbidden);
    };
    if !profile.email_verified {
        return Err(AuthError::Forbidden);
    }

    // 3. Match by email?
    if let Some(user) = users::find_by_email(ctx, email)
        .await
        .map_err(|e| AuthError::Internal(format!("users find_by_email: {e}")))?
    {
        return Ok(ResolveOutcome::LinkedToExisting(user.id));
    }

    // 4. Create new user.
    let new_user = users::NewUser {
        email: email.to_string(),
        display_name: profile.display_name.clone(),
        avatar_url: profile.avatar_url.clone(),
        role: "user".into(),
    };
    let created = users::insert(ctx, new_user)
        .await
        .map_err(|e| AuthError::Internal(format!("users insert: {e}")))?;
    Ok(ResolveOutcome::Created(created.id))
}
