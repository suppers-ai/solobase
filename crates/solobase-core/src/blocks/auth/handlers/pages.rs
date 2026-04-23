//! GET handlers for HTML pages + POST `/auth/signup` + POST `/auth/cli/issue`
//! (content-negotiated htmx fragment).
//!
//! Handlers build view-models from `AuthService` + repo data and render maud
//! templates declared in [`crate::blocks::auth::templates`]. Validation helpers
//! are split out so they remain unit-testable without a runtime context.

use base64ct::{Base64UrlUnpadded, Encoding};
use serde_json::json;
use wafer_core::{
    clients::crypto,
    interfaces::auth::service::{AuthError, AuthService},
};
use wafer_run::{
    context::Context,
    types::{ErrorCode, Message, WaferError},
};

use super::HttpReply;
use crate::blocks::auth::{
    config::AuthConfig,
    repo::{local_credentials, orgs, pats, users},
    session, templates,
    view_models::{
        CliCodeFragmentViewModel, CliLoginViewModel, DashboardViewModel, LoginViewModel, NavUser,
        OrgsDetailViewModel, PatRow, SignupViewModel,
    },
};

// ---------------------------------------------------------------------------
// Pure helpers (unit-tested below)
// ---------------------------------------------------------------------------

/// Input for [`build_login_vm`].
pub struct BuildLoginInput {
    pub error: Option<String>,
    pub signup_enabled: bool,
    pub configured_oauth_providers: Vec<String>,
    pub next_path: Option<String>,
}

/// Construct a [`LoginViewModel`] from its primitive inputs. Pure — no I/O.
pub fn build_login_vm(input: BuildLoginInput) -> LoginViewModel {
    LoginViewModel {
        error: input.error,
        signup_enabled: input.signup_enabled,
        oauth_providers: input.configured_oauth_providers,
        next_path: input.next_path,
    }
}

/// Input for [`validate_signup`].
pub struct SignupInput {
    pub email: String,
    pub password: String,
    pub password_min_length: u32,
}

/// Validate a signup form submission. Email must contain exactly one `@`
/// with non-empty local + domain parts and a dot in the domain; password
/// must be at least `password_min_length` characters. Pure.
pub fn validate_signup(input: SignupInput) -> Result<(), String> {
    if !is_plausible_email(&input.email) {
        return Err("email must be a valid address".into());
    }
    if (input.password.chars().count() as u32) < input.password_min_length {
        return Err(format!(
            "password must be at least {} characters",
            input.password_min_length
        ));
    }
    Ok(())
}

/// Hand-rolled `\S+@\S+\.\S+`-ish check: one `@`, non-empty on both sides,
/// a dot in the domain, no whitespace anywhere.
fn is_plausible_email(s: &str) -> bool {
    if s.chars().any(|c| c.is_whitespace()) {
        return false;
    }
    let Some((local, domain)) = s.split_once('@') else {
        return false;
    };
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    // second '@' would produce a second split — reject.
    if domain.contains('@') {
        return false;
    }
    let Some((d_l, d_r)) = domain.split_once('.') else {
        return false;
    };
    !d_l.is_empty() && !d_r.is_empty()
}

/// True when the caller's `Accept` header or `hx-request` header indicates
/// they want an HTML fragment instead of a JSON body. Used by the
/// content-negotiated `POST /auth/cli/issue` handler.
pub fn prefers_html_fragment(msg: &Message) -> bool {
    if !msg.header("hx-request").is_empty() {
        return true;
    }
    let accept = msg.header("accept");
    if accept.is_empty() {
        return false;
    }
    let l = accept.to_ascii_lowercase();
    l.contains("text/html")
}

/// Derive the list of OAuth providers whose full triple (CLIENT_ID,
/// CLIENT_SECRET, REDIRECT_URL) is present in the process env. Matches the
/// logic used by `providers::registry::build_providers` so the login page
/// surfaces exactly the buttons that will actually work.
fn configured_oauth_providers() -> Vec<String> {
    ["github", "google", "microsoft"]
        .iter()
        .filter(|p| {
            let up = p.to_ascii_uppercase();
            let id = std::env::var(format!("SOLOBASE_SHARED__AUTH__{up}__CLIENT_ID"))
                .unwrap_or_default();
            let secret = std::env::var(format!("SOLOBASE_SHARED__AUTH__{up}__CLIENT_SECRET"))
                .unwrap_or_default();
            let url = std::env::var(format!("SOLOBASE_SHARED__AUTH__{up}__REDIRECT_URL"))
                .unwrap_or_default();
            !id.is_empty() && !secret.is_empty() && !url.is_empty()
        })
        .map(|s| (*s).to_string())
        .collect()
}

fn query_opt(msg: &Message, key: &str) -> Option<String> {
    let v = msg.query(key);
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}

/// Parse a `application/x-www-form-urlencoded` body into a list of
/// `(key, value)` pairs. Kept in-module to avoid pulling an extra crate.
fn parse_form(body: &[u8]) -> Vec<(String, String)> {
    let s = std::str::from_utf8(body).unwrap_or("");
    s.split('&')
        .filter(|p| !p.is_empty())
        .map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (url_decode(k), url_decode(v))
        })
        .collect()
}

fn form_get(form: &[(String, String)], key: &str) -> Option<String> {
    form.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let (Some(hi), Some(lo)) = (hex_nib(bytes[i + 1]), hex_nib(bytes[i + 2])) {
                    out.push((hi << 4) | lo);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_nib(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Build the `Set-Cookie` value for a freshly-issued session. Mirrors the
/// JSON API login handler so the cookie shape stays identical.
fn session_cookie(raw: &str, lifetime_days: u32) -> String {
    let max_age = (lifetime_days as u64) * 24 * 60 * 60;
    format!(
        "{name}={raw}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age={max_age}",
        name = session::COOKIE_NAME,
    )
}

fn html(status: u16, body: String) -> HttpReply {
    HttpReply::new(status)
        .header("Content-Type", "text/html; charset=utf-8")
        .raw_body(body.into_bytes())
}

fn redirect_see_other(location: &str) -> HttpReply {
    HttpReply::new(303).header("Location", location)
}

// ---------------------------------------------------------------------------
// HTTP handlers
// ---------------------------------------------------------------------------

/// GET `/auth/login` — render the sign-in form.
pub async fn get_login(msg: &Message, cfg: &AuthConfig) -> HttpReply {
    let vm = build_login_vm(BuildLoginInput {
        error: query_opt(msg, "error"),
        signup_enabled: cfg.signup_enabled,
        configured_oauth_providers: configured_oauth_providers(),
        next_path: query_opt(msg, "next"),
    });
    html(200, templates::login::render(&vm).into_string())
}

/// GET `/auth/signup` — render the account-creation form. Returns 404 when
/// `SIGNUP_ENABLED` is false so the endpoint is effectively absent.
pub async fn get_signup(msg: &Message, cfg: &AuthConfig) -> HttpReply {
    if !cfg.signup_enabled {
        return HttpReply::new(404).json_body(&json!({ "error": "not_found" }));
    }
    let vm = SignupViewModel {
        error: query_opt(msg, "error"),
        password_min_length: cfg.password_min_length,
        next_path: query_opt(msg, "next"),
    };
    html(200, templates::signup::render(&vm).into_string())
}

/// POST `/auth/signup` — create user + local_credentials + session.
///
/// Validation failures re-render the form with an inline banner at 400/409;
/// the success path 303-redirects to `next` (or `/auth/dashboard`) with the
/// `wafer_session` cookie set.
pub async fn post_signup(
    ctx: &dyn Context,
    cfg: &AuthConfig,
    body: &[u8],
) -> Result<HttpReply, WaferError> {
    if !cfg.signup_enabled {
        return Ok(HttpReply::new(404).json_body(&json!({ "error": "not_found" })));
    }

    let form = parse_form(body);
    let email = form_get(&form, "email").unwrap_or_default();
    let password = form_get(&form, "password").unwrap_or_default();
    let next = form_get(&form, "next");

    let min = cfg.password_min_length;
    if let Err(err) = validate_signup(SignupInput {
        email: email.clone(),
        password: password.clone(),
        password_min_length: min,
    }) {
        let vm = SignupViewModel {
            error: Some(err),
            password_min_length: min,
            next_path: next,
        };
        return Ok(html(400, templates::signup::render(&vm).into_string()));
    }

    let email_lower = email.trim().to_ascii_lowercase();

    // Duplicate-email check. Emails are stored lower-case on insert, so a
    // case-insensitive compare against the normalised form is sufficient.
    let existing = users::find_by_email(ctx, &email_lower)
        .await
        .map_err(|e| WaferError::new(ErrorCode::INTERNAL, format!("users find: {e}")))?;
    if existing.is_some() {
        let vm = SignupViewModel {
            error: Some("email already registered".into()),
            password_min_length: min,
            next_path: next,
        };
        return Ok(html(409, templates::signup::render(&vm).into_string()));
    }

    let hash = crypto::hash(ctx, &password).await?;
    let display_name = email_lower
        .split_once('@')
        .map(|(l, _)| l.to_string())
        .unwrap_or_else(|| email_lower.clone());
    let user = users::insert(
        ctx,
        users::NewUser {
            email: email_lower,
            display_name,
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .map_err(|e| WaferError::new(ErrorCode::INTERNAL, format!("users insert: {e}")))?;

    local_credentials::insert(ctx, &user.id, &hash, false)
        .await
        .map_err(|e| WaferError::new(ErrorCode::INTERNAL, format!("credentials insert: {e}")))?;

    let issued = session::issue_for(ctx, &user.id, cfg.session_lifetime_days).await?;
    let location = next.as_deref().unwrap_or("/auth/dashboard").to_string();
    Ok(redirect_see_other(&location).header(
        "Set-Cookie",
        session_cookie(&issued.raw_token, cfg.session_lifetime_days),
    ))
}

/// GET `/auth/dashboard` — profile + orgs + PATs for the signed-in user.
/// Unauthenticated callers are redirected to `/auth/login?next=/auth/dashboard`.
pub async fn get_dashboard(
    ctx: &dyn Context,
    service: &dyn AuthService,
    msg: &Message,
) -> Result<HttpReply, WaferError> {
    let user_id = match service.require_user(msg).await {
        Ok(u) => u,
        Err(_) => return Ok(redirect_see_other("/auth/login?next=/auth/dashboard")),
    };
    let profile = match service.user_profile(user_id.clone()).await {
        Ok(p) => p,
        Err(e) => return Err(internal_from_auth(e)),
    };
    let pat_rows = pats::list_for_user(ctx, &user_id.0)
        .await
        .map_err(|e| WaferError::new(ErrorCode::INTERNAL, format!("pats list: {e}")))?;
    let pats_vm: Vec<PatRow> = pat_rows
        .into_iter()
        .map(|p| PatRow {
            id: super::tokens::hex(&p.token_hash),
            name: p.name,
            scopes: p.scopes,
            created_at_iso: p.created_at,
            last_used_at_iso: p.last_used_at,
        })
        .collect();

    let vm = DashboardViewModel {
        user: NavUser::from_profile(&profile),
        email: profile.email.clone(),
        orgs: profile.orgs.clone(),
        pats: pats_vm,
    };
    Ok(html(200, templates::dashboard::render(&vm).into_string()))
}

/// GET `/auth/cli-login` — authenticated landing page for the CLI flow.
pub async fn get_cli_login(
    service: &dyn AuthService,
    msg: &Message,
) -> Result<HttpReply, WaferError> {
    let user_id = match service.require_user(msg).await {
        Ok(u) => u,
        Err(_) => return Ok(redirect_see_other("/auth/login?next=/auth/cli-login")),
    };
    let profile = match service.user_profile(user_id).await {
        Ok(p) => p,
        Err(e) => return Err(internal_from_auth(e)),
    };
    let vm = CliLoginViewModel {
        user: NavUser::from_profile(&profile),
    };
    Ok(html(200, templates::cli_login::render(&vm).into_string()))
}

/// GET `/auth/orgs/{name}` — public org page. Renders the Manage section
/// only when `verify_org_admin` returns true for the viewer.
pub async fn get_org_detail(
    ctx: &dyn Context,
    service: &dyn AuthService,
    msg: &Message,
    org_name: &str,
) -> Result<HttpReply, WaferError> {
    let org = match orgs::find_by_name(ctx, org_name)
        .await
        .map_err(|e| WaferError::new(ErrorCode::INTERNAL, format!("orgs find: {e}")))?
    {
        Some(o) => o,
        None => return Ok(HttpReply::new(404).json_body(&json!({ "error": "not_found" }))),
    };

    let (user_nav, viewer_is_admin) = match service.require_user(msg).await {
        Ok(uid) => {
            let profile = service.user_profile(uid.clone()).await.ok();
            let is_admin = match (&org.verified_via, &org.verified_ref) {
                (Some(v), Some(r)) => service
                    .verify_org_admin(uid.clone(), v, r)
                    .await
                    .unwrap_or(false),
                // Reserved-org path: no verified_via/ref, but site admin
                // still counts. Ask with a sentinel "reserved" provider
                // name — AuthServiceImpl's reserved-org branch inspects the
                // row, not the provider string, so any non-empty value is
                // fine.
                _ if org.is_reserved => service
                    .verify_org_admin(uid.clone(), "reserved", org_name)
                    .await
                    .unwrap_or(false),
                _ => false,
            };
            (profile.as_ref().map(NavUser::from_profile), is_admin)
        }
        Err(_) => (None, false),
    };

    let vm = OrgsDetailViewModel {
        user: user_nav,
        org: wafer_core::interfaces::auth::service::OrgSummary {
            name: org.name.clone(),
            verified_via: org.verified_via.clone(),
            verified_ref: org.verified_ref.clone(),
            is_reserved: org.is_reserved,
        },
        viewer_is_admin,
        is_reserved: org.is_reserved,
    };
    Ok(html(200, templates::orgs_detail::render(&vm).into_string()))
}

/// POST `/auth/cli/issue` when the caller wants an HTML fragment back
/// (htmx button click). Wraps the JSON implementation by re-issuing a code
/// and rendering [`templates::cli_code_fragment`].
pub async fn post_cli_issue_fragment(
    ctx: &dyn Context,
    service: &dyn AuthService,
    msg: &Message,
) -> Result<HttpReply, WaferError> {
    use sha2::{Digest, Sha256};
    // Reject bearer — same rule as the JSON handler: only the browser
    // session can mint CLI codes.
    if !msg.header("authorization").is_empty() {
        return Ok(HttpReply::new(401).json_body(&json!({
            "error": "unauthorized",
            "detail": "use session cookie, not bearer token",
        })));
    }

    let user_id = match service.require_user(msg).await {
        Ok(u) => u,
        Err(AuthError::Unauthorized) | Err(AuthError::Forbidden) | Err(AuthError::NotFound) => {
            return Ok(HttpReply::new(401).json_body(&json!({
                "error": "unauthorized",
                "detail": "authentication required",
            })))
        }
        Err(e) => return Err(internal_from_auth(e)),
    };

    // Generate a fresh 32-byte code and persist its sha256 hash with 15-min TTL.
    let raw = {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes)
            .map_err(|e| WaferError::new(ErrorCode::INTERNAL, format!("getrandom: {e}")))?;
        Base64UrlUnpadded::encode_string(&bytes)
    };
    let hash = {
        let mut h = Sha256::new();
        h.update(raw.as_bytes());
        h.finalize().to_vec()
    };
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(15 * 60);
    let expires_iso = expires_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    crate::blocks::auth::repo::cli_codes::insert(
        ctx,
        crate::blocks::auth::repo::cli_codes::NewCode {
            code_hash: &hash,
            user_id: &user_id.0,
            expires_at: &expires_iso,
        },
    )
    .await
    .map_err(|e| WaferError::new(ErrorCode::INTERNAL, format!("cli_codes insert: {e}")))?;

    let vm = CliCodeFragmentViewModel {
        code: raw,
        expires_in_minutes: 15,
    };
    Ok(html(
        200,
        templates::cli_code_fragment::render(&vm).into_string(),
    ))
}

// ---------------------------------------------------------------------------
// Internals — error mapping
// ---------------------------------------------------------------------------

fn internal_from_auth(e: AuthError) -> WaferError {
    match e {
        AuthError::ProviderDown(m) => WaferError::new(ErrorCode::UNAVAILABLE, m),
        other => WaferError::new(ErrorCode::INTERNAL, other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_login_vm_respects_signup_enabled_flag() {
        let vm = build_login_vm(BuildLoginInput {
            error: None,
            signup_enabled: true,
            configured_oauth_providers: vec!["github".into()],
            next_path: Some("/x".into()),
        });
        assert!(vm.signup_enabled);
        assert_eq!(vm.oauth_providers, vec!["github".to_string()]);
        assert_eq!(vm.next_path.as_deref(), Some("/x"));
    }

    #[test]
    fn signup_validation_rejects_short_password() {
        let err = validate_signup(SignupInput {
            email: "a@b.com".into(),
            password: "short".into(),
            password_min_length: 8,
        })
        .unwrap_err();
        assert!(err.contains("at least 8"), "got: {err}");
    }

    #[test]
    fn signup_validation_rejects_bad_email() {
        let err = validate_signup(SignupInput {
            email: "not-an-email".into(),
            password: "aaaaaaaa".into(),
            password_min_length: 8,
        })
        .unwrap_err();
        assert!(err.to_lowercase().contains("email"), "got: {err}");
    }

    #[test]
    fn signup_validation_accepts_valid_input() {
        let ok = validate_signup(SignupInput {
            email: "a@b.com".into(),
            password: "longenough".into(),
            password_min_length: 8,
        });
        assert!(ok.is_ok(), "got: {ok:?}");
    }

    #[test]
    fn plausible_email_accepts_usual_shapes() {
        assert!(is_plausible_email("a@b.com"));
        assert!(is_plausible_email("alice+tag@example.co"));
    }

    #[test]
    fn plausible_email_rejects_missing_parts() {
        assert!(!is_plausible_email(""));
        assert!(!is_plausible_email("a@"));
        assert!(!is_plausible_email("@b.com"));
        assert!(!is_plausible_email("a@b"));
        assert!(!is_plausible_email("a b@c.com"));
        assert!(!is_plausible_email("a@b@c.com"));
    }

    #[test]
    fn parse_form_roundtrips_urlencoded() {
        let pairs = parse_form(b"email=a%40b.com&password=hunter2&next=%2Fx");
        assert_eq!(form_get(&pairs, "email").as_deref(), Some("a@b.com"));
        assert_eq!(form_get(&pairs, "password").as_deref(), Some("hunter2"));
        assert_eq!(form_get(&pairs, "next").as_deref(), Some("/x"));
    }

    #[test]
    fn parse_form_handles_plus_as_space() {
        let pairs = parse_form(b"name=hello+world");
        assert_eq!(form_get(&pairs, "name").as_deref(), Some("hello world"));
    }
}
