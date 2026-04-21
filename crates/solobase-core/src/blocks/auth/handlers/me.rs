//! GET `/auth/me` — return the authenticated user's profile.
//!
//! Uses the block's `AuthService` implementation to resolve whatever
//! credentials were presented on the incoming [`Message`] (bearer token or
//! `wafer_session` cookie). On missing/invalid creds, returns 401; otherwise
//! returns the `UserProfile` serialised to JSON. `orgs` is always `[]` until
//! Plan C lands.

use serde_json::{json, Value};
use wafer_core::interfaces::auth::service::{AuthError, AuthService, Role};
use wafer_run::types::Message;

use super::HttpReply;

fn unauthorized() -> HttpReply {
    HttpReply::new(401).json_body(&json!({ "error": "unauthorized" }))
}

/// GET `/auth/me`.
pub async fn get_me(service: &dyn AuthService, msg: &Message) -> HttpReply {
    let user_id = match service.require_user(msg).await {
        Ok(u) => u,
        Err(AuthError::Unauthorized) | Err(AuthError::NotFound) => return unauthorized(),
        Err(AuthError::Forbidden) => return unauthorized(),
        Err(AuthError::ProviderDown(m)) => {
            return HttpReply::new(503).json_body(&json!({ "error": "provider_down", "detail": m }))
        }
        Err(AuthError::Internal(m)) => {
            return HttpReply::new(500).json_body(&json!({ "error": "internal", "detail": m }))
        }
    };

    let profile = match service.user_profile(user_id).await {
        Ok(p) => p,
        Err(AuthError::NotFound) => return unauthorized(),
        Err(e) => {
            return HttpReply::new(500)
                .json_body(&json!({ "error": "profile_lookup_failed", "detail": format!("{e:?}") }))
        }
    };

    let role_str = match profile.role {
        Role::Admin => "admin",
        Role::User => "user",
    };
    let orgs: Vec<Value> = profile
        .orgs
        .iter()
        .map(|o| {
            json!({
                "name": o.name,
                "verified_via": o.verified_via,
                "verified_ref": o.verified_ref,
                "is_reserved": o.is_reserved,
            })
        })
        .collect();

    HttpReply::new(200).json_body(&json!({
        "id": profile.id.0,
        "email": profile.email,
        "display_name": profile.display_name,
        "avatar_url": profile.avatar_url,
        "role": role_str,
        "orgs": orgs,
    }))
}
