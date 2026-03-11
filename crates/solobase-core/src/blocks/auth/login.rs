use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::crypto;
use super::helpers::*;
use super::{AuthBlock, USERS_COLLECTION};
use crate::blocks::errors::{ErrorCode, error_response};
use crate::blocks::helpers::{RecordExt, json_map};

impl AuthBlock {
    pub(super) async fn handle_login(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
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
        let stored_hash = user.str_field("password_hash");
        if crypto::compare_hash(ctx, &body.password, stored_hash).await.is_err() {
            return error_response(msg, ErrorCode::InvalidCredentials, "Invalid email or password");
        }

        // Check if user is disabled
        if user.bool_field("disabled") {
            return error_response(msg, ErrorCode::AccountDisabled, "Account is disabled");
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
        let upd = json_map(serde_json::json!({"last_login_at": crate::blocks::helpers::now_rfc3339()}));
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
                    "name": user.str_field("name")
                }
            }))
    }

    pub(super) async fn handle_signup(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
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

        let mut data = json_map(serde_json::json!({
            "email": email_lower,
            "password_hash": password_hash,
            "name": body.name.unwrap_or_default(),
            "disabled": false
        }));
        crate::blocks::helpers::stamp_created(&mut data);

        let (user, default_role) = match create_user_and_assign_role(ctx, data).await {
            Ok(r) => r,
            Err(e) => return err_internal(msg, &e),
        };

        let roles = vec![default_role];

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
                    "name": user.str_field("name")
                }
            }))
    }

    pub(super) async fn handle_refresh(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
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

        let email = user.str_field("email").to_string();
        let roles = get_user_roles(ctx, &user_id).await;

        // Revoke old refresh token family and issue new
        let family = claims.get("family").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if !family.is_empty() {
            db::delete_by_field(ctx, super::TOKENS_COLLECTION, "family", serde_json::Value::String(family)).await.ok();
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

    pub(super) async fn handle_logout(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let user_id = msg.user_id();
        if !user_id.is_empty() {
            db::delete_by_field(ctx, super::TOKENS_COLLECTION, "user_id", serde_json::Value::String(user_id.to_string())).await.ok();
        }

        let cookie = build_auth_cookie("", 0, ctx).await;
        ResponseBuilder::new(msg)
            .set_cookie(&cookie)
            .json(&serde_json::json!({"message": "Logged out successfully"}))
    }

    pub(super) async fn handle_me_get(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
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
                "email": user.str_field("email"),
                "name": user.str_field("name"),
                "roles": roles,
                "created_at": user.str_field("created_at"),
                "avatar_url": user.str_field("avatar_url")
            }
        }))
    }

    pub(super) async fn handle_me_update(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
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
        crate::blocks::helpers::stamp_updated(&mut data);

        match db::update(ctx, USERS_COLLECTION, user_id, data).await {
            Ok(user) => {
                let roles = get_user_roles(ctx, user_id).await;
                json_respond(msg, &serde_json::json!({
                    "id": user.id,
                    "email": user.str_field("email"),
                    "name": user.str_field("name"),
                    "roles": roles
                }))
            }
            Err(e) => err_internal(msg, &format!("Update failed: {e}")),
        }
    }

    pub(super) async fn handle_change_password(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
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

        let stored_hash = user.str_field("password_hash");
        if crypto::compare_hash(ctx, &body.current_password, stored_hash).await.is_err() {
            return error_response(msg, ErrorCode::InvalidCredentials, "Current password is incorrect");
        }

        let new_hash = match crypto::hash(ctx, &body.new_password).await {
            Ok(h) => h,
            Err(e) => return err_internal(msg, &format!("Hash failed: {e}")),
        };

        let mut data = json_map(serde_json::json!({"password_hash": new_hash}));
        crate::blocks::helpers::stamp_updated(&mut data);

        match db::update(ctx, USERS_COLLECTION, user_id, data).await {
            Ok(_) => json_respond(msg, &serde_json::json!({"message": "Password changed successfully"})),
            Err(e) => err_internal(msg, &format!("Update failed: {e}")),
        }
    }

    pub(super) async fn handle_sync_user(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        // Internal endpoint for OAuth user sync — requires INTERNAL_SECRET
        let expected_secret = wafer_core::clients::config::get_default(ctx, "INTERNAL_SECRET", "").await;
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
                let mut data = json_map(serde_json::json!({
                    "email": email_lower,
                    "name": body.name.unwrap_or_default(),
                    "oauth_provider": body.provider.unwrap_or_default(),
                    "disabled": false
                }));
                crate::blocks::helpers::stamp_created(&mut data);
                match db::create(ctx, USERS_COLLECTION, data).await {
                    Ok(u) => u,
                    Err(e) => return err_internal(msg, &format!("Create failed: {e}")),
                }
            }
        };

        json_respond(msg, &serde_json::json!({"id": user.id, "email": user.data.get("email")}))
    }
}
