use std::collections::HashMap;
use std::time::Duration;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::{crypto, config, network};
use super::helpers::*;
use super::{AuthBlock, USERS_COLLECTION};
use crate::blocks::helpers::json_map;

impl AuthBlock {
    pub(super) async fn handle_oauth_providers(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let mut providers = Vec::new();

        for provider_name in &["google", "github", "microsoft"] {
            let client_id_key = format!("OAUTH_{}_CLIENT_ID", provider_name.to_uppercase());
            if config::get(ctx, &client_id_key).await.is_ok() {
                providers.push(serde_json::json!({
                    "name": provider_name,
                    "enabled": true
                }));
            }
        }

        json_respond(msg, &serde_json::json!({"providers": providers}))
    }

    pub(super) async fn handle_oauth_login(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let provider = msg.query("provider");
        if provider.is_empty() {
            return err_bad_request(msg, "Missing provider parameter");
        }

        let client_id_key = format!("OAUTH_{}_CLIENT_ID", provider.to_uppercase());
        let client_id = match config::get(ctx, &client_id_key).await {
            Ok(id) => id,
            Err(_) => return err_bad_request(msg, &format!("OAuth provider '{}' not configured", provider)),
        };

        let redirect_uri = config::get_default(ctx, "OAUTH_REDIRECT_URI", "http://localhost:8090/auth/oauth/callback").await;

        // Generate CSRF state token (signed JWT containing the provider name)
        let mut state_claims = HashMap::new();
        state_claims.insert("provider".to_string(), serde_json::Value::String(provider.to_string()));
        state_claims.insert("type".to_string(), serde_json::Value::String("oauth_state".to_string()));
        let state = match crypto::sign(ctx, &state_claims, Duration::from_secs(600)).await {
            Ok(s) => s,
            Err(e) => return err_internal(msg, &format!("Failed to generate state: {e}")),
        };

        let auth_url = match provider {
            "google" => format!(
                "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&state={}",
                client_id, redirect_uri, urlencode(&state)
            ),
            "github" => format!(
                "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=user:email&state={}",
                client_id, redirect_uri, urlencode(&state)
            ),
            "microsoft" => format!(
                "https://login.microsoftonline.com/common/oauth2/v2.0/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&state={}",
                client_id, redirect_uri, urlencode(&state)
            ),
            _ => return err_bad_request(msg, &format!("Unsupported provider: {}", provider)),
        };

        json_respond(msg, &serde_json::json!({
            "auth_url": auth_url,
            "provider": provider
        }))
    }

    pub(super) async fn handle_oauth_callback(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let code = msg.query("code");
        let state = msg.query("state");
        if code.is_empty() || state.is_empty() {
            return err_bad_request(msg, "Missing code or state parameter");
        }

        // Verify CSRF state token and extract provider name
        let state_claims = match crypto::verify(ctx, state).await {
            Ok(c) => c,
            Err(_) => return err_bad_request(msg, "Invalid or expired OAuth state"),
        };
        let state_type = state_claims.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if state_type != "oauth_state" {
            return err_bad_request(msg, "Invalid OAuth state token");
        }
        let provider = state_claims.get("provider").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if provider.is_empty() {
            return err_bad_request(msg, "Missing provider in OAuth state");
        }

        let client_id = config::get_default(ctx, &format!("OAUTH_{}_CLIENT_ID", provider.to_uppercase()), "").await;
        let client_secret = config::get_default(ctx, &format!("OAUTH_{}_CLIENT_SECRET", provider.to_uppercase()), "").await;
        let redirect_uri = config::get_default(ctx, "OAUTH_REDIRECT_URI", "http://localhost:8090/auth/oauth/callback").await;

        if client_id.is_empty() || client_secret.is_empty() {
            return err_internal(msg, "OAuth provider not fully configured");
        }

        // Exchange code for token (URL-encode all values)
        let (token_url, token_body_str) = match provider.as_str() {
            "google" => (
                "https://oauth2.googleapis.com/token".to_string(),
                format!("code={}&client_id={}&client_secret={}&redirect_uri={}&grant_type=authorization_code",
                    urlencode(code), urlencode(&client_id), urlencode(&client_secret), urlencode(&redirect_uri)),
            ),
            "github" => (
                "https://github.com/login/oauth/access_token".to_string(),
                format!("code={}&client_id={}&client_secret={}&redirect_uri={}",
                    urlencode(code), urlencode(&client_id), urlencode(&client_secret), urlencode(&redirect_uri)),
            ),
            _ => return err_bad_request(msg, "Unsupported OAuth provider"),
        };

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
        headers.insert("Accept".to_string(), "application/json".to_string());

        let token_body_bytes = token_body_str.into_bytes();
        let token_resp = match network::do_request(ctx, "POST", &token_url, &headers, Some(&token_body_bytes)).await {
            Ok(r) => r,
            Err(e) => return err_internal(msg, &format!("Token exchange failed: {e}")),
        };

        let token_data: serde_json::Value = match serde_json::from_slice(&token_resp.body) {
            Ok(d) => d,
            Err(_) => return err_internal(msg, "Failed to parse token response"),
        };

        let access_token_oauth = token_data.get("access_token").and_then(|v| v.as_str()).unwrap_or("");
        if access_token_oauth.is_empty() {
            return err_internal(msg, "No access token in OAuth response");
        }

        // Get user info
        let (userinfo_url, auth_header) = match provider.as_str() {
            "google" => ("https://www.googleapis.com/oauth2/v2/userinfo".to_string(), format!("Bearer {}", access_token_oauth)),
            "github" => ("https://api.github.com/user".to_string(), format!("token {}", access_token_oauth)),
            _ => return err_internal(msg, "Unsupported provider"),
        };

        let mut info_headers = HashMap::new();
        info_headers.insert("Authorization".to_string(), auth_header);
        info_headers.insert("Accept".to_string(), "application/json".to_string());

        let info_resp = match network::do_request(ctx, "GET", &userinfo_url, &info_headers, None).await {
            Ok(r) => r,
            Err(e) => return err_internal(msg, &format!("User info request failed: {e}")),
        };

        let user_info: serde_json::Value = match serde_json::from_slice(&info_resp.body) {
            Ok(d) => d,
            Err(_) => return err_internal(msg, "Failed to parse user info"),
        };

        let email = user_info.get("email").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
        let name = user_info.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let avatar = user_info.get("picture")
            .or_else(|| user_info.get("avatar_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if email.is_empty() {
            return err_internal(msg, "No email returned by OAuth provider");
        }

        // Upsert user
        let user = match db::get_by_field(ctx, USERS_COLLECTION, "email", serde_json::Value::String(email.clone())).await {
            Ok(existing) => {
                let mut upd = json_map(serde_json::json!({
                    "last_login_at": crate::blocks::helpers::now_rfc3339(),
                    "oauth_provider": provider
                }));
                if !name.is_empty() { upd.insert("name".to_string(), serde_json::Value::String(name.clone())); }
                if !avatar.is_empty() { upd.insert("avatar_url".to_string(), serde_json::Value::String(avatar.clone())); }
                if let Err(e) = db::update(ctx, USERS_COLLECTION, &existing.id, upd).await {
                    tracing::warn!("Failed to update OAuth user profile: {e}");
                }
                existing
            }
            Err(_) => {
                let mut data = json_map(serde_json::json!({
                    "email": email,
                    "name": name,
                    "avatar_url": avatar,
                    "oauth_provider": provider,
                    "disabled": false
                }));
                crate::blocks::helpers::stamp_created(&mut data);
                match create_user_and_assign_role(ctx, data).await {
                    Ok((u, _)) => u,
                    Err(e) => return err_internal(msg, &e),
                }
            }
        };

        let roles = get_user_roles(ctx, &user.id).await;
        let (jwt_token, refresh_token) = match generate_tokens(ctx, &user.id, &email, &roles).await {
            Ok(t) => t,
            Err(r) => return r,
        };
        store_refresh_token(ctx, &user.id, &refresh_token).await;

        // Redirect to frontend with token
        let frontend_url = config::get_default(ctx, "FRONTEND_URL", "http://localhost:5173").await;
        let redirect_url = format!("{}/?token={}", frontend_url, jwt_token);

        let cookie = build_auth_cookie(&jwt_token, 86400, ctx).await;

        ResponseBuilder::new(msg).status(302)
            .set_cookie(&cookie)
            .set_header("Location", &redirect_url)
            .json(&serde_json::json!({"redirect": redirect_url}))
    }
}
