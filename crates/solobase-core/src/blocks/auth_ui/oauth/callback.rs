//! GET /b/auth/oauth/callback — relocated from auth/oauth.rs::handle_oauth_callback
//! in Task 5.

use std::collections::HashMap;

use wafer_core::clients::{config, database as db, network};
use wafer_run::{context::Context, types::Message, OutputStream};

use crate::blocks::{
    auth::{
        helpers::{build_auth_cookie, ensure_admin_role, generate_tokens, store_refresh_token},
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

    // Exchange code for token (URL-encode all values, include PKCE verifier)
    let token_url = spec.token_url;
    let token_body_str = spec.build_token_body(
        code,
        &client_id,
        &client_secret,
        &redirect_uri,
        &code_verifier,
    );

    let mut headers = HashMap::new();
    headers.insert(
        "Content-Type".to_string(),
        "application/x-www-form-urlencoded".to_string(),
    );
    headers.insert("Accept".to_string(), "application/json".to_string());

    let token_body_bytes = token_body_str.into_bytes();
    let token_resp = match network::do_request(
        ctx,
        "POST",
        token_url,
        &headers,
        Some(&token_body_bytes),
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return err_internal("Token exchange failed", e),
    };

    let token_data: serde_json::Value = match serde_json::from_slice(&token_resp.body) {
        Ok(d) => d,
        Err(_) => return err_internal_no_cause("Failed to parse token response"),
    };

    let access_token_oauth = token_data
        .get("access_token")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if access_token_oauth.is_empty() {
        return err_internal_no_cause("No access token in OAuth response");
    }

    // Get user info
    let userinfo_url = spec.userinfo_url;
    let auth_header = spec.userinfo_auth_header(access_token_oauth);

    let mut info_headers = HashMap::new();
    info_headers.insert("Authorization".to_string(), auth_header);
    info_headers.insert("Accept".to_string(), "application/json".to_string());
    // GitHub's REST API rejects requests without a User-Agent header
    // (returns 403 + an HTML error body). Other providers accept it.
    info_headers.insert(
        "User-Agent".to_string(),
        concat!("solobase-auth/", env!("CARGO_PKG_VERSION")).to_string(),
    );

    let info_resp = match network::do_request(ctx, "GET", userinfo_url, &info_headers, None).await {
        Ok(r) => r,
        Err(e) => return err_internal("User info request failed", e),
    };

    let user_info: serde_json::Value = match serde_json::from_slice(&info_resp.body) {
        Ok(d) => d,
        Err(e) => {
            // Log the SHA-256 hash of the body instead of the body itself —
            // a parse failure is rare and the raw body typically contains
            // the upstream email / provider IDs that we don't want to drop
            // into the error log surface.
            let body_hash = crate::blocks::helpers::sha256_hex(&info_resp.body);
            return err_internal(
                "Failed to parse OAuth user info",
                format!(
                    "status={} parse={} body_len={} body_sha256={}",
                    info_resp.status_code,
                    e,
                    info_resp.body.len(),
                    body_hash
                ),
            );
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
        let mut emails_headers = HashMap::new();
        emails_headers.insert(
            "Authorization".to_string(),
            spec.userinfo_auth_header(access_token_oauth),
        );
        emails_headers.insert("Accept".to_string(), "application/json".to_string());
        emails_headers.insert(
            "User-Agent".to_string(),
            concat!("solobase-auth/", env!("CARGO_PKG_VERSION")).to_string(),
        );
        if let Ok(emails_resp) =
            network::do_request(ctx, "GET", emails_url, &emails_headers, None).await
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
        return err_internal_no_cause("No email returned by OAuth provider");
    }

    // Extract the stable provider-side user identifier.
    // GitHub returns `id` as a JSON number; Google returns `sub` (string);
    // Microsoft returns `id` (string). Coerce to string in all cases.
    let provider_ref = {
        // Try `sub` first (Google OIDC), then `id` (GitHub, Microsoft).
        let raw = user_info.get("sub").or_else(|| user_info.get("id"));
        match raw {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            _ => String::new(),
        }
    };
    if provider_ref.is_empty() {
        return err_internal_no_cause("OAuth provider did not return a stable user id");
    }

    // Stable per-provider handle (GitHub `login`, others fall back to email local-part).
    let provider_login = user_info
        .get("login")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| email.split('@').next().unwrap_or(""))
        .to_string();

    // --- Step 1: look up existing link by (provider, provider_ref) ---
    let existing_link =
        match provider_links::find_by_provider_ref(ctx, &provider, &provider_ref).await {
            Ok(l) => l,
            Err(e) => return err_internal("provider_links lookup failed", e),
        };

    // --- Step 2 / 3: resolve user_id ---
    let user_id: String = if let Some(link) = existing_link {
        // Known provider link — reuse the bound user.
        link.user_id
    } else {
        // No link yet. Try email-based account merging.
        match users::find_by_email(ctx, &email).await {
            Ok(Some(existing_user)) => {
                // Check if the existing user account is disabled.
                if existing_user.role == "disabled" {
                    return err_forbidden("Account is disabled");
                }
                // Reuse this account; the upsert below will create the new link.
                existing_user.id
            }
            Ok(None) => {
                // Brand-new user — enforce signup gates.
                let allow_signup =
                    config::get_default(ctx, "SOLOBASE_SHARED__ALLOW_SIGNUP", "true").await;
                if allow_signup != "true" && allow_signup != "1" {
                    return err_forbidden("Signups are currently disabled");
                }

                let allowed_domains =
                    config::get_default(ctx, "SUPPERS_AI__AUTH__ALLOWED_EMAIL_DOMAINS", "").await;
                if !allowed_domains.is_empty() {
                    let email_domain = email.split_once('@').map(|x| x.1).unwrap_or("");
                    let allowed = allowed_domains.split(',').any(|d| d.trim() == email_domain);
                    if !allowed {
                        return err_forbidden("Signups from this email domain are not allowed");
                    }
                }

                // Determine role: admin if email matches bootstrap email.
                let admin_email =
                    config::get_default(ctx, "SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_EMAIL", "")
                        .await;
                let role = if !admin_email.is_empty() && email.eq_ignore_ascii_case(&admin_email) {
                    "admin"
                } else {
                    "user"
                };

                let display_name = if name.is_empty() {
                    email.clone()
                } else {
                    name.clone()
                };
                let new_user = users::NewUser {
                    email: email.clone(),
                    display_name,
                    avatar_url: if avatar.is_empty() {
                        None
                    } else {
                        Some(avatar.clone())
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
                    Err(e) => return err_internal("Failed to create user", e),
                }
            }
            Err(e) => return err_internal("User lookup failed", e),
        }
    };

    // --- Step 4: upsert the provider_links row ---
    if let Err(e) = provider_links::upsert(
        ctx,
        provider_links::NewLink {
            provider: &provider,
            provider_ref: &provider_ref,
            user_id: &user_id,
            provider_login: &provider_login,
            access_token: access_token_oauth,
        },
    )
    .await
    {
        // Log but don't fail — the user is authenticated; link persistence
        // is best-effort metadata. A failed upsert means re-login will
        // fall back to email-based merging on next attempt.
        tracing::warn!("Failed to upsert provider_links: {e}");
    }

    // Step 5: update last_login_at on the users row (best-effort).
    let upd = json_map(serde_json::json!({
        "last_login_at": crate::blocks::helpers::now_rfc3339()
    }));
    if let Err(e) = db::update(ctx, USERS_TABLE, &user_id, upd).await {
        tracing::warn!("Failed to update last_login_at: {e}");
    }

    let roles = ensure_admin_role(ctx, &user_id, &email).await;
    let (jwt_token, refresh_token, family) = match generate_tokens(
        ctx,
        &user_id,
        &email,
        &roles,
        &format!("oauth.{}", provider),
    )
    .await
    {
        Ok(t) => t,
        Err(r) => return r,
    };
    store_refresh_token(ctx, &user_id, &refresh_token, &family, 0).await;

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

    let access_lifetime = crate::blocks::auth::helpers::access_token_lifetime_secs(ctx).await;
    let cookie = build_auth_cookie(&jwt_token, access_lifetime, ctx).await;

    ResponseBuilder::new()
        .status(302)
        .set_cookie(&cookie)
        .set_header("Location", &redirect_url)
        .json(&serde_json::json!({"redirect": redirect_url}))
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
