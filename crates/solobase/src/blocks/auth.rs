use std::collections::HashMap;
use std::time::Duration;
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_core::clients::{crypto, config, network};
use super::helpers::{hex_encode, sha256_hex};
use super::errors::{ErrorCode, error_response};
use super::rate_limit::{UserRateLimiter, RateLimit, set_rate_limit_headers, rate_limited_response};

pub struct AuthBlock {
    limiter: UserRateLimiter,
}

impl AuthBlock {
    pub fn new() -> Self {
        Self { limiter: UserRateLimiter::new() }
    }

    async fn check_rate_limit(&self, ctx: &dyn Context, msg: &mut Message, identity: &str, category: &str, default: RateLimit) -> Option<Result_> {
        let limit = match default.resolve(ctx, category).await {
            Some(l) => l,
            None => return None, // disabled via config
        };
        let key = UserRateLimiter::key(identity, category);
        match self.limiter.check(&key, limit) {
            Ok(remaining) => {
                set_rate_limit_headers(msg, limit.max_requests, remaining);
                None
            }
            Err(retry_after) => Some(rate_limited_response(msg, retry_after)),
        }
    }
}

const USERS_COLLECTION: &str = "auth_users";
const TOKENS_COLLECTION: &str = "auth_tokens";
const API_KEYS_COLLECTION: &str = "api_keys";
const USER_ROLES_COLLECTION: &str = "iam_user_roles";

impl AuthBlock {
    async fn handle_login(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        #[derive(serde::Deserialize)]
        struct LoginReq { email: String, password: String }
        let body: LoginReq = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
        };

        let email_lower = body.email.trim().to_lowercase();

        // Find user by email
        let user = match db::get_by_field(ctx, USERS_COLLECTION, "email", serde_json::Value::String(email_lower.clone())).await {
            Ok(u) => u,
            Err(_) => return error_response(msg, ErrorCode::InvalidCredentials, "Invalid email or password"),
        };

        // Check password
        let stored_hash = user.data.get("password_hash").and_then(|v| v.as_str()).unwrap_or("");
        if crypto::compare_hash(ctx, &body.password, stored_hash).await.is_err() {
            return error_response(msg, ErrorCode::InvalidCredentials, "Invalid email or password");
        }

        // Check if user is disabled
        if let Some(disabled) = user.data.get("disabled") {
            if disabled.as_bool().unwrap_or(false) {
                return error_response(msg, ErrorCode::AccountDisabled, "Account is disabled");
            }
        }

        // Get roles
        let roles = get_user_roles(ctx, &user.id).await;

        // Generate tokens
        let (access_token, refresh_token) = match generate_tokens(ctx, &user.id, &email_lower, &roles).await {
            Ok(t) => t,
            Err(r) => return r,
        };

        // Store refresh token
        store_refresh_token(ctx, &user.id, &refresh_token).await;

        // Update last login
        let mut upd = HashMap::new();
        upd.insert("last_login_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
        if let Err(e) = db::update(ctx, USERS_COLLECTION, &user.id, upd).await {
            tracing::warn!("Failed to update last login time: {e}");
        }

        let cookie = build_auth_cookie(&access_token, 86400, ctx).await;

        ResponseBuilder::new(msg)
            .set_cookie(&cookie)
            .json(&serde_json::json!({
                "access_token": access_token,
                "refresh_token": refresh_token,
                "token_type": "Bearer",
                "expires_in": 86400,
                "user": {
                    "id": user.id,
                    "email": email_lower,
                    "roles": roles,
                    "name": user.data.get("name").and_then(|v| v.as_str()).unwrap_or("")
                }
            }))
    }

    async fn handle_signup(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        #[derive(serde::Deserialize)]
        struct SignupReq { email: String, password: String, name: Option<String> }
        let body: SignupReq = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
        };

        let email_lower = body.email.trim().to_lowercase();
        let parts: Vec<&str> = email_lower.splitn(2, '@').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() || !parts[1].contains('.') {
            return error_response(msg, ErrorCode::InvalidEmail, "Invalid email address");
        }
        if body.password.len() < 8 {
            return error_response(msg, ErrorCode::PasswordTooShort, "Password must be at least 8 characters");
        }
        if body.password.len() > 1024 {
            return error_response(msg, ErrorCode::PasswordTooLong, "Password must not exceed 1024 characters");
        }
        if body.password.chars().any(|c| c.is_control()) {
            return error_response(msg, ErrorCode::InvalidInput, "Password must not contain control characters");
        }
        if email_lower.len() > 255 {
            return error_response(msg, ErrorCode::InvalidEmail, "Email must not exceed 255 characters");
        }
        if let Some(ref name) = body.name {
            if name.len() > 200 {
                return error_response(msg, ErrorCode::InvalidInput, "Name must not exceed 200 characters");
            }
        }

        // Check if user exists
        if db::get_by_field(ctx, USERS_COLLECTION, "email", serde_json::Value::String(email_lower.clone())).await.is_ok() {
            return error_response(msg, ErrorCode::EmailAlreadyExists, "Email already registered");
        }

        // Hash password
        let password_hash = match crypto::hash(ctx, &body.password).await {
            Ok(h) => h,
            Err(e) => return err_internal(msg, &format!("Failed to hash password: {e}")),
        };

        let now = chrono::Utc::now().to_rfc3339();
        let mut data = HashMap::new();
        data.insert("email".to_string(), serde_json::Value::String(email_lower.clone()));
        data.insert("password_hash".to_string(), serde_json::Value::String(password_hash));
        data.insert("name".to_string(), serde_json::Value::String(body.name.unwrap_or_default()));
        data.insert("created_at".to_string(), serde_json::Value::String(now.clone()));
        data.insert("updated_at".to_string(), serde_json::Value::String(now));
        data.insert("disabled".to_string(), serde_json::Value::Bool(false));

        let user = match db::create(ctx, USERS_COLLECTION, data).await {
            Ok(u) => u,
            Err(e) => return err_internal(msg, &format!("Failed to create user: {e}")),
        };

        // Check if this is the very first user by counting users.
        // The user was already created, so count of 1 means this IS the first user.
        let user_count = db::count(ctx, USERS_COLLECTION, &[]).await.unwrap_or(2);
        let default_role = if user_count == 1 { "admin" } else { "user" };
        let mut role_data = HashMap::new();
        role_data.insert("user_id".to_string(), serde_json::Value::String(user.id.clone()));
        role_data.insert("role".to_string(), serde_json::Value::String(default_role.to_string()));
        role_data.insert("assigned_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
        if let Err(e) = db::create(ctx, USER_ROLES_COLLECTION, role_data).await {
            tracing::warn!("Failed to assign default role during signup: {e}");
        }

        let roles = vec![default_role.to_string()];

        // Generate tokens
        let (access_token, refresh_token) = match generate_tokens(ctx, &user.id, &email_lower, &roles).await {
            Ok(t) => t,
            Err(r) => return r,
        };

        store_refresh_token(ctx, &user.id, &refresh_token).await;

        let cookie = build_auth_cookie(&access_token, 86400, ctx).await;

        ResponseBuilder::new(msg).status(201)
            .set_cookie(&cookie)
            .json(&serde_json::json!({
                "access_token": access_token,
                "refresh_token": refresh_token,
                "token_type": "Bearer",
                "expires_in": 86400,
                "user": {
                    "id": user.id,
                    "email": email_lower,
                    "roles": roles,
                    "name": user.data.get("name").and_then(|v| v.as_str()).unwrap_or("")
                }
            }))
    }

    async fn handle_refresh(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        #[derive(serde::Deserialize)]
        struct RefreshReq { refresh_token: String }
        let body: RefreshReq = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
        };

        // Verify refresh token
        let claims = match crypto::verify(ctx, &body.refresh_token).await {
            Ok(c) => c,
            Err(_) => return error_response(msg, ErrorCode::InvalidToken, "Invalid or expired refresh token"),
        };

        let user_id = claims.get("user_id")
            .or_else(|| claims.get("sub"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if user_id.is_empty() {
            return error_response(msg, ErrorCode::InvalidToken, "Invalid refresh token");
        }

        let token_type = claims.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if token_type != "refresh" {
            return error_response(msg, ErrorCode::InvalidToken, "Not a refresh token");
        }

        // Get user
        let user = match db::get(ctx, USERS_COLLECTION, &user_id).await {
            Ok(u) => u,
            Err(_) => return error_response(msg, ErrorCode::NotAuthenticated, "User not found"),
        };

        let email = user.data.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let roles = get_user_roles(ctx, &user_id).await;

        // Revoke old refresh token family and issue new
        let family = claims.get("family").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if !family.is_empty() {
            db::delete_by_field(ctx, TOKENS_COLLECTION, "family", serde_json::Value::String(family)).await.ok();
        }

        let (access_token, refresh_token) = match generate_tokens(ctx, &user_id, &email, &roles).await {
            Ok(t) => t,
            Err(r) => return r,
        };

        store_refresh_token(ctx, &user_id, &refresh_token).await;

        let cookie = build_auth_cookie(&access_token, 86400, ctx).await;

        ResponseBuilder::new(msg)
            .set_cookie(&cookie)
            .json(&serde_json::json!({
                "access_token": access_token,
                "refresh_token": refresh_token,
                "token_type": "Bearer",
                "expires_in": 86400
            }))
    }

    async fn handle_logout(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let user_id = msg.user_id();
        if !user_id.is_empty() {
            db::delete_by_field(ctx, TOKENS_COLLECTION, "user_id", serde_json::Value::String(user_id.to_string())).await.ok();
        }

        let cookie = build_auth_cookie("", 0, ctx).await;
        ResponseBuilder::new(msg)
            .set_cookie(&cookie)
            .json(&serde_json::json!({"message": "Logged out successfully"}))
    }

    async fn handle_me_get(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let user_id = msg.user_id();
        if user_id.is_empty() {
            return error_response(msg, ErrorCode::NotAuthenticated, "Not authenticated");
        }
        let user = match db::get(ctx, USERS_COLLECTION, user_id).await {
            Ok(u) => u,
            Err(_) => return err_not_found(msg, "User not found"),
        };
        let roles = get_user_roles(ctx, user_id).await;
        json_respond(msg, &serde_json::json!({
            "user": {
                "id": user.id,
                "email": user.data.get("email").and_then(|v| v.as_str()).unwrap_or(""),
                "name": user.data.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                "roles": roles,
                "created_at": user.data.get("created_at").and_then(|v| v.as_str()).unwrap_or(""),
                "avatar_url": user.data.get("avatar_url").and_then(|v| v.as_str()).unwrap_or("")
            }
        }))
    }

    async fn handle_me_update(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let user_id = msg.user_id();
        if user_id.is_empty() {
            return error_response(msg, ErrorCode::NotAuthenticated, "Not authenticated");
        }

        let body: HashMap<String, serde_json::Value> = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
        };

        // Only allow updating certain fields
        let mut data = HashMap::new();
        for key in &["name", "avatar_url"] {
            if let Some(val) = body.get(*key) {
                data.insert(key.to_string(), val.clone());
            }
        }
        data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

        match db::update(ctx, USERS_COLLECTION, user_id, data).await {
            Ok(user) => {
                let roles = get_user_roles(ctx, user_id).await;
                json_respond(msg, &serde_json::json!({
                    "id": user.id,
                    "email": user.data.get("email").and_then(|v| v.as_str()).unwrap_or(""),
                    "name": user.data.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                    "roles": roles
                }))
            }
            Err(e) => err_internal(msg, &format!("Update failed: {e}")),
        }
    }

    async fn handle_change_password(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let user_id = msg.user_id();
        if user_id.is_empty() {
            return error_response(msg, ErrorCode::NotAuthenticated, "Not authenticated");
        }

        #[derive(serde::Deserialize)]
        struct ChangePwReq { current_password: String, new_password: String }
        let body: ChangePwReq = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
        };

        if body.new_password.len() < 8 {
            return error_response(msg, ErrorCode::PasswordTooShort, "New password must be at least 8 characters");
        }
        if body.new_password.len() > 1024 {
            return error_response(msg, ErrorCode::PasswordTooLong, "Password must not exceed 1024 characters");
        }

        let user = match db::get(ctx, USERS_COLLECTION, user_id).await {
            Ok(u) => u,
            Err(_) => return err_not_found(msg, "User not found"),
        };

        let stored_hash = user.data.get("password_hash").and_then(|v| v.as_str()).unwrap_or("");
        if crypto::compare_hash(ctx, &body.current_password, stored_hash).await.is_err() {
            return error_response(msg, ErrorCode::InvalidCredentials, "Current password is incorrect");
        }

        let new_hash = match crypto::hash(ctx, &body.new_password).await {
            Ok(h) => h,
            Err(e) => return err_internal(msg, &format!("Hash failed: {e}")),
        };

        let mut data = HashMap::new();
        data.insert("password_hash".to_string(), serde_json::Value::String(new_hash));
        data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

        match db::update(ctx, USERS_COLLECTION, user_id, data).await {
            Ok(_) => json_respond(msg, &serde_json::json!({"message": "Password changed successfully"})),
            Err(e) => err_internal(msg, &format!("Update failed: {e}")),
        }
    }

    // --- API Key Management ---

    async fn handle_api_keys_list(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let user_id = msg.user_id();
        let opts = ListOptions {
            filters: vec![Filter {
                field: "user_id".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String(user_id.to_string()),
            }],
            sort: vec![SortField { field: "created_at".to_string(), desc: true }],
            limit: 100,
            ..Default::default()
        };
        match db::list(ctx, API_KEYS_COLLECTION, &opts).await {
            Ok(mut result) => {
                // Strip key_hash from response
                for record in &mut result.records {
                    record.data.remove("key_hash");
                }
                json_respond(msg, &result)
            }
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    async fn handle_api_keys_create(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let user_id = msg.user_id();

        #[derive(serde::Deserialize)]
        struct CreateKeyReq { name: String, expires_at: Option<String> }
        let body: CreateKeyReq = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
        };

        // Generate random key
        let random_bytes = match crypto::random_bytes(ctx, 24).await {
            Ok(b) => b,
            Err(e) => return err_internal(msg, &format!("Failed to generate key: {e}")),
        };
        let key_string = format!("sb_{}", hex_encode(&random_bytes));

        // Use deterministic SHA-256 hash for key lookup (not argon2, which is non-deterministic)
        let key_hash = sha256_hex(key_string.as_bytes());

        let now = chrono::Utc::now().to_rfc3339();
        let mut data = HashMap::new();
        data.insert("user_id".to_string(), serde_json::Value::String(user_id.to_string()));
        data.insert("name".to_string(), serde_json::Value::String(body.name));
        data.insert("key_hash".to_string(), serde_json::Value::String(key_hash));
        data.insert("key_prefix".to_string(), serde_json::Value::String(key_string[..10].to_string()));
        data.insert("created_at".to_string(), serde_json::Value::String(now));
        if let Some(exp) = body.expires_at {
            data.insert("expires_at".to_string(), serde_json::Value::String(exp));
        }

        match db::create(ctx, API_KEYS_COLLECTION, data).await {
            Ok(record) => json_respond(msg, &serde_json::json!({
                "id": record.id,
                "key": key_string,
                "name": record.data.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                "key_prefix": record.data.get("key_prefix").and_then(|v| v.as_str()).unwrap_or(""),
                "message": "Save this key — it won't be shown again"
            })),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    async fn handle_api_keys_revoke(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path();
        let id = path.strip_prefix("/auth/api-keys/").unwrap_or("");
        if id.is_empty() {
            return err_bad_request(msg, "Missing key ID");
        }
        let user_id = msg.user_id();

        // Verify ownership
        let key = match db::get(ctx, API_KEYS_COLLECTION, id).await {
            Ok(k) => k,
            Err(_) => return err_not_found(msg, "API key not found"),
        };
        let key_owner = key.data.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
        if key_owner != user_id && !msg.is_admin() {
            return err_forbidden(msg, "Cannot revoke another user's API key");
        }

        let mut data = HashMap::new();
        data.insert("revoked_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
        match db::update(ctx, API_KEYS_COLLECTION, id, data).await {
            Ok(_) => json_respond(msg, &serde_json::json!({"message": "API key revoked"})),
            Err(e) => err_internal(msg, &format!("Database error: {e}")),
        }
    }

    // --- OAuth ---

    async fn handle_oauth_providers(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
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

    async fn handle_oauth_login(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
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

    async fn handle_oauth_callback(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
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
                let mut upd = HashMap::new();
                upd.insert("last_login_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                if !name.is_empty() { upd.insert("name".to_string(), serde_json::Value::String(name.clone())); }
                if !avatar.is_empty() { upd.insert("avatar_url".to_string(), serde_json::Value::String(avatar.clone())); }
                upd.insert("oauth_provider".to_string(), serde_json::Value::String(provider.to_string()));
                if let Err(e) = db::update(ctx, USERS_COLLECTION, &existing.id, upd).await {
                    tracing::warn!("Failed to update OAuth user profile: {e}");
                }
                existing
            }
            Err(_) => {
                let now = chrono::Utc::now().to_rfc3339();
                let mut data = HashMap::new();
                data.insert("email".to_string(), serde_json::Value::String(email.clone()));
                data.insert("name".to_string(), serde_json::Value::String(name.clone()));
                data.insert("avatar_url".to_string(), serde_json::Value::String(avatar));
                data.insert("oauth_provider".to_string(), serde_json::Value::String(provider.to_string()));
                data.insert("created_at".to_string(), serde_json::Value::String(now.clone()));
                data.insert("updated_at".to_string(), serde_json::Value::String(now));
                data.insert("disabled".to_string(), serde_json::Value::Bool(false));
                match db::create(ctx, USERS_COLLECTION, data).await {
                    Ok(u) => {
                        // Check if this is the very first user by counting users.
                        // The user was already created, so count of 1 means this IS the first user.
                        let user_count = db::count(ctx, USERS_COLLECTION, &[]).await.unwrap_or(2);
                        let role = if user_count == 1 { "admin" } else { "user" };
                        let mut role_data = HashMap::new();
                        role_data.insert("user_id".to_string(), serde_json::Value::String(u.id.clone()));
                        role_data.insert("role".to_string(), serde_json::Value::String(role.to_string()));
                        role_data.insert("assigned_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                        if let Err(e) = db::create(ctx, USER_ROLES_COLLECTION, role_data).await {
                            tracing::warn!("Failed to assign default role during OAuth signup: {e}");
                        }
                        u
                    }
                    Err(e) => return err_internal(msg, &format!("Failed to create user: {e}")),
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

    async fn handle_sync_user(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        // Internal endpoint for OAuth user sync — requires INTERNAL_SECRET
        let expected_secret = config::get_default(ctx, "INTERNAL_SECRET", "").await;
        if expected_secret.is_empty() {
            return err_forbidden(msg, "INTERNAL_SECRET not configured — internal endpoints are disabled");
        }
        let provided_secret = msg.header("x-internal-secret");
        if provided_secret != expected_secret {
            return err_unauthorized(msg, "Invalid internal secret");
        }

        #[derive(serde::Deserialize)]
        struct SyncReq { email: String, name: Option<String>, provider: Option<String> }
        let body: SyncReq = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
        };

        let email_lower = body.email.trim().to_lowercase();
        let user = match db::get_by_field(ctx, USERS_COLLECTION, "email", serde_json::Value::String(email_lower.clone())).await {
            Ok(u) => u,
            Err(_) => {
                let now = chrono::Utc::now().to_rfc3339();
                let mut data = HashMap::new();
                data.insert("email".to_string(), serde_json::Value::String(email_lower));
                data.insert("name".to_string(), serde_json::Value::String(body.name.unwrap_or_default()));
                data.insert("oauth_provider".to_string(), serde_json::Value::String(body.provider.unwrap_or_default()));
                data.insert("created_at".to_string(), serde_json::Value::String(now.clone()));
                data.insert("updated_at".to_string(), serde_json::Value::String(now));
                data.insert("disabled".to_string(), serde_json::Value::Bool(false));
                match db::create(ctx, USERS_COLLECTION, data).await {
                    Ok(u) => u,
                    Err(e) => return err_internal(msg, &format!("Create failed: {e}")),
                }
            }
        };

        json_respond(msg, &serde_json::json!({"id": user.id, "email": user.data.get("email")}))
    }
}

// --- Helper functions ---

async fn get_user_roles(ctx: &dyn Context, user_id: &str) -> Vec<String> {
    let opts = ListOptions {
        filters: vec![Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.to_string()),
        }],
        ..Default::default()
    };
    match db::list(ctx, USER_ROLES_COLLECTION, &opts).await {
        Ok(r) => r.records.iter()
            .filter_map(|rec| rec.data.get("role").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect(),
        Err(_) => Vec::new(),
    }
}

async fn generate_tokens(
    ctx: &dyn Context,
    user_id: &str,
    email: &str,
    roles: &[String],
) -> Result<(String, String), Result_> {
    let family = match crypto::random_bytes(ctx, 16).await {
        Ok(bytes) => hex_encode(&bytes),
        Err(e) => return Err(Result_::error(e)),
    };

    let mut access_claims = HashMap::new();
    access_claims.insert("user_id".to_string(), serde_json::Value::String(user_id.to_string()));
    access_claims.insert("sub".to_string(), serde_json::Value::String(user_id.to_string()));
    access_claims.insert("email".to_string(), serde_json::Value::String(email.to_string()));
    access_claims.insert("roles".to_string(), serde_json::json!(roles));
    access_claims.insert("type".to_string(), serde_json::Value::String("access".to_string()));

    let access_token = crypto::sign(ctx, &access_claims, Duration::from_secs(86400)).await
        .map_err(|e| Result_::error(e))?;

    let mut refresh_claims = HashMap::new();
    refresh_claims.insert("user_id".to_string(), serde_json::Value::String(user_id.to_string()));
    refresh_claims.insert("sub".to_string(), serde_json::Value::String(user_id.to_string()));
    refresh_claims.insert("type".to_string(), serde_json::Value::String("refresh".to_string()));
    refresh_claims.insert("family".to_string(), serde_json::Value::String(family));

    let refresh_token = crypto::sign(ctx, &refresh_claims, Duration::from_secs(604800)).await
        .map_err(|e| Result_::error(e))?;

    Ok((access_token, refresh_token))
}

async fn store_refresh_token(ctx: &dyn Context, user_id: &str, token: &str) {
    let mut data = HashMap::new();
    data.insert("user_id".to_string(), serde_json::Value::String(user_id.to_string()));
    data.insert("token".to_string(), serde_json::Value::String(token.to_string()));
    data.insert("created_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    if let Err(e) = db::create(ctx, TOKENS_COLLECTION, data).await {
        tracing::warn!("Failed to store refresh token: {e}");
    }
}

async fn build_auth_cookie(token: &str, max_age: u64, ctx: &dyn Context) -> String {
    let env = config::get_default(ctx, "ENVIRONMENT", "development").await;
    let secure = env == "production";
    format!(
        "auth_token={}; HttpOnly; Path=/; SameSite=Lax; Max-Age={}{}",
        token, max_age, if secure { "; Secure" } else { "" }
    )
}

fn urlencode(s: &str) -> String {
    s.as_bytes().iter().map(|&b| match b {
        b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
            String::from(b as char)
        }
        _ => format!("%{:02X}", b),
    }).collect()
}

/// Seed a default admin user if no users exist yet.
pub async fn seed_admin_user(ctx: &dyn Context) {
    let count = db::count(ctx, USERS_COLLECTION, &[]).await.unwrap_or(0);
    if count > 0 {
        return;
    }

    let admin_email = config::get_default(ctx, "ADMIN_EMAIL", "admin@example.com").await;
    let admin_password_env = config::get_default(ctx, "ADMIN_PASSWORD", "").await;

    let (password_to_use, was_randomly_generated) = if !admin_password_env.is_empty() {
        (admin_password_env, false)
    } else {
        let random_bytes = match crypto::random_bytes(ctx, 16).await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Failed to generate random password: {e}");
                return;
            }
        };
        (hex_encode(&random_bytes), true)
    };

    let password_hash = match crypto::hash(ctx, &password_to_use).await {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to hash default admin password: {e}");
            return;
        }
    };

    let now = chrono::Utc::now().to_rfc3339();
    let mut data = HashMap::new();
    data.insert("email".to_string(), serde_json::Value::String(admin_email.clone()));
    data.insert("password_hash".to_string(), serde_json::Value::String(password_hash));
    data.insert("name".to_string(), serde_json::Value::String("Admin".to_string()));
    data.insert("created_at".to_string(), serde_json::Value::String(now.clone()));
    data.insert("updated_at".to_string(), serde_json::Value::String(now));
    data.insert("disabled".to_string(), serde_json::Value::Bool(false));

    match db::create(ctx, USERS_COLLECTION, data).await {
        Ok(user) => {
            let mut role_data = HashMap::new();
            role_data.insert("user_id".to_string(), serde_json::Value::String(user.id.clone()));
            role_data.insert("role".to_string(), serde_json::Value::String("admin".to_string()));
            role_data.insert("assigned_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            if let Err(e) = db::create(ctx, USER_ROLES_COLLECTION, role_data).await {
                tracing::warn!("Failed to assign admin role to seeded user: {e}");
            }
            tracing::info!("==========================================================");
            tracing::info!("Default admin user seeded:");
            tracing::info!("  Email:    {}", admin_email);
            if was_randomly_generated {
                tracing::info!("  Password: {}", password_to_use);
                tracing::info!("  CHANGE THIS PASSWORD IMMEDIATELY!");
            } else {
                tracing::info!("  Password: (set via ADMIN_PASSWORD env var)");
            }
            tracing::info!("==========================================================");
        }
        Err(e) => {
            tracing::error!("Failed to seed admin user: {e}");
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for AuthBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "@solobase/auth".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "Authentication: login, signup, JWT, refresh tokens, OAuth, API keys".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
            runtime: wafer_run::types::BlockRuntime::Native,
            requires: Vec::new(),
        }
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let action = msg.action().to_string();
        let path = msg.path().to_string();

        // Apply per-user/IP rate limiting based on endpoint category
        match (action.as_str(), path.as_str()) {
            // Unauthenticated sensitive endpoints: rate limit by IP
            ("create", "/auth/login") | ("create", "/auth/signup") => {
                let ip = msg.remote_addr().to_string();
                let identity = if ip.is_empty() { "unknown".to_string() } else { ip };
                if let Some(r) = self.check_rate_limit(ctx, msg, &identity, "auth", RateLimit::AUTH).await {
                    return r;
                }
            }
            ("create", "/auth/refresh") => {
                let ip = msg.remote_addr().to_string();
                let identity = if ip.is_empty() { "unknown".to_string() } else { ip };
                if let Some(r) = self.check_rate_limit(ctx, msg, &identity, "refresh", RateLimit::REFRESH).await {
                    return r;
                }
            }
            // Authenticated write endpoints: rate limit by user_id
            ("update", _) | ("create", "/auth/change-password") | ("create", "/auth/api-keys") | ("delete", _) => {
                let user_id = msg.user_id().to_string();
                if !user_id.is_empty() {
                    if let Some(r) = self.check_rate_limit(ctx, msg, &user_id, "auth_write", RateLimit::API_WRITE).await {
                        return r;
                    }
                }
            }
            // Authenticated read endpoints: rate limit by user_id
            ("retrieve", "/auth/me") | ("retrieve", "/auth/api-keys") => {
                let user_id = msg.user_id().to_string();
                if !user_id.is_empty() {
                    if let Some(r) = self.check_rate_limit(ctx, msg, &user_id, "auth_read", RateLimit::API_READ).await {
                        return r;
                    }
                }
            }
            _ => {}
        }

        match (action.as_str(), path.as_str()) {
            ("create", "/auth/login") => self.handle_login(ctx, msg).await,
            ("create", "/auth/signup") => self.handle_signup(ctx, msg).await,
            ("create", "/auth/refresh") => self.handle_refresh(ctx, msg).await,
            ("create", "/auth/logout") => self.handle_logout(ctx, msg).await,
            ("retrieve", "/auth/me") => self.handle_me_get(ctx, msg).await,
            ("update", "/auth/me") => self.handle_me_update(ctx, msg).await,
            ("create", "/auth/change-password") => self.handle_change_password(ctx, msg).await,
            // API keys
            ("retrieve", "/auth/api-keys") => self.handle_api_keys_list(ctx, msg).await,
            ("create", "/auth/api-keys") => self.handle_api_keys_create(ctx, msg).await,
            ("delete", _) if path.starts_with("/auth/api-keys/") => self.handle_api_keys_revoke(ctx, msg).await,
            // OAuth
            ("retrieve", "/auth/oauth/providers") => self.handle_oauth_providers(ctx, msg).await,
            ("retrieve", "/auth/oauth/login") => self.handle_oauth_login(ctx, msg).await,
            ("retrieve", "/auth/oauth/callback") => self.handle_oauth_callback(ctx, msg).await,
            // Internal
            ("create", "/internal/oauth/sync-user") => self.handle_sync_user(ctx, msg).await,
            _ => err_not_found(msg, "not found"),
        }
    }

    async fn lifecycle(&self, ctx: &dyn Context, event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        if matches!(event.event_type, LifecycleType::Init) {
            seed_admin_user(ctx).await;
        }
        Ok(())
    }
}
