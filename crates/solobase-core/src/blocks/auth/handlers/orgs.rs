//! POST `/auth/orgs/claim` — claim an organisation name after proving
//! provider-level admin.
//!
//! Flow (spec §5 "Org claim"):
//!   1. Require a session user (cookie or PAT both accepted — no special
//!      restriction beyond `require_user`).
//!   2. Parse `{name, provider}` from the JSON body.
//!   3. Validate `name` against `^[a-z][a-z0-9-]{0,39}$` → 400 otherwise.
//!   4. Reject the four reserved names `wafer-run`, `wafer`, `suppers-ai`,
//!      `solobase` → 409 with `error: "reserved"` (distinct from the two
//!      upsert 409s below).
//!   5. Look up the caller's `provider_links` row for `(user_id, provider)`
//!      — missing → 422 asking the user to link the provider first.
//!   6. Dispatch to `provider.check_org_admin(access_token, name)` — an
//!      upstream 5xx bubbles as 503, a flat "no" as 403.
//!   7. `orgs::upsert_claimed(...)`:
//!      - `NameTaken` → 409 `"name taken"`
//!      - `AlreadyClaimed` → 409 `"org already claimed"`
//!      - success → 201 + JSON + warm the cache with `true`.
//!
//! Two distinct 409 shapes: the body `error` field distinguishes them so
//! the Plan D HTML can render a different message for each.

use serde_json::{json, Value};
use wafer_core::interfaces::auth::service::{AuthError, AuthService, UserId};
use wafer_run::{
    context::Context,
    types::{Message, WaferError},
};

use super::HttpReply;
use crate::blocks::auth::{
    cache::OrgAdminCache,
    providers::{registry::ProviderRegistry, ProviderError},
    repo::{
        orgs::{self, OrgsRepoError},
        provider_links,
    },
};

/// Reserved org names (spec §3 and migration 002). Claim requests targeting
/// these return 409 without touching the database — the seeded rows are
/// flagged `is_reserved = 1` and owned implicitly by the platform.
pub const RESERVED: &[&str] = &["wafer-run", "wafer", "suppers-ai", "solobase"];

/// Validate against `^[a-z][a-z0-9-]{0,39}$` without pulling a `regex`
/// dependency in just for this one check. The grammar is small enough that
/// a hand-rolled validator is clearer and faster.
fn is_valid_org_name(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes.len() > 40 {
        return false;
    }
    if !matches!(bytes[0], b'a'..=b'z') {
        return false;
    }
    bytes[1..]
        .iter()
        .all(|b| matches!(*b, b'a'..=b'z' | b'0'..=b'9' | b'-'))
}

fn unauthorized() -> HttpReply {
    HttpReply::new(401).json_body(&json!({ "error": "unauthorized" }))
}

fn parse_claim(body: &[u8]) -> Option<(String, String)> {
    let v: Value = serde_json::from_slice(body).ok()?;
    let name = v.get("name").and_then(Value::as_str)?.to_string();
    let provider = v.get("provider").and_then(Value::as_str)?.to_string();
    Some((name, provider))
}

/// POST `/auth/orgs/claim`.
pub async fn post_claim(
    ctx: &dyn Context,
    service: &dyn AuthService,
    providers: &ProviderRegistry,
    org_admin_cache: &OrgAdminCache,
    msg: &Message,
    body: &[u8],
) -> Result<HttpReply, WaferError> {
    let user_id = match service.require_user(msg).await {
        Ok(u) => u,
        Err(AuthError::Unauthorized)
        | Err(AuthError::Forbidden)
        | Err(AuthError::NotFound) => return Ok(unauthorized()),
        Err(AuthError::ProviderDown(m)) => {
            return Ok(
                HttpReply::new(503).json_body(&json!({ "error": "provider_down", "detail": m }))
            )
        }
        Err(AuthError::Internal(m)) => {
            return Ok(
                HttpReply::new(500).json_body(&json!({ "error": "internal", "detail": m }))
            )
        }
    };

    let (name, provider) = match parse_claim(body) {
        Some(p) => p,
        None => {
            return Ok(HttpReply::new(400).json_body(&json!({
                "error": "invalid_body",
                "detail": "expected {name, provider}",
            })))
        }
    };

    if !is_valid_org_name(&name) {
        return Ok(HttpReply::new(400).json_body(&json!({
            "error": "invalid_name",
            "detail": "must match ^[a-z][a-z0-9-]{0,39}$",
        })));
    }

    if RESERVED.contains(&name.as_str()) {
        return Ok(HttpReply::new(409).json_body(&json!({
            "error": "reserved",
            "detail": "this name is reserved by the platform",
        })));
    }

    // Caller must have linked the provider before claiming so we can prove
    // identity with the stored access token.
    let link = match provider_links::find_by_user_provider(ctx, &user_id.0, &provider)
        .await
        .map_err(|e| {
            WaferError::new(
                wafer_run::types::ErrorCode::INTERNAL,
                format!("provider_links lookup: {e}"),
            )
        })? {
        Some(l) => l,
        None => {
            return Ok(HttpReply::new(422).json_body(&json!({
                "error": "provider_not_linked",
                "detail": format!("link {provider} first"),
            })))
        }
    };

    let provider_impl = match providers.get(&provider) {
        Some(p) => p,
        None => {
            return Ok(HttpReply::new(400).json_body(&json!({
                "error": "unknown_provider",
                "detail": provider,
            })))
        }
    };

    let is_admin = match provider_impl.check_org_admin(&link.access_token, &name).await {
        Ok(v) => v,
        Err(ProviderError::NotSupported) | Err(ProviderError::Unauthorized) => false,
        Err(ProviderError::Upstream(m)) => {
            return Ok(HttpReply::new(503).json_body(&json!({
                "error": "provider_unavailable",
                "detail": m,
            })))
        }
        Err(other) => {
            return Ok(HttpReply::new(500).json_body(&json!({
                "error": "provider_error",
                "detail": other.to_string(),
            })))
        }
    };

    if !is_admin {
        return Ok(HttpReply::new(403).json_body(&json!({
            "error": "not_org_admin",
            "detail": format!("{provider} says you are not an admin of {name}"),
        })));
    }

    match orgs::upsert_claimed(
        ctx,
        orgs::NewClaim {
            name: &name,
            owner_user_id: &user_id.0,
            verified_via: &provider,
            verified_ref: &name,
        },
    )
    .await
    {
        Ok(row) => {
            // Warm the cache — we just proved admin, no need to re-ask
            // the provider for 5 minutes.
            org_admin_cache.insert(&UserId(user_id.0.clone()), &provider, &name, true);
            Ok(HttpReply::new(201).json_body(&json!({
                "id": row.id,
                "name": row.name,
                "verified_via": row.verified_via,
                "verified_ref": row.verified_ref,
                "owner_user_id": row.owner_user_id,
                "created_at": row.created_at,
            })))
        }
        Err(OrgsRepoError::NameTaken) => Ok(HttpReply::new(409).json_body(&json!({
            "error": "name_taken",
            "detail": "that name is already taken",
        }))),
        Err(OrgsRepoError::AlreadyClaimed) => Ok(HttpReply::new(409).json_body(&json!({
            "error": "already_claimed",
            "detail": "that provider org is already claimed by another user",
        }))),
        Err(OrgsRepoError::Db(e)) => Ok(HttpReply::new(500)
            .json_body(&json!({ "error": "internal", "detail": format!("orgs upsert: {e}") }))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_name_min_and_max() {
        assert!(is_valid_org_name("a"));
        assert!(is_valid_org_name("acme"));
        assert!(is_valid_org_name("a-b-c"));
        assert!(is_valid_org_name(&"a".repeat(40)));
    }

    #[test]
    fn invalid_names() {
        assert!(!is_valid_org_name(""));
        assert!(!is_valid_org_name("Acme"));
        assert!(!is_valid_org_name("-acme"));
        assert!(!is_valid_org_name("1acme"));
        assert!(!is_valid_org_name("acme_org"));
        assert!(!is_valid_org_name("acme/corp"));
        assert!(!is_valid_org_name(&"a".repeat(41)));
    }
}
