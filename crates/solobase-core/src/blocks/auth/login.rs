use super::helpers::*;
use super::{AuthBlock, USERS_COLLECTION};
use crate::blocks::errors::{error_response, ErrorCode};
use crate::blocks::helpers::{
    err_bad_request, err_forbidden, err_internal, err_not_found, err_unauthorized, hex_encode,
    json_map, ok_json, RecordExt, ResponseBuilder,
};
use crate::ui;
use maud::{html, PreEscaped};
use std::collections::HashMap;
use wafer_core::clients::database as db;
use wafer_core::clients::{config, crypto};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::{InputStream, OutputStream};

impl AuthBlock {
    pub(super) async fn handle_login(
        &self,
        ctx: &dyn Context,
        input: InputStream,
    ) -> OutputStream {
        #[derive(serde::Deserialize)]
        struct LoginReq {
            email: String,
            password: String,
        }
        let raw = input.collect_to_bytes().await;
        let body: LoginReq = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
        };

        let email_lower = body.email.trim().to_lowercase();

        // Find user by email
        let user = db::get_by_field(
            ctx,
            USERS_COLLECTION,
            "email",
            serde_json::Value::String(email_lower.clone()),
        )
        .await;

        // Always run Argon2 verification to prevent timing-based user enumeration.
        // If user not found, compare against a dummy hash so the response time
        // is indistinguishable from a wrong-password attempt.
        let stored_hash = match &user {
            Ok(u) => u.str_field("password_hash"),
            Err(_) => super::DUMMY_HASH,
        };
        let password_ok = crypto::compare_hash(ctx, &body.password, stored_hash)
            .await
            .is_ok();

        let user = match user {
            Ok(u) if password_ok => u,
            _ => {
                return error_response(
                    ErrorCode::InvalidCredentials,
                    "Invalid email or password",
                )
            }
        };

        // Check if user is disabled
        if user.bool_field("disabled") {
            return error_response(ErrorCode::AccountDisabled, "Account is disabled");
        }

        // Check email verification if required
        let require_verification =
            config::get_default(ctx, "SUPPERS_AI__AUTH__REQUIRE_VERIFICATION", "false").await;
        if (require_verification == "true" || require_verification == "1")
            && !user.bool_field("email_verified")
        {
            return error_response(ErrorCode::EmailNotVerified, "Please verify your email before logging in. Check your inbox for the verification link.");
        }

        // Get roles
        let roles = get_user_roles(ctx, &user.id).await;

        // Generate tokens
        let (access_token, refresh_token, family) =
            match generate_tokens(ctx, &user.id, &email_lower, &roles).await {
                Ok(t) => t,
                Err(r) => return r,
            };

        // Store refresh token
        store_refresh_token(ctx, &user.id, &refresh_token, &family).await;

        // Update last login
        let upd =
            json_map(serde_json::json!({"last_login_at": crate::blocks::helpers::now_rfc3339()}));
        if let Err(e) = db::update(ctx, USERS_COLLECTION, &user.id, upd).await {
            tracing::warn!("Failed to update last login time: {e}");
        }

        let cookie = build_auth_cookie(&access_token, 86400, ctx).await;

        ResponseBuilder::new()
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

    pub(super) async fn handle_signup(
        &self,
        ctx: &dyn Context,
        input: InputStream,
    ) -> OutputStream {
        // Enforce ALLOW_SIGNUP on the API (not just the page)
        let allow_signup = config::get_default(ctx, "SOLOBASE_SHARED__ALLOW_SIGNUP", "true").await;
        if allow_signup != "true" && allow_signup != "1" {
            return error_response(ErrorCode::Forbidden, "Signups are currently disabled");
        }

        #[derive(serde::Deserialize)]
        struct SignupReq {
            email: String,
            password: String,
            name: Option<String>,
        }
        let raw = input.collect_to_bytes().await;
        let body: SignupReq = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
        };

        let email_lower = body.email.trim().to_lowercase();
        let parts: Vec<&str> = email_lower.splitn(2, '@').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() || !parts[1].contains('.')
        {
            return error_response(ErrorCode::InvalidEmail, "Invalid email address");
        }

        // Check allowed email domains (if configured)
        let allowed_domains =
            config::get_default(ctx, "SUPPERS_AI__AUTH__ALLOWED_EMAIL_DOMAINS", "").await;
        if !allowed_domains.is_empty() {
            let email_domain = parts[1];
            let allowed = allowed_domains.split(',').any(|d| d.trim() == email_domain);
            if !allowed {
                return error_response(
                    ErrorCode::InvalidEmail,
                    "Signups from this email domain are not allowed",
                );
            }
        }

        if body.password.len() < 8 {
            return error_response(
                ErrorCode::PasswordTooShort,
                "Password must be at least 8 characters",
            );
        }
        if body.password.len() > 1024 {
            return error_response(
                ErrorCode::PasswordTooLong,
                "Password must not exceed 1024 characters",
            );
        }
        if body.password.chars().any(|c| c.is_control()) {
            return error_response(
                ErrorCode::InvalidInput,
                "Password must not contain control characters",
            );
        }
        if email_lower.len() > 255 {
            return error_response(
                ErrorCode::InvalidEmail,
                "Email must not exceed 255 characters",
            );
        }
        if let Some(ref name) = body.name {
            if name.len() > 200 {
                return error_response(
                    ErrorCode::InvalidInput,
                    "Name must not exceed 200 characters",
                );
            }
        }

        // Check if user exists
        if db::get_by_field(
            ctx,
            USERS_COLLECTION,
            "email",
            serde_json::Value::String(email_lower.clone()),
        )
        .await
        .is_ok()
        {
            return error_response(
                ErrorCode::EmailAlreadyExists,
                "Email already registered",
            );
        }

        // Hash password
        let password_hash = match crypto::hash(ctx, &body.password).await {
            Ok(h) => h,
            Err(e) => return err_internal(&format!("Failed to hash password: {e}")),
        };

        // Check if email verification is required
        let require_verification =
            config::get_default(ctx, "SUPPERS_AI__AUTH__REQUIRE_VERIFICATION", "false").await;
        let require_verification = require_verification == "true" || require_verification == "1";

        // Generate verification token if needed
        let verification_token = if require_verification {
            match crypto::random_bytes(ctx, 32).await {
                Ok(bytes) => hex_encode(&bytes),
                Err(e) => {
                    return err_internal(&format!("Failed to generate verification token: {e}"))
                }
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
            Err(e) => return err_internal(&e),
        };

        let roles = vec![default_role];

        // Send verification email if required
        if require_verification {
            send_verification_email(ctx, &email_lower, &verification_token).await;
            // Do NOT issue tokens before email is verified
            return ResponseBuilder::new()
                .status(201)
                .json(&serde_json::json!({
                    "email_verified": false,
                    "message": "Account created. Please verify your email before signing in.",
                    "user": {
                        "id": user.id,
                        "email": email_lower,
                    }
                }));
        }

        // Generate tokens (only when email verification is NOT required)
        let (access_token, refresh_token, family) =
            match generate_tokens(ctx, &user.id, &email_lower, &roles).await {
                Ok(t) => t,
                Err(r) => return r,
            };

        store_refresh_token(ctx, &user.id, &refresh_token, &family).await;

        let cookie = build_auth_cookie(&access_token, 86400, ctx).await;

        ResponseBuilder::new()
            .status(201)
            .set_cookie(&cookie)
            .json(&serde_json::json!({
                "access_token": access_token,
                "refresh_token": refresh_token,
                "token_type": "Bearer",
                "expires_in": 86400,
                "email_verified": true,
                "user": {
                    "id": user.id,
                    "email": email_lower,
                    "roles": roles,
                    "name": user.str_field("name")
                }
            }))
    }

    pub(super) async fn handle_refresh(
        &self,
        ctx: &dyn Context,
        input: InputStream,
    ) -> OutputStream {
        #[derive(serde::Deserialize)]
        struct RefreshReq {
            refresh_token: String,
        }
        let raw = input.collect_to_bytes().await;
        let body: RefreshReq = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
        };

        // Verify refresh token
        let claims = match crypto::verify(ctx, &body.refresh_token).await {
            Ok(c) => c,
            Err(_) => {
                return error_response(
                    ErrorCode::InvalidToken,
                    "Invalid or expired refresh token",
                )
            }
        };

        let user_id = claims
            .get("user_id")
            .or_else(|| claims.get("sub"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if user_id.is_empty() {
            return error_response(ErrorCode::InvalidToken, "Invalid refresh token");
        }

        let token_type = claims.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if token_type != "refresh" {
            return error_response(ErrorCode::InvalidToken, "Not a refresh token");
        }

        // Validate refresh token exists in DB (prevents use of revoked tokens)
        let token_filter = wafer_core::clients::database::Filter {
            field: "token".to_string(),
            operator: wafer_core::clients::database::FilterOp::Equal,
            value: serde_json::Value::String(body.refresh_token.clone()),
        };
        let token_opts = wafer_core::clients::database::ListOptions {
            filters: vec![token_filter],
            limit: 1,
            ..Default::default()
        };
        match db::list(ctx, super::TOKENS_COLLECTION, &token_opts).await {
            Ok(result) if !result.records.is_empty() => {} // Token exists — proceed
            _ => {
                return error_response(
                    ErrorCode::InvalidToken,
                    "Refresh token has been revoked",
                )
            }
        }

        // Get user and verify account is still active
        let user = match db::get(ctx, USERS_COLLECTION, &user_id).await {
            Ok(u) => u,
            Err(_) => return error_response(ErrorCode::NotAuthenticated, "User not found"),
        };

        if user.bool_field("disabled") {
            return error_response(ErrorCode::AccountDisabled, "Account is disabled");
        }

        let require_verification =
            config::get_default(ctx, "SUPPERS_AI__AUTH__REQUIRE_VERIFICATION", "false").await;
        if (require_verification == "true" || require_verification == "1")
            && !user.bool_field("email_verified")
        {
            return error_response(ErrorCode::EmailNotVerified, "Email not verified");
        }

        let email = user.str_field("email").to_string();
        let roles = get_user_roles(ctx, &user_id).await;

        // Revoke old refresh token family and issue new
        let family = claims
            .get("family")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !family.is_empty() {
            db::delete_by_field(
                ctx,
                super::TOKENS_COLLECTION,
                "family",
                serde_json::Value::String(family),
            )
            .await
            .ok();
        }

        let (access_token, refresh_token, new_family) =
            match generate_tokens(ctx, &user_id, &email, &roles).await {
                Ok(t) => t,
                Err(r) => return r,
            };

        store_refresh_token(ctx, &user_id, &refresh_token, &new_family).await;

        let cookie = build_auth_cookie(&access_token, 86400, ctx).await;

        ResponseBuilder::new()
            .set_cookie(&cookie)
            .json(&serde_json::json!({
                "access_token": access_token,
                "refresh_token": refresh_token,
                "token_type": "Bearer",
                "expires_in": 86400
            }))
    }

    pub(super) async fn handle_logout(&self, ctx: &dyn Context, msg: &Message) -> OutputStream {
        let user_id = msg.user_id();
        if !user_id.is_empty() {
            db::delete_by_field(
                ctx,
                super::TOKENS_COLLECTION,
                "user_id",
                serde_json::Value::String(user_id.to_string()),
            )
            .await
            .ok();
        }

        let cookie = build_auth_cookie("", 0, ctx).await;
        ResponseBuilder::new()
            .set_cookie(&cookie)
            .status(303)
            .set_header("Location", "/b/auth/login")
            .json(&serde_json::json!({"message": "Logged out successfully"}))
    }

    pub(super) async fn handle_me_get(&self, ctx: &dyn Context, msg: &Message) -> OutputStream {
        let user_id = msg.user_id();
        if user_id.is_empty() {
            return error_response(ErrorCode::NotAuthenticated, "Not authenticated");
        }
        let user = match db::get(ctx, USERS_COLLECTION, user_id).await {
            Ok(u) => u,
            Err(_) => return err_not_found("User not found"),
        };
        let roles = get_user_roles(ctx, user_id).await;
        ok_json(&serde_json::json!({
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

    pub(super) async fn handle_me_update(
        &self,
        ctx: &dyn Context,
        msg: &Message,
        input: InputStream,
    ) -> OutputStream {
        let user_id = msg.user_id();
        if user_id.is_empty() {
            return error_response(ErrorCode::NotAuthenticated, "Not authenticated");
        }

        let raw = input.collect_to_bytes().await;
        let body: HashMap<String, serde_json::Value> = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
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
                ok_json(&serde_json::json!({
                    "id": user.id,
                    "email": user.str_field("email"),
                    "name": user.str_field("name"),
                    "roles": roles
                }))
            }
            Err(e) => err_internal(&format!("Update failed: {e}")),
        }
    }

    pub(super) async fn handle_change_password(
        &self,
        ctx: &dyn Context,
        msg: &Message,
        input: InputStream,
    ) -> OutputStream {
        let user_id = msg.user_id();
        if user_id.is_empty() {
            return error_response(ErrorCode::NotAuthenticated, "Not authenticated");
        }

        #[derive(serde::Deserialize)]
        struct ChangePwReq {
            current_password: String,
            new_password: String,
        }
        let raw = input.collect_to_bytes().await;
        let body: ChangePwReq = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
        };

        if body.new_password.len() < 8 {
            return error_response(
                ErrorCode::PasswordTooShort,
                "New password must be at least 8 characters",
            );
        }
        if body.new_password.len() > 1024 {
            return error_response(
                ErrorCode::PasswordTooLong,
                "Password must not exceed 1024 characters",
            );
        }

        let user = match db::get(ctx, USERS_COLLECTION, user_id).await {
            Ok(u) => u,
            Err(_) => return err_not_found("User not found"),
        };

        let stored_hash = user.str_field("password_hash");
        if crypto::compare_hash(ctx, &body.current_password, stored_hash)
            .await
            .is_err()
        {
            return error_response(
                ErrorCode::InvalidCredentials,
                "Current password is incorrect",
            );
        }

        let new_hash = match crypto::hash(ctx, &body.new_password).await {
            Ok(h) => h,
            Err(e) => return err_internal(&format!("Hash failed: {e}")),
        };

        let mut data = json_map(serde_json::json!({"password_hash": new_hash}));
        crate::blocks::helpers::stamp_updated(&mut data);

        match db::update(ctx, USERS_COLLECTION, user_id, data).await {
            Ok(_) => {
                // Revoke all refresh tokens — force re-login with new password
                db::delete_by_field(
                    ctx,
                    super::TOKENS_COLLECTION,
                    "user_id",
                    serde_json::Value::String(user_id.to_string()),
                )
                .await
                .ok();
                ok_json(&serde_json::json!({"message": "Password changed successfully"}))
            }
            Err(e) => err_internal(&format!("Update failed: {e}")),
        }
    }

    pub(super) async fn handle_sync_user(
        &self,
        ctx: &dyn Context,
        msg: &Message,
        input: InputStream,
    ) -> OutputStream {
        // Internal endpoint for OAuth user sync — requires INTERNAL_SECRET
        let expected_secret =
            wafer_core::clients::config::get_default(ctx, "SUPPERS_AI__AUTH__INTERNAL_SECRET", "")
                .await;
        if expected_secret.is_empty() {
            return err_forbidden(
                "INTERNAL_SECRET not configured — internal endpoints are disabled",
            );
        }
        let provided_secret = msg.header("x-internal-secret");
        if !crate::crypto::constant_time_eq(provided_secret.as_bytes(), expected_secret.as_bytes())
        {
            return err_unauthorized("Invalid internal secret");
        }

        #[derive(serde::Deserialize)]
        struct SyncReq {
            email: String,
            name: Option<String>,
            provider: Option<String>,
        }
        let raw = input.collect_to_bytes().await;
        let body: SyncReq = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
        };

        let email_lower = body.email.trim().to_lowercase();
        let user = match db::get_by_field(
            ctx,
            USERS_COLLECTION,
            "email",
            serde_json::Value::String(email_lower.clone()),
        )
        .await
        {
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
                    Err(e) => return err_internal(&format!("Create failed: {e}")),
                }
            }
        };

        ok_json(&serde_json::json!({"id": user.id, "email": user.data.get("email")}))
    }

    pub(super) async fn handle_verify_email(
        &self,
        ctx: &dyn Context,
        msg: &Message,
        input: InputStream,
    ) -> OutputStream {
        let logo_url = db::get_by_field(
            ctx,
            crate::blocks::admin::VARIABLES_COLLECTION,
            "key",
            serde_json::Value::String("SOLOBASE_SHARED__AUTH_LOGO_URL".into()),
        )
        .await
        .map(|r| r.str_field("value").to_string())
        .unwrap_or_default();

        // Token comes from query param or body
        let token = {
            let q = msg.get_meta("req.query.token").to_string();
            if !q.is_empty() {
                q
            } else {
                #[derive(serde::Deserialize)]
                struct Req {
                    token: String,
                }
                let raw = input.collect_to_bytes().await;
                match serde_json::from_slice::<Req>(&raw) {
                    Ok(r) => r.token,
                    Err(_) => return err_bad_request("Missing verification token"),
                }
            }
        };

        if token.is_empty() {
            return err_bad_request("Missing verification token");
        }

        // Find user by verification token
        let user = match db::get_by_field(
            ctx,
            USERS_COLLECTION,
            "verification_token",
            serde_json::Value::String(token.clone()),
        )
        .await
        {
            Ok(u) => u,
            Err(_) => {
                return html_respond(
                    "Invalid Link",
                    "This verification link is invalid or has expired. Please request a new one.",
                    false,
                    &logo_url,
                )
            }
        };

        if user.bool_field("email_verified") {
            return html_respond(
                "Email Already Verified",
                "Your email has already been verified. You can sign in now.",
                true,
                &logo_url,
            );
        }

        // Mark as verified, clear token
        let mut data = json_map(serde_json::json!({
            "email_verified": true,
            "verification_token": ""
        }));
        crate::blocks::helpers::stamp_updated(&mut data);

        if let Err(e) = db::update(ctx, USERS_COLLECTION, &user.id, data).await {
            return err_internal(&format!("Failed to verify email: {e}"));
        }

        html_respond(
            "Email Verified",
            "Your email has been verified successfully. You can now sign in.",
            true,
            &logo_url,
        )
    }

    pub(super) async fn handle_resend_verification(
        &self,
        ctx: &dyn Context,
        input: InputStream,
    ) -> OutputStream {
        #[derive(serde::Deserialize)]
        struct Req {
            email: String,
        }
        let raw = input.collect_to_bytes().await;
        let body: Req = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
        };

        let email_lower = body.email.trim().to_lowercase();
        let safe_msg = "If that email is registered, a verification link has been sent.";

        let user = match db::get_by_field(
            ctx,
            USERS_COLLECTION,
            "email",
            serde_json::Value::String(email_lower.clone()),
        )
        .await
        {
            Ok(u) => u,
            Err(_) => return ok_json(&serde_json::json!({"message": safe_msg})),
        };

        if user.bool_field("email_verified") {
            return ok_json(&serde_json::json!({"message": "Email is already verified."}));
        }

        // Rate limit: 60 second cooldown
        let last_sent = user.str_field("last_verification_sent");
        if !last_sent.is_empty() {
            if let Ok(last) = chrono::DateTime::parse_from_rfc3339(last_sent) {
                let elapsed = chrono::Utc::now() - last.with_timezone(&chrono::Utc);
                let remaining = 60 - elapsed.num_seconds();
                if remaining > 0 {
                    return ok_json(&serde_json::json!({
                        "message": format!("Please wait {} seconds before requesting another email.", remaining),
                        "retry_after": remaining
                    }));
                }
            }
        }

        // Generate new token
        let new_token = match crypto::random_bytes(ctx, 32).await {
            Ok(bytes) => hex_encode(&bytes),
            Err(e) => return err_internal(&format!("Token generation failed: {e}")),
        };

        let now = crate::blocks::helpers::now_rfc3339();
        let mut data = json_map(serde_json::json!({
            "verification_token": new_token,
            "last_verification_sent": now
        }));
        crate::blocks::helpers::stamp_updated(&mut data);

        if let Err(e) = db::update(ctx, USERS_COLLECTION, &user.id, data).await {
            return err_internal(&format!("Failed to update token: {e}"));
        }

        send_verification_email(ctx, &email_lower, &new_token).await;

        ok_json(&serde_json::json!({"message": safe_msg}))
    }

    pub(super) async fn handle_forgot_password(
        &self,
        ctx: &dyn Context,
        input: InputStream,
    ) -> OutputStream {
        #[derive(serde::Deserialize)]
        struct Req {
            email: String,
        }
        let raw = input.collect_to_bytes().await;
        let body: Req = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
        };

        let email_lower = body.email.trim().to_lowercase();
        let safe_msg = "If that email is registered, a password reset link has been sent.";

        let user = match db::get_by_field(
            ctx,
            USERS_COLLECTION,
            "email",
            serde_json::Value::String(email_lower.clone()),
        )
        .await
        {
            Ok(u) => u,
            Err(_) => return ok_json(&serde_json::json!({"message": safe_msg})),
        };

        // Generate reset token (expires in 1 hour)
        let reset_token = match crypto::random_bytes(ctx, 32).await {
            Ok(bytes) => hex_encode(&bytes),
            Err(e) => return err_internal(&format!("Token generation failed: {e}")),
        };

        let expires = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let mut data = json_map(serde_json::json!({
            "reset_token": reset_token,
            "reset_token_expires": expires
        }));
        crate::blocks::helpers::stamp_updated(&mut data);

        if let Err(e) = db::update(ctx, USERS_COLLECTION, &user.id, data).await {
            return err_internal(&format!("Failed to store reset token: {e}"));
        }

        // Send reset email
        send_reset_email(ctx, &email_lower, &reset_token).await;

        ok_json(&serde_json::json!({"message": safe_msg}))
    }

    pub(super) async fn handle_reset_password_form(
        &self,
        ctx: &dyn Context,
        msg: &Message,
    ) -> OutputStream {
        let logo_url = db::get_by_field(
            ctx,
            crate::blocks::admin::VARIABLES_COLLECTION,
            "key",
            serde_json::Value::String("SOLOBASE_SHARED__AUTH_LOGO_URL".into()),
        )
        .await
        .map(|r| r.str_field("value").to_string())
        .unwrap_or_default();

        let token = msg.get_meta("req.query.token").to_string();
        if token.is_empty() {
            return html_respond(
                "Invalid Link",
                "This password reset link is invalid.",
                false,
                &logo_url,
            );
        }

        let config = ui::SiteConfig {
            app_name: "Solobase".into(),
            logo_url: logo_url.clone(),
            logo_icon_url: String::new(),
            favicon_url: String::new(),
        };

        let markup = ui::layout::centered_page(
            "Reset Password",
            &config,
            html! {
                div .login-container {
                    div .login-logo {
                        @if !logo_url.is_empty() {
                            img .logo-image src=(logo_url) alt="Solobase";
                        }
                        p .login-subtitle { "Reset your password" }
                    }

                    div #error .login-error style="display:none" {}
                    div #success style="background:#ecfdf5;border:1px solid #a7f3d0;border-radius:8px;padding:.75rem;margin-bottom:1rem;font-size:.813rem;color:#059669;display:none" {}

                    form #form .login-form onsubmit="return handleReset(event)" {
                        input type="hidden" #reset-token name="token" value=(token);

                        div .form-group {
                            label .form-label for="password" { "New Password" }
                            input .form-input type="password" #password required minlength="8" placeholder="Min 8 characters";
                        }
                        div .form-group {
                            label .form-label for="confirm" { "Confirm Password" }
                            input .form-input type="password" #confirm required minlength="8" placeholder="Repeat password";
                        }

                        button .login-button type="submit" #btn { "Reset Password" }
                    }
                }

                script { (PreEscaped(r#"
var $=function(id){return document.getElementById(id)};
async function handleReset(e){
  e.preventDefault();
  var pw=$('password').value,cf=$('confirm').value;
  var err=$('error'),suc=$('success'),btn=$('btn');
  var token=$('reset-token').value;
  err.style.display='none';suc.style.display='none';
  if(pw!==cf){err.textContent='Passwords do not match.';err.style.display='flex';return false;}
  if(pw.length<8){err.textContent='Password must be at least 8 characters.';err.style.display='flex';return false;}
  btn.disabled=true;btn.textContent='Resetting...';
  try{
    var r=await fetch('/b/auth/api/reset-password',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({token:token,new_password:pw})});
    var d=await r.json();
    if(d.error){err.textContent=d.error.message||d.error;err.style.display='flex';}
    else{suc.textContent='Password reset successfully. You can now sign in.';suc.style.display='block';$('form').style.display='none';
      setTimeout(function(){window.location.href='/b/auth/login';},2000);}
  }catch(ex){err.textContent='Something went wrong.';err.style.display='flex';}
  btn.disabled=false;btn.textContent='Reset Password';
  return false;
}
"#)) }
            },
        );

        ui::html_response(markup)
    }

    pub(super) async fn handle_reset_password(
        &self,
        ctx: &dyn Context,
        input: InputStream,
    ) -> OutputStream {
        #[derive(serde::Deserialize)]
        struct Req {
            token: String,
            new_password: String,
        }
        let raw = input.collect_to_bytes().await;
        let body: Req = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
        };

        if body.new_password.len() < 8 {
            return error_response(
                ErrorCode::PasswordTooShort,
                "Password must be at least 8 characters",
            );
        }
        if body.new_password.len() > 1024 {
            return error_response(
                ErrorCode::PasswordTooLong,
                "Password must not exceed 1024 characters",
            );
        }

        // Find user by reset token
        let user = match db::get_by_field(
            ctx,
            USERS_COLLECTION,
            "reset_token",
            serde_json::Value::String(body.token.clone()),
        )
        .await
        {
            Ok(u) => u,
            Err(_) => {
                return error_response(
                    ErrorCode::InvalidToken,
                    "Invalid or expired reset token",
                )
            }
        };

        // Check expiry — reject if missing or malformed (tokens must have an expiry)
        let expires = user.str_field("reset_token_expires");
        if expires.is_empty() {
            return error_response(
                ErrorCode::TokenExpired,
                "Reset token has expired. Please request a new one.",
            );
        }
        match chrono::DateTime::parse_from_rfc3339(expires) {
            Ok(exp) => {
                if chrono::Utc::now() > exp.with_timezone(&chrono::Utc) {
                    return error_response(
                        ErrorCode::TokenExpired,
                        "Reset token has expired. Please request a new one.",
                    );
                }
            }
            Err(_) => {
                return error_response(
                    ErrorCode::TokenExpired,
                    "Reset token has expired. Please request a new one.",
                );
            }
        }

        // Hash new password
        let new_hash = match crypto::hash(ctx, &body.new_password).await {
            Ok(h) => h,
            Err(e) => return err_internal(&format!("Hash failed: {e}")),
        };

        // Update password, clear reset token
        let mut data = json_map(serde_json::json!({
            "password_hash": new_hash,
            "reset_token": "",
            "reset_token_expires": ""
        }));
        crate::blocks::helpers::stamp_updated(&mut data);

        if let Err(e) = db::update(ctx, USERS_COLLECTION, &user.id, data).await {
            return err_internal(&format!("Failed to update password: {e}"));
        }

        // Revoke all refresh tokens — invalidate any stolen sessions
        db::delete_by_field(
            ctx,
            super::TOKENS_COLLECTION,
            "user_id",
            serde_json::Value::String(user.id.clone()),
        )
        .await
        .ok();

        ok_json(&serde_json::json!({"message": "Password reset successfully"}))
    }
}

/// Return an HTML page response (for verify/reset endpoints opened in browser).
fn html_respond(title: &str, message: &str, success: bool, logo_url: &str) -> OutputStream {
    let color = if success { "#10b981" } else { "#ef4444" };
    let config = ui::SiteConfig {
        app_name: "Solobase".into(),
        logo_url: logo_url.to_string(),
        logo_icon_url: String::new(),
        favicon_url: String::new(),
    };
    let markup = ui::layout::centered_page(
        title,
        &config,
        html! {
            div .login-container {
                div .login-logo {
                    @if !logo_url.is_empty() {
                        img .logo-image src=(logo_url) alt="Solobase";
                    }
                }
                div style="text-align:center" {
                    div style={"width:48px;height:48px;background:" (color) "15;border-radius:50%;display:flex;align-items:center;justify-content:center;margin:0 auto 1rem;font-size:1.5rem;color:" (color)} {
                        @if success { "✓" } @else { "✗" }
                    }
                    h2 style="font-size:1.25rem;font-weight:700;margin:0 0 .5rem" { (title) }
                    p .login-subtitle style="line-height:1.6;margin:0 0 1.5rem" { (message) }
                    a .login-button href="/b/auth/login" style="display:inline-block;width:auto;padding:.625rem 1.25rem;text-decoration:none" {
                        "Go to Sign In"
                    }
                }
            }
        },
    );
    ui::html_response(markup)
}

/// Send verification email via the email block.
async fn send_verification_email(ctx: &dyn Context, email: &str, token: &str) {
    send_template_email(ctx, "verification", email, Some(token), None).await;
}

/// Send password reset email via the email block.
async fn send_reset_email(ctx: &dyn Context, email: &str, token: &str) {
    send_template_email(ctx, "password_reset", email, Some(token), None).await;
}

async fn send_template_email(
    ctx: &dyn Context,
    template: &str,
    to: &str,
    token: Option<&str>,
    name: Option<&str>,
) {
    let mut req = serde_json::json!({ "template": template, "to": to });
    if let Some(t) = token {
        req["token"] = serde_json::Value::String(t.to_string());
    }
    if let Some(n) = name {
        req["name"] = serde_json::Value::String(n.to_string());
    }
    let email_msg = Message {
        kind: "email.send_template".to_string(),
        meta: Vec::new(),
    };
    let body_bytes = serde_json::to_vec(&req).unwrap_or_default();
    let out = ctx
        .call_block(
            "suppers-ai/email",
            email_msg,
            InputStream::from_bytes(body_bytes),
        )
        .await;
    match out.collect_buffered().await {
        Ok(_) => {}
        Err(e) => {
            tracing::warn!("Failed to send {} email to {}: {:?}", template, to, e);
        }
    }
}
