use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_run::services::database::{self, DatabaseService, Filter, FilterOp, ListOptions, SortField};
use wafer_run::services::crypto::CryptoService;

pub struct AuthBlock;

const USERS_COLLECTION: &str = "auth_users";
const TOKENS_COLLECTION: &str = "auth_tokens";
const API_KEYS_COLLECTION: &str = "api_keys";
const USER_ROLES_COLLECTION: &str = "iam_user_roles";

impl AuthBlock {
    fn handle_login(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let (db, crypto) = match get_services(ctx) {
            Ok(s) => s,
            Err(r) => return r,
        };

        #[derive(serde::Deserialize)]
        struct LoginReq { email: String, password: String }
        let body: LoginReq = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
        };

        let email_lower = body.email.trim().to_lowercase();

        // Find user by email
        let user = match database::get_by_field(db.as_ref(), USERS_COLLECTION, "email", serde_json::Value::String(email_lower.clone())) {
            Ok(u) => u,
            Err(_) => return err_unauthorized(msg.clone(), "Invalid email or password"),
        };

        // Check password
        let stored_hash = user.data.get("password_hash").and_then(|v| v.as_str()).unwrap_or("");
        if crypto.compare_hash(&body.password, stored_hash).is_err() {
            return err_unauthorized(msg.clone(), "Invalid email or password");
        }

        // Check if user is disabled
        if let Some(disabled) = user.data.get("disabled") {
            if disabled.as_bool().unwrap_or(false) {
                return err_forbidden(msg.clone(), "Account is disabled");
            }
        }

        // Get roles
        let roles = get_user_roles(db.as_ref(), &user.id);

        // Generate tokens
        let (access_token, refresh_token) = match generate_tokens(crypto.as_ref(), &user.id, &email_lower, &roles) {
            Ok(t) => t,
            Err(r) => return r,
        };

        // Store refresh token
        store_refresh_token(db.as_ref(), &user.id, &refresh_token);

        // Update last login
        let mut upd = HashMap::new();
        upd.insert("last_login_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
        let _ = db.update(USERS_COLLECTION, &user.id, upd);

        let cookie = format!(
            "auth_token={}; HttpOnly; Path=/; SameSite=Lax; Max-Age=86400",
            access_token
        );

        ResponseBuilder::new(msg.clone(), 200)
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

    fn handle_signup(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let (db, crypto) = match get_services(ctx) {
            Ok(s) => s,
            Err(r) => return r,
        };

        #[derive(serde::Deserialize)]
        struct SignupReq { email: String, password: String, name: Option<String> }
        let body: SignupReq = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
        };

        let email_lower = body.email.trim().to_lowercase();
        if email_lower.is_empty() || !email_lower.contains('@') {
            return err_bad_request(msg.clone(), "Invalid email address");
        }
        if body.password.len() < 8 {
            return err_bad_request(msg.clone(), "Password must be at least 8 characters");
        }

        // Check if user exists
        if database::get_by_field(db.as_ref(), USERS_COLLECTION, "email", serde_json::Value::String(email_lower.clone())).is_ok() {
            return err_conflict(msg.clone(), "Email already registered");
        }

        // Hash password
        let password_hash = match crypto.hash(&body.password) {
            Ok(h) => h,
            Err(e) => return err_internal(msg.clone(), &format!("Failed to hash password: {e}")),
        };

        let now = chrono::Utc::now().to_rfc3339();
        let mut data = HashMap::new();
        data.insert("email".to_string(), serde_json::Value::String(email_lower.clone()));
        data.insert("password_hash".to_string(), serde_json::Value::String(password_hash));
        data.insert("name".to_string(), serde_json::Value::String(body.name.unwrap_or_default()));
        data.insert("created_at".to_string(), serde_json::Value::String(now.clone()));
        data.insert("updated_at".to_string(), serde_json::Value::String(now));
        data.insert("disabled".to_string(), serde_json::Value::Bool(false));

        let user = match db.create(USERS_COLLECTION, data) {
            Ok(u) => u,
            Err(e) => return err_internal(msg.clone(), &format!("Failed to create user: {e}")),
        };

        // Assign default role. First user gets "admin".
        let user_count = db.count(USERS_COLLECTION, &[]).unwrap_or(1);
        let default_role = if user_count <= 1 { "admin" } else { "user" };
        let mut role_data = HashMap::new();
        role_data.insert("user_id".to_string(), serde_json::Value::String(user.id.clone()));
        role_data.insert("role".to_string(), serde_json::Value::String(default_role.to_string()));
        role_data.insert("assigned_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
        let _ = db.create(USER_ROLES_COLLECTION, role_data);

        let roles = vec![default_role.to_string()];

        // Generate tokens
        let (access_token, refresh_token) = match generate_tokens(crypto.as_ref(), &user.id, &email_lower, &roles) {
            Ok(t) => t,
            Err(r) => return r,
        };

        store_refresh_token(db.as_ref(), &user.id, &refresh_token);

        let cookie = format!(
            "auth_token={}; HttpOnly; Path=/; SameSite=Lax; Max-Age=86400",
            access_token
        );

        ResponseBuilder::new(msg.clone(), 201)
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

    fn handle_refresh(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let (db, crypto) = match get_services(ctx) {
            Ok(s) => s,
            Err(r) => return r,
        };

        #[derive(serde::Deserialize)]
        struct RefreshReq { refresh_token: String }
        let body: RefreshReq = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
        };

        // Verify refresh token
        let claims = match crypto.verify(&body.refresh_token) {
            Ok(c) => c,
            Err(_) => return err_unauthorized(msg.clone(), "Invalid or expired refresh token"),
        };

        let user_id = claims.get("user_id")
            .or_else(|| claims.get("sub"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if user_id.is_empty() {
            return err_unauthorized(msg.clone(), "Invalid refresh token");
        }

        let token_type = claims.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if token_type != "refresh" {
            return err_unauthorized(msg.clone(), "Not a refresh token");
        }

        // Get user
        let user = match db.get(USERS_COLLECTION, &user_id) {
            Ok(u) => u,
            Err(_) => return err_unauthorized(msg.clone(), "User not found"),
        };

        let email = user.data.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let roles = get_user_roles(db.as_ref(), &user_id);

        // Revoke old refresh token family and issue new
        let family = claims.get("family").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if !family.is_empty() {
            database::delete_by_field(db.as_ref(), TOKENS_COLLECTION, "family", serde_json::Value::String(family)).ok();
        }

        let (access_token, refresh_token) = match generate_tokens(crypto.as_ref(), &user_id, &email, &roles) {
            Ok(t) => t,
            Err(r) => return r,
        };

        store_refresh_token(db.as_ref(), &user_id, &refresh_token);

        let cookie = format!(
            "auth_token={}; HttpOnly; Path=/; SameSite=Lax; Max-Age=86400",
            access_token
        );

        ResponseBuilder::new(msg.clone(), 200)
            .set_cookie(&cookie)
            .json(&serde_json::json!({
                "access_token": access_token,
                "refresh_token": refresh_token,
                "token_type": "Bearer",
                "expires_in": 86400
            }))
    }

    fn handle_logout(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let user_id = msg.user_id();
        if !user_id.is_empty() {
            if let Some(db) = ctx.services().and_then(|s| s.database.as_ref()) {
                database::delete_by_field(db.as_ref(), TOKENS_COLLECTION, "user_id", serde_json::Value::String(user_id.to_string())).ok();
            }
        }

        let cookie = "auth_token=; HttpOnly; Path=/; SameSite=Lax; Max-Age=0";
        ResponseBuilder::new(msg.clone(), 200)
            .set_cookie(cookie)
            .json(&serde_json::json!({"message": "Logged out successfully"}))
    }

    fn handle_me_get(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let db = match get_db(ctx) {
            Ok(db) => db,
            Err(r) => return r,
        };
        let user_id = msg.user_id();
        if user_id.is_empty() {
            return err_unauthorized(msg.clone(), "Not authenticated");
        }
        let user = match db.get(USERS_COLLECTION, user_id) {
            Ok(u) => u,
            Err(_) => return err_not_found(msg.clone(), "User not found"),
        };
        let roles = get_user_roles(db.as_ref(), user_id);
        json_respond(msg.clone(), 200, &serde_json::json!({
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

    fn handle_me_update(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let db = match get_db(ctx) {
            Ok(db) => db,
            Err(r) => return r,
        };
        let user_id = msg.user_id();
        if user_id.is_empty() {
            return err_unauthorized(msg.clone(), "Not authenticated");
        }

        let body: HashMap<String, serde_json::Value> = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
        };

        // Only allow updating certain fields
        let mut data = HashMap::new();
        for key in &["name", "avatar_url"] {
            if let Some(val) = body.get(*key) {
                data.insert(key.to_string(), val.clone());
            }
        }
        data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

        match db.update(USERS_COLLECTION, user_id, data) {
            Ok(user) => {
                let roles = get_user_roles(db.as_ref(), user_id);
                json_respond(msg.clone(), 200, &serde_json::json!({
                    "id": user.id,
                    "email": user.data.get("email").and_then(|v| v.as_str()).unwrap_or(""),
                    "name": user.data.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                    "roles": roles
                }))
            }
            Err(e) => err_internal(msg.clone(), &format!("Update failed: {e}")),
        }
    }

    fn handle_change_password(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let (db, crypto) = match get_services(ctx) {
            Ok(s) => s,
            Err(r) => return r,
        };
        let user_id = msg.user_id();
        if user_id.is_empty() {
            return err_unauthorized(msg.clone(), "Not authenticated");
        }

        #[derive(serde::Deserialize)]
        struct ChangePwReq { current_password: String, new_password: String }
        let body: ChangePwReq = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
        };

        if body.new_password.len() < 8 {
            return err_bad_request(msg.clone(), "New password must be at least 8 characters");
        }

        let user = match db.get(USERS_COLLECTION, user_id) {
            Ok(u) => u,
            Err(_) => return err_not_found(msg.clone(), "User not found"),
        };

        let stored_hash = user.data.get("password_hash").and_then(|v| v.as_str()).unwrap_or("");
        if crypto.compare_hash(&body.current_password, stored_hash).is_err() {
            return err_unauthorized(msg.clone(), "Current password is incorrect");
        }

        let new_hash = match crypto.hash(&body.new_password) {
            Ok(h) => h,
            Err(e) => return err_internal(msg.clone(), &format!("Hash failed: {e}")),
        };

        let mut data = HashMap::new();
        data.insert("password_hash".to_string(), serde_json::Value::String(new_hash));
        data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

        match db.update(USERS_COLLECTION, user_id, data) {
            Ok(_) => json_respond(msg.clone(), 200, &serde_json::json!({"message": "Password changed successfully"})),
            Err(e) => err_internal(msg.clone(), &format!("Update failed: {e}")),
        }
    }

    // --- API Key Management ---

    fn handle_api_keys_list(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let db = match get_db(ctx) {
            Ok(db) => db,
            Err(r) => return r,
        };
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
        match db.list(API_KEYS_COLLECTION, &opts) {
            Ok(mut result) => {
                // Strip key_hash from response
                for record in &mut result.records {
                    record.data.remove("key_hash");
                }
                json_respond(msg.clone(), 200, &result)
            }
            Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
        }
    }

    fn handle_api_keys_create(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let (db, crypto) = match get_services(ctx) {
            Ok(s) => s,
            Err(r) => return r,
        };
        let user_id = msg.user_id();

        #[derive(serde::Deserialize)]
        struct CreateKeyReq { name: String, expires_at: Option<String> }
        let body: CreateKeyReq = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
        };

        // Generate random key
        let random_bytes = match crypto.random_bytes(24) {
            Ok(b) => b,
            Err(e) => return err_internal(msg.clone(), &format!("Failed to generate key: {e}")),
        };
        let key_string = format!("sb_{}", hex_encode(&random_bytes));

        let key_hash = match crypto.hash(&key_string) {
            Ok(h) => h,
            Err(e) => return err_internal(msg.clone(), &format!("Hash failed: {e}")),
        };

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

        match db.create(API_KEYS_COLLECTION, data) {
            Ok(record) => json_respond(msg.clone(), 201, &serde_json::json!({
                "id": record.id,
                "key": key_string,
                "name": record.data.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                "key_prefix": record.data.get("key_prefix").and_then(|v| v.as_str()).unwrap_or(""),
                "message": "Save this key — it won't be shown again"
            })),
            Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
        }
    }

    fn handle_api_keys_revoke(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let db = match get_db(ctx) {
            Ok(db) => db,
            Err(r) => return r,
        };
        let id = msg.var("id");
        if id.is_empty() {
            return err_bad_request(msg.clone(), "Missing key ID");
        }
        let user_id = msg.user_id();

        // Verify ownership
        let key = match db.get(API_KEYS_COLLECTION, id) {
            Ok(k) => k,
            Err(_) => return err_not_found(msg.clone(), "API key not found"),
        };
        let key_owner = key.data.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
        if key_owner != user_id && !msg.is_admin() {
            return err_forbidden(msg.clone(), "Cannot revoke another user's API key");
        }

        let mut data = HashMap::new();
        data.insert("revoked_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
        match db.update(API_KEYS_COLLECTION, id, data) {
            Ok(_) => json_respond(msg.clone(), 200, &serde_json::json!({"message": "API key revoked"})),
            Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
        }
    }

    // --- OAuth ---

    fn handle_oauth_providers(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let config = ctx.services().and_then(|s| s.config.as_ref());
        let mut providers = Vec::new();

        if let Some(cfg) = config {
            for provider_name in &["google", "github", "microsoft"] {
                let client_id_key = format!("OAUTH_{}_CLIENT_ID", provider_name.to_uppercase());
                if cfg.get(&client_id_key).is_some() {
                    providers.push(serde_json::json!({
                        "name": provider_name,
                        "enabled": true
                    }));
                }
            }
        }

        json_respond(msg.clone(), 200, &serde_json::json!({"providers": providers}))
    }

    fn handle_oauth_login(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let config = match ctx.services().and_then(|s| s.config.as_ref()) {
            Some(c) => c,
            None => return err_internal(msg.clone(), "Config service unavailable"),
        };

        let provider = msg.query("provider");
        if provider.is_empty() {
            return err_bad_request(msg.clone(), "Missing provider parameter");
        }

        let client_id_key = format!("OAUTH_{}_CLIENT_ID", provider.to_uppercase());
        let client_id = match config.get(&client_id_key) {
            Some(id) => id,
            None => return err_bad_request(msg.clone(), &format!("OAuth provider '{}' not configured", provider)),
        };

        let redirect_uri = config.get_default("OAUTH_REDIRECT_URI", "http://localhost:8090/auth/oauth/callback");

        let auth_url = match provider {
            "google" => format!(
                "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&state={}",
                client_id, redirect_uri, provider
            ),
            "github" => format!(
                "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=user:email&state={}",
                client_id, redirect_uri, provider
            ),
            "microsoft" => format!(
                "https://login.microsoftonline.com/common/oauth2/v2.0/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&state={}",
                client_id, redirect_uri, provider
            ),
            _ => return err_bad_request(msg.clone(), &format!("Unsupported provider: {}", provider)),
        };

        json_respond(msg.clone(), 200, &serde_json::json!({
            "auth_url": auth_url,
            "provider": provider
        }))
    }

    fn handle_oauth_callback(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let (db, crypto) = match get_services(ctx) {
            Ok(s) => s,
            Err(r) => return r,
        };
        let config = match ctx.services().and_then(|s| s.config.as_ref()) {
            Some(c) => c,
            None => return err_internal(msg.clone(), "Config service unavailable"),
        };
        let network = match ctx.services().and_then(|s| s.network.as_ref()) {
            Some(n) => n,
            None => return err_internal(msg.clone(), "Network service unavailable"),
        };

        let code = msg.query("code");
        let state = msg.query("state"); // provider name
        if code.is_empty() || state.is_empty() {
            return err_bad_request(msg.clone(), "Missing code or state parameter");
        }

        let provider = state;
        let client_id = config.get_default(&format!("OAUTH_{}_CLIENT_ID", provider.to_uppercase()), "");
        let client_secret = config.get_default(&format!("OAUTH_{}_CLIENT_SECRET", provider.to_uppercase()), "");
        let redirect_uri = config.get_default("OAUTH_REDIRECT_URI", "http://localhost:8090/auth/oauth/callback");

        if client_id.is_empty() || client_secret.is_empty() {
            return err_internal(msg.clone(), "OAuth provider not fully configured");
        }

        // Exchange code for token
        let (token_url, token_body) = match provider {
            "google" => (
                "https://oauth2.googleapis.com/token".to_string(),
                format!("code={}&client_id={}&client_secret={}&redirect_uri={}&grant_type=authorization_code", code, client_id, client_secret, redirect_uri),
            ),
            "github" => (
                "https://github.com/login/oauth/access_token".to_string(),
                format!("code={}&client_id={}&client_secret={}&redirect_uri={}", code, client_id, client_secret, redirect_uri),
            ),
            _ => return err_bad_request(msg.clone(), "Unsupported OAuth provider"),
        };

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
        headers.insert("Accept".to_string(), "application/json".to_string());

        let token_resp = match network.do_request(&wafer_run::services::network::Request {
            method: "POST".to_string(),
            url: token_url,
            headers,
            body: Some(token_body.into_bytes()),
        }) {
            Ok(r) => r,
            Err(e) => return err_internal(msg.clone(), &format!("Token exchange failed: {e}")),
        };

        let token_data: serde_json::Value = match serde_json::from_slice(&token_resp.body) {
            Ok(d) => d,
            Err(_) => return err_internal(msg.clone(), "Failed to parse token response"),
        };

        let access_token_oauth = token_data.get("access_token").and_then(|v| v.as_str()).unwrap_or("");
        if access_token_oauth.is_empty() {
            return err_internal(msg.clone(), "No access token in OAuth response");
        }

        // Get user info
        let (userinfo_url, auth_header) = match provider {
            "google" => ("https://www.googleapis.com/oauth2/v2/userinfo".to_string(), format!("Bearer {}", access_token_oauth)),
            "github" => ("https://api.github.com/user".to_string(), format!("token {}", access_token_oauth)),
            _ => return err_internal(msg.clone(), "Unsupported provider"),
        };

        let mut info_headers = HashMap::new();
        info_headers.insert("Authorization".to_string(), auth_header);
        info_headers.insert("Accept".to_string(), "application/json".to_string());

        let info_resp = match network.do_request(&wafer_run::services::network::Request {
            method: "GET".to_string(),
            url: userinfo_url,
            headers: info_headers,
            body: None,
        }) {
            Ok(r) => r,
            Err(e) => return err_internal(msg.clone(), &format!("User info request failed: {e}")),
        };

        let user_info: serde_json::Value = match serde_json::from_slice(&info_resp.body) {
            Ok(d) => d,
            Err(_) => return err_internal(msg.clone(), "Failed to parse user info"),
        };

        let email = user_info.get("email").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
        let name = user_info.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let avatar = user_info.get("picture")
            .or_else(|| user_info.get("avatar_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if email.is_empty() {
            return err_internal(msg.clone(), "No email returned by OAuth provider");
        }

        // Upsert user
        let user = match database::get_by_field(db.as_ref(), USERS_COLLECTION, "email", serde_json::Value::String(email.clone())) {
            Ok(existing) => {
                let mut upd = HashMap::new();
                upd.insert("last_login_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                if !name.is_empty() { upd.insert("name".to_string(), serde_json::Value::String(name.clone())); }
                if !avatar.is_empty() { upd.insert("avatar_url".to_string(), serde_json::Value::String(avatar.clone())); }
                upd.insert("oauth_provider".to_string(), serde_json::Value::String(provider.to_string()));
                let _ = db.update(USERS_COLLECTION, &existing.id, upd);
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
                match db.create(USERS_COLLECTION, data) {
                    Ok(u) => {
                        // Assign default role
                        let user_count = db.count(USERS_COLLECTION, &[]).unwrap_or(1);
                        let role = if user_count <= 1 { "admin" } else { "user" };
                        let mut role_data = HashMap::new();
                        role_data.insert("user_id".to_string(), serde_json::Value::String(u.id.clone()));
                        role_data.insert("role".to_string(), serde_json::Value::String(role.to_string()));
                        role_data.insert("assigned_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                        let _ = db.create(USER_ROLES_COLLECTION, role_data);
                        u
                    }
                    Err(e) => return err_internal(msg.clone(), &format!("Failed to create user: {e}")),
                }
            }
        };

        let roles = get_user_roles(db.as_ref(), &user.id);
        let (jwt_token, refresh_token) = match generate_tokens(crypto.as_ref(), &user.id, &email, &roles) {
            Ok(t) => t,
            Err(r) => return r,
        };
        store_refresh_token(db.as_ref(), &user.id, &refresh_token);

        // Redirect to frontend with token
        let frontend_url = config.get_default("FRONTEND_URL", "http://localhost:5173");
        let redirect_url = format!("{}/?token={}", frontend_url, jwt_token);

        let cookie = format!(
            "auth_token={}; HttpOnly; Path=/; SameSite=Lax; Max-Age=86400",
            jwt_token
        );

        ResponseBuilder::new(msg.clone(), 302)
            .set_cookie(&cookie)
            .set_header("Location", &redirect_url)
            .json(&serde_json::json!({"redirect": redirect_url}))
    }

    fn handle_sync_user(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        // Internal endpoint for OAuth user sync
        let (db, _crypto) = match get_services(ctx) {
            Ok(s) => s,
            Err(r) => return r,
        };

        #[derive(serde::Deserialize)]
        struct SyncReq { email: String, name: Option<String>, provider: Option<String> }
        let body: SyncReq = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
        };

        let email_lower = body.email.trim().to_lowercase();
        let user = match database::get_by_field(db.as_ref(), USERS_COLLECTION, "email", serde_json::Value::String(email_lower.clone())) {
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
                match db.create(USERS_COLLECTION, data) {
                    Ok(u) => u,
                    Err(e) => return err_internal(msg.clone(), &format!("Create failed: {e}")),
                }
            }
        };

        json_respond(msg.clone(), 200, &serde_json::json!({"id": user.id, "email": user.data.get("email")}))
    }
}

// --- Helper functions ---

fn get_db(ctx: &dyn Context) -> Result<&Arc<dyn DatabaseService>, Result_> {
    ctx.services()
        .and_then(|s| s.database.as_ref())
        .ok_or_else(|| Result_::error(WaferError::new("unavailable", "Database service unavailable")))
}

fn get_services(ctx: &dyn Context) -> Result<(&Arc<dyn DatabaseService>, &Arc<dyn CryptoService>), Result_> {
    let svc = ctx.services().ok_or_else(|| Result_::error(WaferError::new("unavailable", "Services unavailable")))?;
    let db = svc.database.as_ref().ok_or_else(|| Result_::error(WaferError::new("unavailable", "Database unavailable")))?;
    let crypto = svc.crypto.as_ref().ok_or_else(|| Result_::error(WaferError::new("unavailable", "Crypto unavailable")))?;
    Ok((db, crypto))
}

fn get_user_roles(db: &dyn DatabaseService, user_id: &str) -> Vec<String> {
    let opts = ListOptions {
        filters: vec![Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.to_string()),
        }],
        ..Default::default()
    };
    match db.list(USER_ROLES_COLLECTION, &opts) {
        Ok(r) => r.records.iter()
            .filter_map(|rec| rec.data.get("role").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn generate_tokens(
    crypto: &dyn CryptoService,
    user_id: &str,
    email: &str,
    roles: &[String],
) -> Result<(String, String), Result_> {
    let family = hex_encode(&crypto.random_bytes(16).unwrap_or_else(|_| vec![0u8; 16]));

    let mut access_claims = HashMap::new();
    access_claims.insert("user_id".to_string(), serde_json::Value::String(user_id.to_string()));
    access_claims.insert("sub".to_string(), serde_json::Value::String(user_id.to_string()));
    access_claims.insert("email".to_string(), serde_json::Value::String(email.to_string()));
    access_claims.insert("roles".to_string(), serde_json::json!(roles));
    access_claims.insert("type".to_string(), serde_json::Value::String("access".to_string()));

    let access_token = crypto.sign(access_claims, Duration::from_secs(86400))
        .map_err(|e| Result_::error(WaferError::new("internal", format!("Token sign failed: {e}"))))?;

    let mut refresh_claims = HashMap::new();
    refresh_claims.insert("user_id".to_string(), serde_json::Value::String(user_id.to_string()));
    refresh_claims.insert("sub".to_string(), serde_json::Value::String(user_id.to_string()));
    refresh_claims.insert("type".to_string(), serde_json::Value::String("refresh".to_string()));
    refresh_claims.insert("family".to_string(), serde_json::Value::String(family));

    let refresh_token = crypto.sign(refresh_claims, Duration::from_secs(604800))
        .map_err(|e| Result_::error(WaferError::new("internal", format!("Refresh token sign failed: {e}"))))?;

    Ok((access_token, refresh_token))
}

fn store_refresh_token(db: &dyn DatabaseService, user_id: &str, token: &str) {
    let mut data = HashMap::new();
    data.insert("user_id".to_string(), serde_json::Value::String(user_id.to_string()));
    data.insert("token".to_string(), serde_json::Value::String(token.to_string()));
    data.insert("created_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    let _ = db.create(TOKENS_COLLECTION, data);
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Seed a default admin user if no users exist yet.
/// This is idempotent: it only creates the user when the auth_users collection is empty.
fn seed_admin_user(db: &dyn DatabaseService, crypto: &dyn CryptoService) {
    let count = db.count(USERS_COLLECTION, &[]).unwrap_or(0);
    if count > 0 {
        return;
    }

    let password_hash = match crypto.hash("admin123") {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to hash default admin password: {e}");
            return;
        }
    };

    let now = chrono::Utc::now().to_rfc3339();
    let mut data = HashMap::new();
    data.insert("email".to_string(), serde_json::Value::String("admin@example.com".to_string()));
    data.insert("password_hash".to_string(), serde_json::Value::String(password_hash));
    data.insert("name".to_string(), serde_json::Value::String("Admin".to_string()));
    data.insert("created_at".to_string(), serde_json::Value::String(now.clone()));
    data.insert("updated_at".to_string(), serde_json::Value::String(now));
    data.insert("disabled".to_string(), serde_json::Value::Bool(false));

    match db.create(USERS_COLLECTION, data) {
        Ok(user) => {
            let mut role_data = HashMap::new();
            role_data.insert("user_id".to_string(), serde_json::Value::String(user.id.clone()));
            role_data.insert("role".to_string(), serde_json::Value::String("admin".to_string()));
            role_data.insert("assigned_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            let _ = db.create(USER_ROLES_COLLECTION, role_data);
            tracing::info!("default admin user seeded (admin@example.com)");
        }
        Err(e) => {
            tracing::error!("Failed to seed admin user: {e}");
        }
    }
}

impl Block for AuthBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "auth-feature".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "Authentication: login, signup, JWT, refresh tokens, OAuth, API keys".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
        }
    }

    fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let action = msg.action();
        let path = msg.path();

        match (action, path) {
            ("create", "/auth/login") => self.handle_login(ctx, msg),
            ("create", "/auth/signup") => self.handle_signup(ctx, msg),
            ("create", "/auth/refresh") => self.handle_refresh(ctx, msg),
            ("create", "/auth/logout") => self.handle_logout(ctx, msg),
            ("retrieve", "/auth/me") => self.handle_me_get(ctx, msg),
            ("update", "/auth/me") => self.handle_me_update(ctx, msg),
            ("create", "/auth/change-password") => self.handle_change_password(ctx, msg),
            // API keys
            ("retrieve", "/auth/api-keys") => self.handle_api_keys_list(ctx, msg),
            ("create", "/auth/api-keys") => self.handle_api_keys_create(ctx, msg),
            ("delete", _) if path.starts_with("/auth/api-keys/") => self.handle_api_keys_revoke(ctx, msg),
            // OAuth
            ("retrieve", "/auth/oauth/providers") => self.handle_oauth_providers(ctx, msg),
            ("retrieve", "/auth/oauth/login") => self.handle_oauth_login(ctx, msg),
            ("retrieve", "/auth/oauth/callback") => self.handle_oauth_callback(ctx, msg),
            // Internal
            ("create", "/internal/oauth/sync-user") => self.handle_sync_user(ctx, msg),
            _ => err_not_found(msg.clone(), "not found"),
        }
    }

    fn lifecycle(&self, ctx: &dyn Context, event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        if matches!(event.event_type, LifecycleType::Init) {
            if let Some(svc) = ctx.services() {
                if let (Some(db), Some(crypto)) = (svc.database.as_ref(), svc.crypto.as_ref()) {
                    seed_admin_user(db.as_ref(), crypto.as_ref());
                }
            }
        }
        Ok(())
    }
}
