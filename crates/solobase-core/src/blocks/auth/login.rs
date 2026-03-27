use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::{crypto, config};
use super::helpers::*;
use super::pages::esc;
use super::{AuthBlock, USERS_COLLECTION};
use crate::blocks::errors::{ErrorCode, error_response};
use crate::blocks::helpers::{RecordExt, json_map, hex_encode};

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

        // Check email verification if required
        let require_verification = config::get_default(ctx, "AUTH_REQUIRE_VERIFICATION", "false").await;
        if (require_verification == "true" || require_verification == "1") && !user.bool_field("email_verified") {
            return error_response(msg, ErrorCode::EmailNotVerified, "Please verify your email before logging in. Check your inbox for the verification link.");
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

        // Check allowed email domains (if configured)
        let allowed_domains = config::get_default(ctx, "AUTH_ALLOWED_EMAIL_DOMAINS", "").await;
        if !allowed_domains.is_empty() {
            let email_domain = parts[1];
            let allowed = allowed_domains.split(',').any(|d| d.trim() == email_domain);
            if !allowed {
                return error_response(msg, ErrorCode::InvalidEmail, "Signups from this email domain are not allowed");
            }
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

        // Check if email verification is required
        let require_verification = config::get_default(ctx, "AUTH_REQUIRE_VERIFICATION", "false").await;
        let require_verification = require_verification == "true" || require_verification == "1";

        // Generate verification token if needed
        let verification_token = if require_verification {
            match crypto::random_bytes(ctx, 32).await {
                Ok(bytes) => hex_encode(&bytes),
                Err(e) => return err_internal(msg, &format!("Failed to generate verification token: {e}")),
            }
        } else {
            String::new()
        };

        let mut data = json_map(serde_json::json!({
            "email": email_lower,
            "password_hash": password_hash,
            "name": body.name.unwrap_or_default(),
            "disabled": false,
            "email_verified": !require_verification,
            "verification_token": verification_token
        }));
        crate::blocks::helpers::stamp_created(&mut data);

        let (user, default_role) = match create_user_and_assign_role(ctx, data).await {
            Ok(r) => r,
            Err(e) => return err_internal(msg, &e),
        };

        let roles = vec![default_role];

        // Send verification email if required
        if require_verification {
            send_verification_email(ctx, &email_lower, &verification_token).await;
        }

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
                "email_verified": !require_verification,
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

    pub(super) async fn handle_verify_email(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let logo_url = db::get_by_field(ctx, "variables", "key", serde_json::Value::String("AUTH_LOGO_URL".into()))
            .await
            .map(|r| r.str_field("value").to_string())
            .unwrap_or_default();
        let logo_url = esc(if logo_url.is_empty() { "https://solobase.dev/images/logo_long.png" } else { &logo_url });

        // Token comes from query param or body
        let token = {
            let q = msg.get_meta("req.query.token").to_string();
            if !q.is_empty() {
                q
            } else {
                #[derive(serde::Deserialize)]
                struct Req { token: String }
                match msg.decode::<Req>() {
                    Ok(r) => r.token,
                    Err(_) => return err_bad_request(msg, "Missing verification token"),
                }
            }
        };

        if token.is_empty() {
            return err_bad_request(msg, "Missing verification token");
        }

        // Find user by verification token
        let user = match db::get_by_field(ctx, USERS_COLLECTION, "verification_token", serde_json::Value::String(token.clone())).await {
            Ok(u) => u,
            Err(_) => return html_respond(msg, "Invalid Link", "This verification link is invalid or has expired. Please request a new one.", false, &logo_url),
        };

        if user.bool_field("email_verified") {
            return html_respond(msg, "Email Already Verified", "Your email has already been verified. You can sign in now.", true, &logo_url);
        }

        // Mark as verified, clear token
        let mut data = json_map(serde_json::json!({
            "email_verified": true,
            "verification_token": ""
        }));
        crate::blocks::helpers::stamp_updated(&mut data);

        if let Err(e) = db::update(ctx, USERS_COLLECTION, &user.id, data).await {
            return err_internal(msg, &format!("Failed to verify email: {e}"));
        }

        html_respond(msg, "Email Verified", "Your email has been verified successfully. You can now sign in.", true, &logo_url)
    }

    pub(super) async fn handle_resend_verification(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        #[derive(serde::Deserialize)]
        struct Req { email: String }
        let body: Req = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
        };

        let email_lower = body.email.trim().to_lowercase();
        let safe_msg = "If that email is registered, a verification link has been sent.";

        let user = match db::get_by_field(ctx, USERS_COLLECTION, "email", serde_json::Value::String(email_lower.clone())).await {
            Ok(u) => u,
            Err(_) => return json_respond(msg, &serde_json::json!({"message": safe_msg})),
        };

        if user.bool_field("email_verified") {
            return json_respond(msg, &serde_json::json!({"message": "Email is already verified."}));
        }

        // Rate limit: 60 second cooldown
        let last_sent = user.str_field("last_verification_sent");
        if !last_sent.is_empty() {
            if let Ok(last) = chrono::DateTime::parse_from_rfc3339(last_sent) {
                let elapsed = chrono::Utc::now() - last.with_timezone(&chrono::Utc);
                let remaining = 60 - elapsed.num_seconds();
                if remaining > 0 {
                    return json_respond(msg, &serde_json::json!({
                        "message": format!("Please wait {} seconds before requesting another email.", remaining),
                        "retry_after": remaining
                    }));
                }
            }
        }

        // Generate new token
        let new_token = match crypto::random_bytes(ctx, 32).await {
            Ok(bytes) => hex_encode(&bytes),
            Err(e) => return err_internal(msg, &format!("Token generation failed: {e}")),
        };

        let now = crate::blocks::helpers::now_rfc3339();
        let mut data = json_map(serde_json::json!({
            "verification_token": new_token,
            "last_verification_sent": now
        }));
        crate::blocks::helpers::stamp_updated(&mut data);

        if let Err(e) = db::update(ctx, USERS_COLLECTION, &user.id, data).await {
            return err_internal(msg, &format!("Failed to update token: {e}"));
        }

        send_verification_email(ctx, &email_lower, &new_token).await;

        json_respond(msg, &serde_json::json!({"message": safe_msg}))
    }

    pub(super) async fn handle_forgot_password(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        #[derive(serde::Deserialize)]
        struct Req { email: String }
        let body: Req = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
        };

        let email_lower = body.email.trim().to_lowercase();
        let safe_msg = "If that email is registered, a password reset link has been sent.";

        let user = match db::get_by_field(ctx, USERS_COLLECTION, "email", serde_json::Value::String(email_lower.clone())).await {
            Ok(u) => u,
            Err(_) => return json_respond(msg, &serde_json::json!({"message": safe_msg})),
        };

        // Generate reset token (expires in 1 hour)
        let reset_token = match crypto::random_bytes(ctx, 32).await {
            Ok(bytes) => hex_encode(&bytes),
            Err(e) => return err_internal(msg, &format!("Token generation failed: {e}")),
        };

        let expires = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let mut data = json_map(serde_json::json!({
            "reset_token": reset_token,
            "reset_token_expires": expires
        }));
        crate::blocks::helpers::stamp_updated(&mut data);

        if let Err(e) = db::update(ctx, USERS_COLLECTION, &user.id, data).await {
            return err_internal(msg, &format!("Failed to store reset token: {e}"));
        }

        // Send reset email
        send_reset_email(ctx, &email_lower, &reset_token).await;

        json_respond(msg, &serde_json::json!({"message": safe_msg}))
    }

    pub(super) async fn handle_reset_password_form(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let logo_url = db::get_by_field(ctx, "variables", "key", serde_json::Value::String("AUTH_LOGO_URL".into()))
            .await
            .map(|r| r.str_field("value").to_string())
            .unwrap_or_default();
        let logo_url = esc(if logo_url.is_empty() { "https://solobase.dev/images/logo_long.png" } else { &logo_url });

        let token = msg.get_meta("req.query.token").to_string();
        if token.is_empty() {
            return html_respond(msg, "Invalid Link", "This password reset link is invalid.", false, &logo_url);
        }

        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Reset Password</title><link rel="icon" type="image/x-icon" href="/favicon.ico"></head>
<body style="margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;background:linear-gradient(135deg,#f0f9ff 0%,#e0f2fe 50%,#f0f9ff 100%);font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif">
<div style="width:100%;max-width:420px;padding:2rem;text-align:center">
<img src="{logo_url}" alt="Solobase" style="height:42px;display:block;margin:0 auto 1.5rem">
<div style="background:white;border-radius:12px;padding:2rem;box-shadow:0 1px 3px rgba(0,0,0,.1)">
<h2 style="font-size:1.25rem;font-weight:700;color:#1e293b;margin:0 0 .5rem">Reset Your Password</h2>
<p style="font-size:.875rem;color:#64748b;margin:0 0 1.5rem">Enter your new password below.</p>
<div id="error" style="display:none;background:#fef2f2;border:1px solid #fecaca;border-radius:8px;padding:.75rem;margin-bottom:1rem;font-size:.813rem;color:#dc2626"></div>
<div id="success" style="display:none;background:#ecfdf5;border:1px solid #a7f3d0;border-radius:8px;padding:.75rem;margin-bottom:1rem;font-size:.813rem;color:#059669"></div>
<form id="form" onsubmit="return handleReset(event)">
<input type="hidden" name="token" value="{token}">
<div style="margin-bottom:1rem;text-align:left">
<label style="display:block;font-size:.813rem;font-weight:500;color:#1e293b;margin-bottom:.375rem">New Password</label>
<input type="password" id="password" required minlength="8" placeholder="Min 8 characters" style="width:100%;padding:.625rem .75rem;border:1px solid #e2e8f0;border-radius:8px;font-size:.875rem;outline:none;box-sizing:border-box">
</div>
<div style="margin-bottom:1.5rem;text-align:left">
<label style="display:block;font-size:.813rem;font-weight:500;color:#1e293b;margin-bottom:.375rem">Confirm Password</label>
<input type="password" id="confirm" required minlength="8" placeholder="Repeat password" style="width:100%;padding:.625rem .75rem;border:1px solid #e2e8f0;border-radius:8px;font-size:.875rem;outline:none;box-sizing:border-box">
</div>
<button type="submit" id="btn" style="width:100%;padding:.75rem;background:linear-gradient(135deg,#189AB4,#0ea5e9);color:white;border:none;border-radius:8px;font-size:.875rem;font-weight:600;cursor:pointer">Reset Password</button>
</form>
</div></div>
<script>
async function handleReset(e){{
  e.preventDefault();
  var pw=document.getElementById('password').value;
  var cf=document.getElementById('confirm').value;
  var err=document.getElementById('error');
  var suc=document.getElementById('success');
  var btn=document.getElementById('btn');
  err.style.display='none';suc.style.display='none';
  if(pw!==cf){{err.textContent='Passwords do not match.';err.style.display='block';return false;}}
  if(pw.length<8){{err.textContent='Password must be at least 8 characters.';err.style.display='block';return false;}}
  btn.disabled=true;btn.textContent='Resetting...';
  try{{
    var r=await fetch('/auth/reset-password',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{token:'{token}',new_password:pw}})}});
    var d=await r.json();
    if(d.error){{err.textContent=d.error.message||d.error;err.style.display='block';}}
    else{{suc.textContent='Password reset successfully. You can now sign in.';suc.style.display='block';document.getElementById('form').style.display='none';
      setTimeout(function(){{window.location.href='/';}},2000);}}
  }}catch(ex){{err.textContent='Something went wrong.';err.style.display='block';}}
  btn.disabled=false;btn.textContent='Reset Password';
  return false;
}}
</script>
</body></html>"#
        );

        ResponseBuilder::new(msg)
            .body(html.into_bytes(), "text/html; charset=utf-8")
    }

    pub(super) async fn handle_reset_password(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        #[derive(serde::Deserialize)]
        struct Req { token: String, new_password: String }
        let body: Req = match msg.decode() {
            Ok(b) => b,
            Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
        };

        if body.new_password.len() < 8 {
            return error_response(msg, ErrorCode::PasswordTooShort, "Password must be at least 8 characters");
        }
        if body.new_password.len() > 1024 {
            return error_response(msg, ErrorCode::PasswordTooLong, "Password must not exceed 1024 characters");
        }

        // Find user by reset token
        let user = match db::get_by_field(ctx, USERS_COLLECTION, "reset_token", serde_json::Value::String(body.token.clone())).await {
            Ok(u) => u,
            Err(_) => return error_response(msg, ErrorCode::InvalidToken, "Invalid or expired reset token"),
        };

        // Check expiry
        let expires = user.str_field("reset_token_expires");
        if !expires.is_empty() {
            if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(expires) {
                if chrono::Utc::now() > exp.with_timezone(&chrono::Utc) {
                    return error_response(msg, ErrorCode::TokenExpired, "Reset token has expired. Please request a new one.");
                }
            }
        }

        // Hash new password
        let new_hash = match crypto::hash(ctx, &body.new_password).await {
            Ok(h) => h,
            Err(e) => return err_internal(msg, &format!("Hash failed: {e}")),
        };

        // Update password, clear reset token
        let mut data = json_map(serde_json::json!({
            "password_hash": new_hash,
            "reset_token": "",
            "reset_token_expires": ""
        }));
        crate::blocks::helpers::stamp_updated(&mut data);

        if let Err(e) = db::update(ctx, USERS_COLLECTION, &user.id, data).await {
            return err_internal(msg, &format!("Failed to update password: {e}"));
        }

        json_respond(msg, &serde_json::json!({"message": "Password reset successfully"}))
    }
}

/// Return an HTML page response (for verify endpoint, which is opened in browser).
fn html_respond(msg: &mut Message, title: &str, message: &str, success: bool, logo_url: &str) -> Result_ {
    let icon = if success { "&#10003;" } else { "&#10007;" };
    let color = if success { "#10b981" } else { "#ef4444" };
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title}</title><link rel="icon" type="image/x-icon" href="/favicon.ico"></head>
<body style="margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;background:linear-gradient(135deg,#f0f9ff 0%,#e0f2fe 50%,#f0f9ff 100%);font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif">
<div style="width:100%;max-width:420px;padding:2rem;text-align:center">
<img src="{logo_url}" alt="Solobase" style="height:42px;display:block;margin:0 auto 1.5rem">
<div style="background:white;border-radius:12px;padding:2rem;box-shadow:0 1px 3px rgba(0,0,0,.1)">
<div style="width:48px;height:48px;background:{color}15;border-radius:50%;display:flex;align-items:center;justify-content:center;margin:0 auto 1rem;font-size:1.5rem;color:{color}">{icon}</div>
<h2 style="font-size:1.25rem;font-weight:700;color:#1e293b;margin:0 0 .5rem">{title}</h2>
<p style="font-size:.875rem;color:#64748b;line-height:1.6;margin:0 0 1.5rem">{message}</p>
<a href="/" style="display:inline-block;padding:.625rem 1.25rem;background:linear-gradient(135deg,#189AB4,#0ea5e9);color:white;border-radius:8px;font-size:.875rem;font-weight:600;text-decoration:none">Go to Dashboard</a>
</div></div></body></html>"#
    );
    ResponseBuilder::new(msg)
        .body(html.into_bytes(), "text/html; charset=utf-8")
}

/// Send verification email via the email block.
async fn send_verification_email(ctx: &dyn Context, email: &str, token: &str) {
    send_template_email(ctx, "verification", email, Some(token), None).await;
}

/// Send password reset email via the email block.
async fn send_reset_email(ctx: &dyn Context, email: &str, token: &str) {
    send_template_email(ctx, "password_reset", email, Some(token), None).await;
}

async fn send_template_email(ctx: &dyn Context, template: &str, to: &str, token: Option<&str>, name: Option<&str>) {
    let mut req = serde_json::json!({ "template": template, "to": to });
    if let Some(t) = token { req["token"] = serde_json::Value::String(t.to_string()); }
    if let Some(n) = name { req["name"] = serde_json::Value::String(n.to_string()); }
    let mut email_msg = Message {
        kind: "email.send_template".to_string(),
        data: serde_json::to_vec(&req).unwrap_or_default(),
        meta: Vec::new(),
    };
    let result = ctx.call_block("suppers-ai/email", &mut email_msg).await;
    if matches!(result.action, Action::Error) {
        tracing::warn!("Failed to send {} email to {}: {:?}", template, to, result.error);
    }
}
