//! GET `/auth/oauth/{provider}/start`
//! GET `/auth/oauth/{provider}/callback`
//!
//! Dispatch is performed in `blocks/auth/block.rs` — it extracts the
//! `{provider}` path segment and passes it to the two async handlers here
//! as a `&str`. The handlers return [`HttpReply`] values converted to
//! `OutputStream` by the `From` impl in `handlers::mod`.

use base64ct::{Base64UrlUnpadded, Encoding};
use serde_json::json;
use wafer_core::interfaces::auth::service::AuthError;
use wafer_run::{
    context::Context,
    types::{ErrorCode, WaferError},
};

use super::oauth_resolve::{resolve_user_for_profile, ResolveOutcome};
use super::oauth_state::{
    clear_cookie_header, parse_cookie_value, set_cookie_header, StatePayload,
};
use super::HttpReply;
use crate::blocks::auth::config::AuthConfig;
use crate::blocks::auth::providers::{pkce, registry::ProviderRegistry};
use crate::blocks::auth::repo::provider_links;
use crate::blocks::auth::session;

/// Generates a fresh 32-byte base64url state (43 chars, unpadded — within
/// the OAuth 2.0 `state` parameter's allowed alphabet).
pub fn new_state_value() -> String {
    let mut b = [0u8; 32];
    getrandom::getrandom(&mut b).expect("OS CSPRNG");
    Base64UrlUnpadded::encode_string(&b)
}

/// Sanitize the `next` query param.
///
/// Only single-slash same-origin relative paths are allowed; anything that
/// could open-redirect (schemed URL, protocol-relative `//host/…`) falls
/// back to the dashboard default.
pub fn safe_next_path(next: Option<&str>) -> String {
    match next {
        Some(s) if s.starts_with('/') && !s.starts_with("//") && !s.contains("://") => {
            s.to_string()
        }
        _ => "/auth/dashboard".to_string(),
    }
}

fn err_reply(status: u16, msg: &str, clear_state: bool) -> HttpReply {
    let mut reply = HttpReply::new(status).json_body(&json!({ "error": msg }));
    if clear_state {
        reply = reply.header("Set-Cookie", clear_cookie_header());
    }
    reply
}

/// Build the session `Set-Cookie` for a freshly-issued session.
///
/// Matches the shape emitted by `handlers::login::post_login` so browsers
/// store the same cookie regardless of how the session was issued.
fn session_cookie(raw_token: &str, lifetime_days: u32) -> String {
    let max_age = (lifetime_days as u64) * 24 * 60 * 60;
    format!(
        "{name}={raw}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age={max_age}",
        name = session::COOKIE_NAME,
        raw = raw_token,
    )
}

/// GET `/auth/oauth/{provider}/start` — redirect the browser to the
/// provider's authorize URL after stashing state + PKCE verifier in the
/// `wafer_oauth_state` cookie.
pub async fn get_start(
    registry: &ProviderRegistry,
    provider_name: &str,
    next: Option<&str>,
) -> Result<HttpReply, WaferError> {
    let Some(provider) = registry.get(provider_name) else {
        return Ok(err_reply(404, "unknown_provider", false));
    };
    let verifier = pkce::new_verifier();
    let challenge = pkce::challenge_for(&verifier);
    let state = new_state_value();
    let payload = StatePayload {
        state: state.clone(),
        pkce_verifier: verifier,
        next: Some(safe_next_path(next)),
    };
    let cookie = set_cookie_header(&payload)
        .map_err(|e| WaferError::new(ErrorCode::INTERNAL, format!("state cookie encode: {e}")))?;
    Ok(HttpReply::new(302)
        .header("Location", provider.authorize_url(&state, &challenge))
        .header("Set-Cookie", cookie))
}

/// GET `/auth/oauth/{provider}/callback` — finish the dance: validate
/// state, exchange `code`, resolve the user, upsert `provider_links`,
/// issue a session, and 303 to the stashed `next`.
pub async fn get_callback(
    ctx: &dyn Context,
    cfg: &AuthConfig,
    registry: &ProviderRegistry,
    provider_name: &str,
    code: &str,
    state: &str,
    state_cookie_raw: &str,
) -> Result<HttpReply, WaferError> {
    let Some(provider) = registry.get(provider_name) else {
        return Ok(err_reply(404, "unknown_provider", false));
    };
    if code.is_empty() || state.is_empty() {
        return Ok(err_reply(400, "missing_code_or_state", true));
    }
    if state_cookie_raw.is_empty() {
        return Ok(err_reply(400, "missing_state_cookie", true));
    }
    let payload = match parse_cookie_value(state_cookie_raw) {
        Ok(p) => p,
        Err(_) => return Ok(err_reply(400, "bad_state_cookie", true)),
    };
    if payload.state != state {
        return Ok(err_reply(400, "state_mismatch", true));
    }

    let profile = match provider.exchange_code(code, &payload.pkce_verifier).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, provider = provider_name, "provider exchange_code failed");
            return Ok(err_reply(503, "provider_unavailable", true));
        }
    };

    let outcome = match resolve_user_for_profile(ctx, provider_name, &profile).await {
        Ok(o) => o,
        Err(AuthError::Forbidden) => {
            return Ok(err_reply(403, "email_unverified_or_missing", true));
        }
        Err(e) => {
            tracing::error!(error = %e, "resolve_user_for_profile failed");
            return Ok(err_reply(500, "internal", true));
        }
    };
    let user_id = match &outcome {
        ResolveOutcome::Existing(u)
        | ResolveOutcome::LinkedToExisting(u)
        | ResolveOutcome::Created(u) => u.clone(),
    };

    provider_links::upsert(
        ctx,
        provider_links::NewLink {
            provider: provider_name,
            provider_ref: &profile.provider_ref,
            user_id: &user_id,
            provider_login: &profile.login,
            access_token: &profile.access_token,
        },
    )
    .await
    .map_err(|e| WaferError::new(ErrorCode::INTERNAL, format!("provider_links upsert: {e}")))?;

    let issued = session::issue_for(ctx, &user_id, cfg.session_lifetime_days).await?;

    let next = payload
        .next
        .unwrap_or_else(|| "/auth/dashboard".to_string());
    Ok(HttpReply::new(303)
        .header("Location", next)
        .header(
            "Set-Cookie",
            session_cookie(&issued.raw_token, cfg.session_lifetime_days),
        )
        .header("Set-Cookie", clear_cookie_header()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_next_path_allows_relative() {
        assert_eq!(safe_next_path(Some("/dashboard")), "/dashboard".to_string());
        assert_eq!(safe_next_path(Some("/orgs/acme")), "/orgs/acme".to_string());
    }

    #[test]
    fn safe_next_path_rejects_schemed_url() {
        assert_eq!(
            safe_next_path(Some("https://evil.example/phish")),
            "/auth/dashboard"
        );
        assert_eq!(
            safe_next_path(Some("//evil.example/phish")),
            "/auth/dashboard"
        );
    }

    #[test]
    fn safe_next_path_rejects_missing() {
        assert_eq!(safe_next_path(None), "/auth/dashboard");
    }

    #[test]
    fn new_state_value_is_43_chars_base64url() {
        let s = new_state_value();
        assert_eq!(s.len(), 43);
        assert!(s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }
}
