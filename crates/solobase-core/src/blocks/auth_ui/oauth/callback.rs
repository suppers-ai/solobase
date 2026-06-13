//! GET /b/auth/oauth/callback — relocated from auth/oauth.rs::handle_oauth_callback
//! in Task 5.

use std::collections::HashMap;

use wafer_core::clients::{config, database as db, network};
use wafer_run::{context::Context, Message, OutputStream};

use crate::blocks::{
    auth::{
        helpers::{
            email_domain_allowed, ensure_admin_role, initial_role_for, issue_tokens_and_cookie,
            signup_allowed,
        },
        repo::{oauth_pkce, provider_links, users},
        USERS_TABLE,
    },
    auth_ui::redirect::is_safe_local_redirect,
    helpers::{
        err_bad_request, err_forbidden, err_internal, err_internal_no_cause, json_map,
        ResponseBuilder,
    },
};

pub async fn handle(ctx: &dyn Context, msg: &Message) -> OutputStream {
    // Check ENABLE_OAUTH flag
    let enable_oauth = config::get_default(ctx, "SOLOBASE_SHARED__ENABLE_OAUTH", "false").await;
    if enable_oauth != "true" && enable_oauth != "1" {
        return err_forbidden("OAuth login is not enabled");
    }

    let code = msg.query("code");
    let state = msg.query("state");
    if code.is_empty() || state.is_empty() {
        return err_bad_request("Missing code or state parameter");
    }

    // SEC-040: look up the server-side PKCE state by the opaque `state_id`
    // the provider echoed back. `take` is single-use (DELETE … RETURNING),
    // so a replayed callback or a stolen state_id can only redeem once,
    // and a state past `expires_at` is treated as missing.
    let pkce_row = match oauth_pkce::take(ctx, state).await {
        Ok(Some(row)) => row,
        Ok(None) => return err_bad_request("Invalid or expired OAuth state"),
        Err(e) => return err_internal("OAuth state lookup failed", e),
    };
    let provider = pkce_row.provider.clone();
    let code_verifier = pkce_row.code_verifier.clone();
    // Use the redirect_uri stored at start-time so the provider's exact-
    // match check passes even if the live config changed mid-flow.
    let redirect_uri = pkce_row.redirect_uri.clone();

    let spec = match super::spec::lookup(&provider) {
        Some(s) => s,
        None => return err_bad_request("Unsupported OAuth provider"),
    };

    let client_id = config::get_default(
        ctx,
        &format!(
            "SUPPERS_AI__AUTH_UI__OAUTH_{}_CLIENT_ID",
            provider.to_uppercase()
        ),
        "",
    )
    .await;
    let client_secret = config::get_default(
        ctx,
        &format!(
            "SUPPERS_AI__AUTH_UI__OAUTH_{}_CLIENT_SECRET",
            provider.to_uppercase()
        ),
        "",
    )
    .await;

    if client_id.is_empty() || client_secret.is_empty() {
        return err_internal_no_cause("OAuth provider not fully configured");
    }

    // Phase 1: exchange the authorization code for a provider access token.
    let oauth_token = match exchange_code(
        ctx,
        spec,
        code,
        &client_id,
        &client_secret,
        &redirect_uri,
        &code_verifier,
    )
    .await
    {
        Ok(t) => t,
        Err(r) => return r,
    };

    // Phase 2: fetch the user's profile (with GitHub's /user/emails fallback).
    let info = match fetch_user_info(ctx, spec, &oauth_token).await {
        Ok(i) => i,
        Err(r) => return r,
    };

    // Phase 3: resolve the local user (link / email-merge / create), enforcing
    // the disabled-account and signup gates, and upsert the provider link.
    let user_id = match resolve_user(ctx, &provider, &oauth_token, &info).await {
        Ok(id) => id,
        Err(r) => return r,
    };

    // Update last_login_at on the users row (best-effort).
    let upd = json_map(serde_json::json!({
        "last_login_at": crate::blocks::helpers::now_rfc3339()
    }));
    if let Err(e) = db::update(ctx, USERS_TABLE, &user_id, upd).await {
        tracing::warn!("Failed to update last_login_at: {e}");
    }

    let email = info.email;

    let roles = ensure_admin_role(ctx, &user_id, &email).await;

    // Mint tokens, persist the refresh + session rows, build the cookie via
    // the shared issuance tail. Previously this flow hand-rolled token minting
    // and *omitted* the session row, so OAuth logins were invisible on the
    // userportal device list; routing through `issue_tokens_and_cookie` fixes
    // that by construction.
    let issued = match issue_tokens_and_cookie(
        ctx,
        &user_id,
        &email,
        &roles,
        &format!("oauth.{}", provider),
        None,
        0,
    )
    .await
    {
        Ok(i) => i,
        Err(r) => return r,
    };

    // Redirect to frontend — token is set via HttpOnly cookie only (not URL)
    let frontend_url = config::get_default(
        ctx,
        "SOLOBASE_SHARED__FRONTEND_URL",
        "http://localhost:5173",
    )
    .await;
    // [SEC-036] Validate FRONTEND_URL before plugging it into a Location
    // header — a misconfigured (or attacker-controlled) value here would
    // turn every OAuth callback into an open redirect.
    if !is_safe_frontend_url(&frontend_url) {
        tracing::error!(
            frontend_url = %frontend_url,
            "SOLOBASE_SHARED__FRONTEND_URL failed validation; refusing OAuth redirect"
        );
        return err_internal_no_cause("Frontend URL is not configured correctly");
    }
    let post_login_raw =
        config::get_default(ctx, "SOLOBASE_SHARED__POST_LOGIN_REDIRECT", "/b/admin/").await;
    let post_login = if is_safe_local_redirect(&post_login_raw) {
        post_login_raw
    } else {
        "/b/admin/".to_string()
    };
    let redirect_url = format!("{}{}", frontend_url.trim_end_matches('/'), post_login);

    ResponseBuilder::new()
        .status(302)
        .set_cookie(&issued.cookie)
        .set_header("Location", &redirect_url)
        .json(&serde_json::json!({"redirect": redirect_url}))
}

/// The profile fields the callback needs from a provider's userinfo response,
/// already normalised (email lowercased, missing strings empty).
struct OAuthUserInfo {
    /// Lowercased verified email (after the GitHub `/user/emails` fallback).
    email: String,
    /// Display name, empty if the provider omitted it.
    name: String,
    /// Avatar URL, empty if the provider omitted it.
    avatar: String,
    /// Stable provider-side user id (`sub` for Google, `id` for GitHub /
    /// Microsoft), coerced to a string.
    provider_ref: String,
    /// Per-provider login handle (GitHub `login`, else the email local-part).
    provider_login: String,
}

/// Phase 1 — exchange the authorization `code` for a provider access token.
///
/// POSTs the token-exchange body (PKCE verifier included where the provider
/// uses it) and returns the `access_token` string. Any transport / parse
/// failure or a missing token is mapped to a ready-to-return [`OutputStream`].
async fn exchange_code(
    ctx: &dyn Context,
    spec: &super::spec::OAuthProviderSpec,
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<String, OutputStream> {
    let token_body_str =
        spec.build_token_body(code, client_id, client_secret, redirect_uri, code_verifier);

    let mut headers = HashMap::new();
    headers.insert(
        "Content-Type".to_string(),
        "application/x-www-form-urlencoded".to_string(),
    );
    headers.insert("Accept".to_string(), "application/json".to_string());

    let token_body_bytes = token_body_str.into_bytes();
    let token_resp =
        match network::do_request(ctx, "POST", spec.token_url, &headers, Some(&token_body_bytes))
            .await
        {
            Ok(r) => r,
            Err(e) => return Err(err_internal("Token exchange failed", e)),
        };

    let token_data: serde_json::Value = match serde_json::from_slice(&token_resp.body) {
        Ok(d) => d,
        Err(_) => return Err(err_internal_no_cause("Failed to parse token response")),
    };

    let access_token_oauth = token_data
        .get("access_token")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if access_token_oauth.is_empty() {
        return Err(err_internal_no_cause("No access token in OAuth response"));
    }
    Ok(access_token_oauth.to_string())
}

/// Phase 2 — fetch and normalise the user's profile.
///
/// Calls the provider userinfo endpoint, then (for providers with an
/// `emails_url`, i.e. GitHub) falls back to `/user/emails` for a verified
/// primary address when the userinfo payload omits one. Returns the normalised
/// [`OAuthUserInfo`]; a missing email or stable id is an error.
async fn fetch_user_info(
    ctx: &dyn Context,
    spec: &super::spec::OAuthProviderSpec,
    oauth_token: &str,
) -> Result<OAuthUserInfo, OutputStream> {
    // Shared header set for every provider API call. GitHub's REST API rejects
    // requests without a User-Agent header (returns 403 + an HTML error body);
    // other providers accept it.
    let api_headers = || {
        let mut h = HashMap::new();
        h.insert(
            "Authorization".to_string(),
            spec.userinfo_auth_header(oauth_token),
        );
        h.insert("Accept".to_string(), "application/json".to_string());
        h.insert(
            "User-Agent".to_string(),
            concat!("solobase-auth/", env!("CARGO_PKG_VERSION")).to_string(),
        );
        h
    };

    let info_resp =
        match network::do_request(ctx, "GET", spec.userinfo_url, &api_headers(), None).await {
            Ok(r) => r,
            Err(e) => return Err(err_internal("User info request failed", e)),
        };

    let user_info: serde_json::Value = match serde_json::from_slice(&info_resp.body) {
        Ok(d) => d,
        Err(e) => {
            // Log the SHA-256 hash of the body instead of the body itself —
            // a parse failure is rare and the raw body typically contains
            // the upstream email / provider IDs that we don't want to drop
            // into the error log surface.
            let body_hash = crate::blocks::helpers::sha256_hex(&info_resp.body);
            return Err(err_internal(
                "Failed to parse OAuth user info",
                format!(
                    "status={} parse={} body_len={} body_sha256={}",
                    info_resp.status_code,
                    e,
                    info_resp.body.len(),
                    body_hash
                ),
            ));
        }
    };

    let mut email = user_info
        .get("email")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let name = user_info
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let avatar = user_info
        .get("picture")
        .or_else(|| user_info.get("avatar_url"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // GitHub's /user endpoint returns a null `email` for users who have
    // their primary email set to private. The authoritative list lives at
    // /user/emails — which is only returned when the `user:email` scope
    // was granted. Pick the first primary verified address. Only providers
    // with an `emails_url` (GitHub) carry this fallback.
    if let (true, Some(emails_url)) = (email.is_empty(), spec.emails_url) {
        if let Ok(emails_resp) =
            network::do_request(ctx, "GET", emails_url, &api_headers(), None).await
        {
            if let Ok(arr) = serde_json::from_slice::<serde_json::Value>(&emails_resp.body) {
                if let Some(entries) = arr.as_array() {
                    // Prefer primary+verified; fall back to any verified.
                    let pick = entries
                        .iter()
                        .find(|e| {
                            e.get("primary").and_then(|v| v.as_bool()).unwrap_or(false)
                                && e.get("verified").and_then(|v| v.as_bool()).unwrap_or(false)
                        })
                        .or_else(|| {
                            entries.iter().find(|e| {
                                e.get("verified").and_then(|v| v.as_bool()).unwrap_or(false)
                            })
                        });
                    if let Some(e) = pick {
                        if let Some(s) = e.get("email").and_then(|v| v.as_str()) {
                            email = s.to_lowercase();
                        }
                    }
                }
            }
        }
    }

    if email.is_empty() {
        return Err(err_internal_no_cause("No email returned by OAuth provider"));
    }

    // Extract the stable provider-side user identifier.
    // GitHub returns `id` as a JSON number; Google returns `sub` (string);
    // Microsoft returns `id` (string). Coerce to string in all cases.
    let provider_ref = match user_info.get("sub").or_else(|| user_info.get("id")) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        _ => String::new(),
    };
    if provider_ref.is_empty() {
        return Err(err_internal_no_cause(
            "OAuth provider did not return a stable user id",
        ));
    }

    // Stable per-provider handle (GitHub `login`, others fall back to email local-part).
    let provider_login = user_info
        .get("login")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| email.split('@').next().unwrap_or(""))
        .to_string();

    Ok(OAuthUserInfo {
        email,
        name,
        avatar,
        provider_ref,
        provider_login,
    })
}

/// Phase 3 — resolve the local user id for this OAuth identity.
///
/// Tries, in order: an existing `(provider, provider_ref)` link; an
/// email-matched local account (rejected if disabled); otherwise creates a new
/// user (subject to the shared signup gates). Then upserts the provider link.
/// Returns the resolved local user id.
async fn resolve_user(
    ctx: &dyn Context,
    provider: &str,
    oauth_token: &str,
    info: &OAuthUserInfo,
) -> Result<String, OutputStream> {
    // --- Step 1: look up existing link by (provider, provider_ref) ---
    let existing_link =
        match provider_links::find_by_provider_ref(ctx, provider, &info.provider_ref).await {
            Ok(l) => l,
            Err(e) => return Err(err_internal("provider_links lookup failed", e)),
        };

    // --- Step 2 / 3: resolve user_id ---
    let user_id: String = if let Some(link) = existing_link {
        // Known provider link — reuse the bound user.
        link.user_id
    } else {
        // No link yet. Try email-based account merging.
        match users::find_by_email(ctx, &info.email).await {
            Ok(Some(existing_user)) => {
                // Check if the existing user account is disabled. The
                // authoritative flag is `UserRow.disabled`; the previous
                // `role == "disabled"` check tested a value nothing ever
                // writes, so disabled accounts could still authenticate via
                // OAuth.
                if existing_user.disabled {
                    return Err(err_forbidden("Account is disabled"));
                }
                // Reuse this account; the upsert below will create the new link.
                existing_user.id
            }
            Ok(None) => {
                // Brand-new user — enforce signup gates. Shared with the JSON
                // signup handler so the ALLOW_SIGNUP / ALLOWED_EMAIL_DOMAINS /
                // bootstrap-admin rules can't drift between the two flows.
                if !signup_allowed(ctx).await {
                    return Err(err_forbidden("Signups are currently disabled"));
                }

                if !email_domain_allowed(ctx, &info.email).await {
                    return Err(err_forbidden(
                        "Signups from this email domain are not allowed",
                    ));
                }

                // Determine role: admin if email matches bootstrap email.
                let role = initial_role_for(ctx, &info.email).await;

                let display_name = if info.name.is_empty() {
                    info.email.clone()
                } else {
                    info.name.clone()
                };
                let new_user = users::NewUser {
                    email: info.email.clone(),
                    display_name,
                    avatar_url: if info.avatar.is_empty() {
                        None
                    } else {
                        Some(info.avatar.clone())
                    },
                    role: role.to_string(),
                };
                match users::insert(ctx, new_user).await {
                    Ok(u) => {
                        // Assign role row in USER_ROLES_TABLE for legacy readers.
                        let role_data = json_map(serde_json::json!({
                            "user_id": u.id,
                            "role": role,
                            "assigned_at": crate::blocks::helpers::now_rfc3339()
                        }));
                        if let Err(e) =
                            db::create(ctx, crate::blocks::admin::USER_ROLES_TABLE, role_data).await
                        {
                            tracing::warn!("Failed to assign default role on OAuth signup: {e}");
                        }
                        u.id
                    }
                    Err(e) => return Err(err_internal("Failed to create user", e)),
                }
            }
            Err(e) => return Err(err_internal("User lookup failed", e)),
        }
    };

    // --- Step 4: upsert the provider_links row ---
    if let Err(e) = provider_links::upsert(
        ctx,
        provider_links::NewLink {
            provider,
            provider_ref: &info.provider_ref,
            user_id: &user_id,
            provider_login: &info.provider_login,
            access_token: oauth_token,
        },
    )
    .await
    {
        // Log but don't fail — the user is authenticated; link persistence
        // is best-effort metadata. A failed upsert means re-login will
        // fall back to email-based merging on next attempt.
        tracing::warn!("Failed to upsert provider_links: {e}");
    }

    Ok(user_id)
}

/// [SEC-036] Validates `SOLOBASE_SHARED__FRONTEND_URL` before it is used as
/// the origin half of an OAuth callback redirect.
///
/// The OAuth flow ends by issuing a `302 Location: {frontend_url}{post_login}`.
/// If `frontend_url` is attacker-controlled (admin UI mistake, env-var
/// injection, copy-paste of a phishing URL) this becomes an open redirect that
/// piggybacks on the trusted authentication step.
///
/// Accept only:
/// - `https://<host>` (any non-empty host), OR
/// - `http://localhost[:port]` / `http://127.0.0.1[:port]` for local dev.
///
/// Reject anything with a path beyond `/`, any query, any fragment, anything
/// containing CRLF/tab/other control characters, or any non-http(s) scheme.
fn is_safe_frontend_url(s: &str) -> bool {
    // Reject control characters outright — they enable header-injection even
    // if the rest of the URL parses cleanly.
    if s.chars().any(|c| c.is_control()) {
        return false;
    }
    let parsed = match url::Url::parse(s) {
        Ok(u) => u,
        Err(_) => return false,
    };
    let host = match parsed.host_str() {
        Some(h) if !h.is_empty() => h,
        _ => return false,
    };
    match parsed.scheme() {
        "https" => {}
        "http" => {
            if !(host == "localhost" || host == "127.0.0.1" || host == "[::1]") {
                return false;
            }
        }
        _ => return false,
    }
    // Forbid an embedded path — the redirect formats as
    // `{frontend_url}{post_login}` where post_login already starts with `/`.
    // Allowing a path on frontend_url would invite double-slashes and
    // injection of an unexpected prefix.
    if !(parsed.path().is_empty() || parsed.path() == "/") {
        return false;
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::is_safe_frontend_url;

    #[test]
    fn accepts_https_origins() {
        assert!(is_safe_frontend_url("https://app.example.com"));
        assert!(is_safe_frontend_url("https://app.example.com/"));
        assert!(is_safe_frontend_url("https://app.example.com:8443"));
    }

    #[test]
    fn accepts_http_localhost_for_dev() {
        assert!(is_safe_frontend_url("http://localhost:5173"));
        assert!(is_safe_frontend_url("http://localhost"));
        assert!(is_safe_frontend_url("http://127.0.0.1:3000"));
        assert!(is_safe_frontend_url("http://[::1]:5173"));
    }

    #[test]
    fn rejects_http_non_localhost() {
        assert!(!is_safe_frontend_url("http://evil.com"));
        assert!(!is_safe_frontend_url("http://example.com"));
    }

    #[test]
    fn rejects_non_http_schemes() {
        assert!(!is_safe_frontend_url("javascript:alert(1)"));
        assert!(!is_safe_frontend_url("data:text/html,<script>x</script>"));
        assert!(!is_safe_frontend_url("file:///etc/passwd"));
        assert!(!is_safe_frontend_url("ftp://example.com"));
    }

    #[test]
    fn rejects_paths_and_queries_and_fragments() {
        assert!(!is_safe_frontend_url("https://example.com/path"));
        assert!(!is_safe_frontend_url("https://example.com/?q=1"));
        assert!(!is_safe_frontend_url("https://example.com/#frag"));
    }

    #[test]
    fn rejects_empty_host() {
        assert!(!is_safe_frontend_url(""));
        assert!(!is_safe_frontend_url("https://"));
        assert!(!is_safe_frontend_url("not a url"));
    }

    #[test]
    fn rejects_control_characters() {
        assert!(!is_safe_frontend_url(
            "https://example.com\r\nLocation: https://evil.com"
        ));
        assert!(!is_safe_frontend_url("https://example.com\n"));
    }
}
