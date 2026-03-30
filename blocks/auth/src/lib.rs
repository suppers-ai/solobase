wit_bindgen::generate!({
    world: "wafer-block",
    path: "../../../wafer-run/wit/wit",
    additional_derives: [serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash],
    export_macro_name: "export_block",
});

use exports::wafer::block_world::block::Guest;
use wafer::block_world::types::*;

// wafer-core clients (use WASM sync variants via WIT call-block import)
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_core::clients::{crypto, config, network};

mod helpers;
use helpers::*;

struct AuthBlockWasm;

const USERS_COLLECTION: &str = "auth_users";
const TOKENS_COLLECTION: &str = "auth_tokens";
const API_KEYS_COLLECTION: &str = "api_keys";
const USER_ROLES_COLLECTION: &str = "iam_user_roles";

impl Guest for AuthBlockWasm {
    fn info() -> BlockInfo {
        BlockInfo {
            name: "suppers-ai/auth".to_string(),
            version: "1.0.0".to_string(),
            interface: "http-handler@v1".to_string(),
            summary: "Authentication: login, signup, JWT, refresh tokens".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: Vec::new(),
            collections: Vec::new(),
            config_schema: None,
        }
    }

    fn handle(msg: Message) -> BlockResult {
        let action = msg_get_meta(&msg, "req.action").to_string();
        let path = msg_get_meta(&msg, "req.resource").to_string();

        // No rate limiting in WASM — CF has platform-level rate limiting.
        match (action.as_str(), path.as_str()) {
            ("create", "/auth/login") => handle_login(&msg),
            ("create", "/auth/signup") => handle_signup(&msg),
            ("create", "/auth/refresh") => handle_refresh(&msg),
            ("create", "/auth/logout") => handle_logout(&msg),
            ("retrieve", "/auth/me") => handle_me_get(&msg),
            ("update", "/auth/me") => handle_me_update(&msg),
            ("create", "/auth/change-password") => handle_change_password(&msg),
            // Email verification
            ("create", "/auth/verify-email") => handle_verify_email(&msg),
            ("create", "/auth/resend-verification") => handle_resend_verification(&msg),
            // Password reset
            ("create", "/auth/forgot-password") => handle_forgot_password(&msg),
            ("create", "/auth/reset-password") => handle_reset_password(&msg),
            // API keys
            ("retrieve", "/auth/api-keys") => handle_api_keys_list(&msg),
            ("create", "/auth/api-keys") => handle_api_keys_create(&msg),
            ("delete", _) if path.starts_with("/auth/api-keys/") => handle_api_keys_revoke(&msg),
            // OAuth
            ("retrieve", "/auth/oauth/providers") => handle_oauth_providers(&msg),
            ("retrieve", "/auth/oauth/login") => handle_oauth_login(&msg),
            ("retrieve", "/auth/oauth/callback") => handle_oauth_callback(&msg),
            _ => err_not_found(&msg, "not found"),
        }
    }

    fn lifecycle(_event: LifecycleEvent) -> Result<(), WaferError> {
        // Lifecycle (admin seeding) is handled by the native runtime.
        Ok(())
    }
}

export_block!(AuthBlockWasm);

// ---------------------------------------------------------------------------
// Handlers (sync — use wafer-core WASM client shims)
// ---------------------------------------------------------------------------

fn handle_login(msg: &Message) -> BlockResult {
    #[derive(serde::Deserialize)]
    struct LoginReq { email: String, password: String }

    let body: LoginReq = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };
    let email_lower = body.email.trim().to_lowercase();

    // Find user by email
    let user = match db::get_by_field(USERS_COLLECTION, "email", serde_json::json!(email_lower)) {
        Ok(u) => u,
        Err(_) => return error_response(msg, ErrorCode::Unauthenticated, "Invalid email or password"),
    };

    // Check password
    let stored_hash = str_field(&user, "password_hash");
    if crypto::compare_hash(&body.password, stored_hash).is_err() {
        return error_response(msg, ErrorCode::Unauthenticated, "Invalid email or password");
    }

    // Check disabled
    if bool_field(&user, "disabled") {
        return error_response(msg, ErrorCode::PermissionDenied, "Account is disabled");
    }

    // Get roles
    let roles = get_user_roles(&user.id);

    // Generate tokens
    let (access_token, refresh_token) = match generate_tokens(&user.id, &email_lower, &roles) {
        Ok(t) => t,
        Err(r) => return r,
    };

    // Store refresh token
    store_refresh_token(&user.id, &refresh_token);

    // Update last login
    let upd = json_map(serde_json::json!({"last_login_at": now_rfc3339()}));
    let _ = db::update(USERS_COLLECTION, &user.id, upd);

    let cookie = build_auth_cookie(&access_token, 86400);

    respond_with_cookie(msg, &cookie, &serde_json::json!({
        "access_token": access_token,
        "refresh_token": refresh_token,
        "token_type": "Bearer",
        "expires_in": 86400,
        "user": {
            "id": user.id,
            "email": email_lower,
            "roles": roles,
            "name": str_field(&user, "name")
        }
    }))
}

fn handle_signup(msg: &Message) -> BlockResult {
    #[derive(serde::Deserialize)]
    struct SignupReq { email: String, password: String, name: Option<String> }

    let body: SignupReq = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };
    let email_lower = body.email.trim().to_lowercase();

    // Validate
    let parts: Vec<&str> = email_lower.splitn(2, '@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() || !parts[1].contains('.') {
        return error_response(msg, ErrorCode::InvalidArgument, "Invalid email address");
    }
    if body.password.len() < 8 {
        return error_response(msg, ErrorCode::InvalidArgument, "Password must be at least 8 characters");
    }
    if body.password.len() > 1024 {
        return error_response(msg, ErrorCode::InvalidArgument, "Password must not exceed 1024 characters");
    }

    // Check if user exists
    if db::get_by_field(USERS_COLLECTION, "email", serde_json::json!(email_lower)).is_ok() {
        return error_response(msg, ErrorCode::AlreadyExists, "Email already registered");
    }

    // Hash password
    let password_hash = match crypto::hash(&body.password) {
        Ok(h) => h,
        Err(e) => return err_internal(msg, &format!("Failed to hash password: {e}")),
    };

    let mut data = json_map(serde_json::json!({
        "email": email_lower,
        "password_hash": password_hash,
        "name": body.name.unwrap_or_default(),
        "disabled": false,
        "created_at": now_rfc3339()
    }));
    stamp_created(&mut data);

    let user = match db::create(USERS_COLLECTION, data) {
        Ok(u) => u,
        Err(e) => return err_internal(msg, &format!("Failed to create user: {e}")),
    };

    // Assign default role
    let admin_email = config::get_default("ADMIN_EMAIL", "");
    let user_email = user.data.get("email").and_then(|v| v.as_str()).unwrap_or("");
    let role = if !admin_email.is_empty() && user_email.eq_ignore_ascii_case(&admin_email) {
        "admin"
    } else {
        "user"
    };
    let role_data = json_map(serde_json::json!({
        "user_id": user.id,
        "role": role,
        "assigned_at": now_rfc3339()
    }));
    let _ = db::create(USER_ROLES_COLLECTION, role_data);
    let roles = vec![role.to_string()];

    // Generate tokens
    let (access_token, refresh_token) = match generate_tokens(&user.id, &email_lower, &roles) {
        Ok(t) => t,
        Err(r) => return r,
    };
    store_refresh_token(&user.id, &refresh_token);

    let cookie = build_auth_cookie(&access_token, 86400);

    respond_with_status_and_cookie(msg, 201, &cookie, &serde_json::json!({
        "access_token": access_token,
        "refresh_token": refresh_token,
        "token_type": "Bearer",
        "expires_in": 86400,
        "user": {
            "id": user.id,
            "email": email_lower,
            "roles": roles,
            "name": str_field(&user, "name")
        }
    }))
}

fn handle_refresh(msg: &Message) -> BlockResult {
    #[derive(serde::Deserialize)]
    struct RefreshReq { refresh_token: String }

    let body: RefreshReq = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    // Verify refresh token
    let claims = match crypto::verify(&body.refresh_token) {
        Ok(c) => c,
        Err(_) => return error_response(msg, ErrorCode::Unauthenticated, "Invalid or expired refresh token"),
    };

    let user_id = claims.get("user_id")
        .or_else(|| claims.get("sub"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if user_id.is_empty() {
        return error_response(msg, ErrorCode::Unauthenticated, "Invalid refresh token");
    }

    let token_type = claims.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if token_type != "refresh" {
        return error_response(msg, ErrorCode::Unauthenticated, "Not a refresh token");
    }

    // Get user
    let user = match db::get(USERS_COLLECTION, &user_id) {
        Ok(u) => u,
        Err(_) => return error_response(msg, ErrorCode::Unauthenticated, "User not found"),
    };

    let email = str_field(&user, "email").to_string();
    let roles = get_user_roles(&user_id);

    // Revoke old family
    let family = claims.get("family").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if !family.is_empty() {
        let _ = db::delete_by_field(TOKENS_COLLECTION, "family", serde_json::json!(family));
    }

    let (access_token, refresh_token) = match generate_tokens(&user_id, &email, &roles) {
        Ok(t) => t,
        Err(r) => return r,
    };
    store_refresh_token(&user_id, &refresh_token);

    let cookie = build_auth_cookie(&access_token, 86400);

    respond_with_cookie(msg, &cookie, &serde_json::json!({
        "access_token": access_token,
        "refresh_token": refresh_token,
        "token_type": "Bearer",
        "expires_in": 86400
    }))
}

fn handle_logout(msg: &Message) -> BlockResult {
    let user_id = msg_get_meta(msg, "auth.user_id");
    if !user_id.is_empty() {
        let _ = db::delete_by_field(TOKENS_COLLECTION, "user_id", serde_json::json!(user_id));
    }
    let cookie = build_auth_cookie("", 0);
    respond_with_cookie(msg, &cookie, &serde_json::json!({"message": "Logged out successfully"}))
}

fn handle_me_get(msg: &Message) -> BlockResult {
    let user_id = msg_get_meta(msg, "auth.user_id");
    if user_id.is_empty() {
        return error_response(msg, ErrorCode::Unauthenticated, "Not authenticated");
    }
    let user = match db::get(USERS_COLLECTION, user_id) {
        Ok(u) => u,
        Err(_) => return err_not_found(msg, "User not found"),
    };
    let roles = get_user_roles(user_id);
    json_respond(msg, &serde_json::json!({
        "user": {
            "id": user.id,
            "email": str_field(&user, "email"),
            "name": str_field(&user, "name"),
            "roles": roles,
            "created_at": str_field(&user, "created_at"),
            "avatar_url": str_field(&user, "avatar_url")
        }
    }))
}

fn handle_me_update(msg: &Message) -> BlockResult {
    let user_id = msg_get_meta(msg, "auth.user_id");
    if user_id.is_empty() {
        return error_response(msg, ErrorCode::Unauthenticated, "Not authenticated");
    }

    let body: std::collections::HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    let mut data = std::collections::HashMap::new();
    for key in &["name", "avatar_url"] {
        if let Some(val) = body.get(*key) {
            data.insert(key.to_string(), val.clone());
        }
    }
    stamp_updated(&mut data);

    match db::update(USERS_COLLECTION, user_id, data) {
        Ok(user) => {
            let roles = get_user_roles(user_id);
            json_respond(msg, &serde_json::json!({
                "id": user.id,
                "email": str_field(&user, "email"),
                "name": str_field(&user, "name"),
                "roles": roles
            }))
        }
        Err(e) => err_internal(msg, &format!("Update failed: {e}")),
    }
}

fn handle_change_password(msg: &Message) -> BlockResult {
    let user_id = msg_get_meta(msg, "auth.user_id");
    if user_id.is_empty() {
        return error_response(msg, ErrorCode::Unauthenticated, "Not authenticated");
    }

    #[derive(serde::Deserialize)]
    struct ChangePwReq { current_password: String, new_password: String }
    let body: ChangePwReq = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    if body.new_password.len() < 8 {
        return error_response(msg, ErrorCode::InvalidArgument, "New password must be at least 8 characters");
    }
    if body.new_password.len() > 1024 {
        return error_response(msg, ErrorCode::InvalidArgument, "Password must not exceed 1024 characters");
    }

    let user = match db::get(USERS_COLLECTION, user_id) {
        Ok(u) => u,
        Err(_) => return err_not_found(msg, "User not found"),
    };

    let stored_hash = str_field(&user, "password_hash");
    if crypto::compare_hash(&body.current_password, stored_hash).is_err() {
        return error_response(msg, ErrorCode::Unauthenticated, "Current password is incorrect");
    }

    let new_hash = match crypto::hash(&body.new_password) {
        Ok(h) => h,
        Err(e) => return err_internal(msg, &format!("Hash failed: {e}")),
    };

    let mut data = json_map(serde_json::json!({"password_hash": new_hash}));
    stamp_updated(&mut data);

    match db::update(USERS_COLLECTION, user_id, data) {
        Ok(_) => json_respond(msg, &serde_json::json!({"message": "Password changed successfully"})),
        Err(e) => err_internal(msg, &format!("Update failed: {e}")),
    }
}

// ---------------------------------------------------------------------------
// Email verification handlers
// ---------------------------------------------------------------------------

fn handle_verify_email(msg: &Message) -> BlockResult {
    #[derive(serde::Deserialize)]
    struct VerifyEmailReq { token: String }

    let body: VerifyEmailReq = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    // Verify the JWT token
    let claims = match crypto::verify(&body.token) {
        Ok(c) => c,
        Err(_) => return error_response(msg, ErrorCode::Unauthenticated, "Invalid or expired verification token"),
    };

    let token_type = claims.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if token_type != "email_verification" {
        return error_response(msg, ErrorCode::InvalidArgument, "Not an email verification token");
    }

    let user_id = claims.get("sub")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if user_id.is_empty() {
        return error_response(msg, ErrorCode::InvalidArgument, "Invalid verification token");
    }

    // Update user: set email_verified = 1
    let data = json_map(serde_json::json!({"email_verified": 1}));
    match db::update(USERS_COLLECTION, &user_id, data) {
        Ok(_) => json_respond(msg, &serde_json::json!({"message": "Email verified successfully"})),
        Err(e) => err_internal(msg, &format!("Failed to update user: {e}")),
    }
}

fn handle_resend_verification(msg: &Message) -> BlockResult {
    let user_id = msg_get_meta(msg, "auth.user_id");
    if user_id.is_empty() {
        return error_response(msg, ErrorCode::Unauthenticated, "Not authenticated");
    }

    let user = match db::get(USERS_COLLECTION, user_id) {
        Ok(u) => u,
        Err(_) => return err_not_found(msg, "User not found"),
    };

    // Check if already verified
    let verified = user.data.get("email_verified")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if verified == 1 {
        return json_respond(msg, &serde_json::json!({"message": "Email is already verified"}));
    }

    let email = str_field(&user, "email").to_string();

    // Generate verification JWT
    let mut claims = std::collections::HashMap::new();
    claims.insert("sub".to_string(), serde_json::json!(user_id));
    claims.insert("email".to_string(), serde_json::json!(email));
    claims.insert("type".to_string(), serde_json::json!("email_verification"));

    let token = match crypto::sign(&claims, std::time::Duration::from_secs(86400)) {
        Ok(t) => t,
        Err(_) => return err_internal(msg, "Failed to generate verification token"),
    };

    json_respond(msg, &serde_json::json!({
        "message": "Verification email sent",
        "_verification_token": token
    }))
}

// ---------------------------------------------------------------------------
// Password reset handlers
// ---------------------------------------------------------------------------

fn handle_forgot_password(msg: &Message) -> BlockResult {
    #[derive(serde::Deserialize)]
    struct ForgotPasswordReq { email: String }

    let body: ForgotPasswordReq = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };
    let email_lower = body.email.trim().to_lowercase();

    let success_msg = "If an account exists with that email, a password reset link has been sent.";

    // Look up user — always return same message regardless
    let user = match db::get_by_field(USERS_COLLECTION, "email", serde_json::json!(email_lower)) {
        Ok(u) => u,
        Err(_) => return json_respond(msg, &serde_json::json!({"message": success_msg})),
    };

    // Generate password reset JWT
    let mut claims = std::collections::HashMap::new();
    claims.insert("sub".to_string(), serde_json::json!(user.id));
    claims.insert("email".to_string(), serde_json::json!(email_lower));
    claims.insert("type".to_string(), serde_json::json!("password_reset"));

    let token = match crypto::sign(&claims, std::time::Duration::from_secs(3600)) {
        Ok(t) => t,
        Err(_) => return json_respond(msg, &serde_json::json!({"message": success_msg})),
    };

    json_respond(msg, &serde_json::json!({
        "message": success_msg,
        "_reset_token": token
    }))
}

fn handle_reset_password(msg: &Message) -> BlockResult {
    #[derive(serde::Deserialize)]
    struct ResetPasswordReq { token: String, new_password: String }

    let body: ResetPasswordReq = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    // Validate password length
    if body.new_password.len() < 8 {
        return error_response(msg, ErrorCode::InvalidArgument, "Password must be at least 8 characters");
    }
    if body.new_password.len() > 256 {
        return error_response(msg, ErrorCode::InvalidArgument, "Password must not exceed 256 characters");
    }

    // Verify the JWT token
    let claims = match crypto::verify(&body.token) {
        Ok(c) => c,
        Err(_) => return error_response(msg, ErrorCode::Unauthenticated, "Invalid or expired reset token"),
    };

    let token_type = claims.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if token_type != "password_reset" {
        return error_response(msg, ErrorCode::InvalidArgument, "Not a password reset token");
    }

    let user_id = claims.get("sub")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if user_id.is_empty() {
        return error_response(msg, ErrorCode::InvalidArgument, "Invalid reset token");
    }

    // Hash new password
    let password_hash = match crypto::hash(&body.new_password) {
        Ok(h) => h,
        Err(e) => return err_internal(msg, &format!("Failed to hash password: {e}")),
    };

    // Update user's password
    let mut data = json_map(serde_json::json!({"password_hash": password_hash}));
    stamp_updated(&mut data);
    if let Err(e) = db::update(USERS_COLLECTION, &user_id, data) {
        return err_internal(msg, &format!("Failed to update password: {e}"));
    }

    // Delete all refresh tokens for the user (force re-login)
    let _ = db::delete_by_field(TOKENS_COLLECTION, "user_id", serde_json::json!(user_id));

    json_respond(msg, &serde_json::json!({
        "message": "Password reset successfully. Please log in with your new password."
    }))
}

// ---------------------------------------------------------------------------
// API Keys handlers
// ---------------------------------------------------------------------------

fn handle_api_keys_list(msg: &Message) -> BlockResult {
    let user_id = msg_get_meta(msg, "auth.user_id");
    let opts = ListOptions {
        filters: vec![Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::json!(user_id),
        }],
        sort: vec![SortField { field: "created_at".to_string(), desc: true }],
        limit: 100,
        ..Default::default()
    };
    match db::list(API_KEYS_COLLECTION, &opts) {
        Ok(mut result) => {
            for record in &mut result.records {
                record.data.remove("key_hash");
            }
            json_respond(msg, &serde_json::json!(result))
        }
        Err(_) => err_internal(msg, "Database error"),
    }
}

fn handle_api_keys_create(msg: &Message) -> BlockResult {
    let user_id = msg_get_meta(msg, "auth.user_id");

    #[derive(serde::Deserialize)]
    struct CreateKeyReq { name: String, expires_at: Option<String> }
    let body: CreateKeyReq = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    let random_bytes = match crypto::random_bytes(24) {
        Ok(b) => b,
        Err(_) => return err_internal(msg, "Failed to generate key"),
    };
    let key_string = format!("sb_{}", hex_encode(&random_bytes));
    let key_hash = sha256_hex(key_string.as_bytes());

    let mut data = std::collections::HashMap::new();
    data.insert("user_id".to_string(), serde_json::json!(user_id));
    data.insert("name".to_string(), serde_json::json!(body.name));
    data.insert("key_hash".to_string(), serde_json::json!(key_hash));
    data.insert("key_prefix".to_string(), serde_json::json!(&key_string[..10]));
    data.insert("created_at".to_string(), serde_json::json!(now_rfc3339()));
    if let Some(exp) = body.expires_at {
        data.insert("expires_at".to_string(), serde_json::json!(exp));
    }

    match db::create(API_KEYS_COLLECTION, data) {
        Ok(record) => json_respond(msg, &serde_json::json!({
            "id": record.id,
            "key": key_string,
            "name": str_field(&record, "name"),
            "key_prefix": str_field(&record, "key_prefix"),
            "message": "Save this key — it won't be shown again"
        })),
        Err(_) => err_internal(msg, "Database error"),
    }
}

fn handle_api_keys_revoke(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let id = path.strip_prefix("/auth/api-keys/").unwrap_or("");
    if id.is_empty() {
        return error_response(msg, ErrorCode::InvalidArgument, "Missing key ID");
    }
    let user_id = msg_get_meta(msg, "auth.user_id");

    let key = match db::get(API_KEYS_COLLECTION, id) {
        Ok(k) => k,
        Err(_) => return err_not_found(msg, "API key not found"),
    };
    let key_owner = str_field(&key, "user_id");
    if key_owner != user_id && !msg_get_meta(msg, "auth.user_roles").split(',').any(|r| r.trim() == "admin") {
        return error_response(msg, ErrorCode::PermissionDenied, "Cannot revoke another user's API key");
    }

    let data = json_map(serde_json::json!({"revoked_at": now_rfc3339()}));
    match db::update(API_KEYS_COLLECTION, id, data) {
        Ok(_) => json_respond(msg, &serde_json::json!({"message": "API key revoked"})),
        Err(_) => err_internal(msg, "Database error"),
    }
}

// ---------------------------------------------------------------------------
// OAuth handlers
// ---------------------------------------------------------------------------

fn handle_oauth_providers(msg: &Message) -> BlockResult {
    let mut providers = Vec::new();
    for name in &["google", "github", "microsoft"] {
        let key = format!("OAUTH_{}_CLIENT_ID", name.to_uppercase());
        if config::get(&key).is_ok() {
            providers.push(serde_json::json!({"name": name, "enabled": true}));
        }
    }
    json_respond(msg, &serde_json::json!({"providers": providers}))
}

fn handle_oauth_login(msg: &Message) -> BlockResult {
    let provider = msg_get_meta(msg, "req.query.provider");
    if provider.is_empty() {
        return error_response(msg, ErrorCode::InvalidArgument, "Missing provider parameter");
    }

    let client_id = match config::get(&format!("OAUTH_{}_CLIENT_ID", provider.to_uppercase())) {
        Ok(id) => id,
        Err(_) => return error_response(msg, ErrorCode::InvalidArgument, &format!("OAuth provider '{}' not configured", provider)),
    };
    let redirect_uri = config::get_default("OAUTH_REDIRECT_URI", "http://localhost:8090/auth/oauth/callback");

    let mut state_claims = std::collections::HashMap::new();
    state_claims.insert("provider".to_string(), serde_json::json!(provider));
    state_claims.insert("type".to_string(), serde_json::json!("oauth_state"));
    let state = match crypto::sign(&state_claims, std::time::Duration::from_secs(600)) {
        Ok(s) => s,
        Err(_) => return err_internal(msg, "Failed to generate state"),
    };

    let auth_url = match provider {
        "google" => format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&state={}",
            client_id, urlencode(&redirect_uri), urlencode(&state)
        ),
        "github" => format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=user:email&state={}",
            client_id, urlencode(&redirect_uri), urlencode(&state)
        ),
        "microsoft" => format!(
            "https://login.microsoftonline.com/common/oauth2/v2.0/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&state={}",
            client_id, urlencode(&redirect_uri), urlencode(&state)
        ),
        _ => return error_response(msg, ErrorCode::InvalidArgument, &format!("Unsupported provider: {}", provider)),
    };

    json_respond(msg, &serde_json::json!({"auth_url": auth_url, "provider": provider}))
}

fn handle_oauth_callback(msg: &Message) -> BlockResult {
    let code = msg_get_meta(msg, "req.query.code");
    let state = msg_get_meta(msg, "req.query.state");
    if code.is_empty() || state.is_empty() {
        return error_response(msg, ErrorCode::InvalidArgument, "Missing code or state parameter");
    }

    let state_claims = match crypto::verify(state) {
        Ok(c) => c,
        Err(_) => return error_response(msg, ErrorCode::InvalidArgument, "Invalid or expired OAuth state"),
    };
    let state_type = state_claims.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if state_type != "oauth_state" {
        return error_response(msg, ErrorCode::InvalidArgument, "Invalid OAuth state token");
    }
    let provider = state_claims.get("provider").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if provider.is_empty() {
        return error_response(msg, ErrorCode::InvalidArgument, "Missing provider in OAuth state");
    }

    let client_id = config::get_default(&format!("OAUTH_{}_CLIENT_ID", provider.to_uppercase()), "");
    let client_secret = config::get_default(&format!("OAUTH_{}_CLIENT_SECRET", provider.to_uppercase()), "");
    let redirect_uri = config::get_default("OAUTH_REDIRECT_URI", "http://localhost:8090/auth/oauth/callback");

    if client_id.is_empty() || client_secret.is_empty() {
        return err_internal(msg, "OAuth provider not fully configured");
    }

    // Exchange code for token
    let (token_url, token_body_str) = match provider.as_str() {
        "google" => (
            "https://oauth2.googleapis.com/token",
            format!("code={}&client_id={}&client_secret={}&redirect_uri={}&grant_type=authorization_code",
                urlencode(code), urlencode(&client_id), urlencode(&client_secret), urlencode(&redirect_uri)),
        ),
        "github" => (
            "https://github.com/login/oauth/access_token",
            format!("code={}&client_id={}&client_secret={}&redirect_uri={}",
                urlencode(code), urlencode(&client_id), urlencode(&client_secret), urlencode(&redirect_uri)),
        ),
        _ => return error_response(msg, ErrorCode::InvalidArgument, "Unsupported OAuth provider"),
    };

    let mut headers = std::collections::HashMap::new();
    headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
    headers.insert("Accept".to_string(), "application/json".to_string());

    let token_body_bytes = token_body_str.into_bytes();
    let token_resp = match network::do_request("POST", token_url, &headers, Some(&token_body_bytes)) {
        Ok(r) => r,
        Err(_) => return err_internal(msg, "Token exchange failed"),
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
        "google" => ("https://www.googleapis.com/oauth2/v2/userinfo", format!("Bearer {}", access_token_oauth)),
        "github" => ("https://api.github.com/user", format!("token {}", access_token_oauth)),
        _ => return err_internal(msg, "Unsupported provider"),
    };

    let mut info_headers = std::collections::HashMap::new();
    info_headers.insert("Authorization".to_string(), auth_header);
    info_headers.insert("Accept".to_string(), "application/json".to_string());

    let info_resp = match network::do_request("GET", userinfo_url, &info_headers, None) {
        Ok(r) => r,
        Err(_) => return err_internal(msg, "User info request failed"),
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
    let user = match db::get_by_field(USERS_COLLECTION, "email", serde_json::json!(email)) {
        Ok(existing) => {
            let mut upd = json_map(serde_json::json!({"oauth_provider": provider}));
            if !name.is_empty() { upd.insert("name".to_string(), serde_json::json!(name)); }
            if !avatar.is_empty() { upd.insert("avatar_url".to_string(), serde_json::json!(avatar)); }
            let _ = db::update(USERS_COLLECTION, &existing.id, upd);
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
            stamp_created(&mut data);
            match db::create(USERS_COLLECTION, data) {
                Ok(u) => {
                    // Assign default role
                    let admin_email = config::get_default("ADMIN_EMAIL", "");
                    let role = if !admin_email.is_empty() && email.eq_ignore_ascii_case(&admin_email) { "admin" } else { "user" };
                    let role_data = json_map(serde_json::json!({"user_id": u.id, "role": role, "assigned_at": now_rfc3339()}));
                    let _ = db::create(USER_ROLES_COLLECTION, role_data);
                    u
                }
                Err(_) => return err_internal(msg, "Failed to create user"),
            }
        }
    };

    let roles = get_user_roles(&user.id);
    let (jwt_token, refresh_token) = match generate_tokens(&user.id, &email, &roles) {
        Ok(t) => t,
        Err(r) => return r,
    };
    store_refresh_token(&user.id, &refresh_token);

    let frontend_url = config::get_default("FRONTEND_URL", "http://localhost:5173");
    let redirect_url = format!("{}/?token={}", frontend_url, jwt_token);
    let cookie = build_auth_cookie(&jwt_token, 86400);

    respond_with_status_and_cookie_and_header(msg, 302, &cookie, "Location", &redirect_url,
        &serde_json::json!({"redirect": redirect_url}))
}
