//! POST /b/auth/api/signup — relocated from auth/login.rs in Task 5.

use wafer_core::clients::{config, crypto, database as db};
use wafer_run::{context::Context, types::Message, InputStream, OutputStream};

use crate::blocks::{
    auth::{
        helpers::{build_auth_cookie, generate_tokens, store_refresh_token},
        repo::{local_credentials, sessions, users},
        service::hash_token,
        USERS_TABLE,
    },
    errors::{error_response, ErrorCode},
    helpers::{err_bad_request, err_internal, hex_encode, json_map, ResponseBuilder},
};

pub async fn handle(ctx: &dyn Context, input: InputStream) -> OutputStream {
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
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() || !parts[1].contains('.') {
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

    // Reject top-25 common passwords. [SEC-041] A length minimum alone lets
    // `password1`, `12345678`, `qwerty12`, etc. through — a credential-stuffing
    // attacker hits these first. The list is intentionally tiny (NordPass
    // 2023 top 25) so the check stays cheap and doesn't drift into HIBP
    // territory in this PR.
    if is_common_password(&body.password) {
        return error_response(
            ErrorCode::InvalidInput,
            "Password is too common. Please choose a less predictable password.",
        );
    }

    // [SEC-035] If the email is already registered, do NOT confirm that to
    // the caller — return the same generic "check your email" response a
    // fresh signup would produce. The signup endpoint is otherwise a free
    // email-enumeration oracle for password-reset / phishing campaigns.
    //
    // Follow-up: send a "someone tried to sign up with your email" notice
    // to the existing account. Not included in this PR — needs the email
    // block's templating to grow a new template, which is out of scope.
    let email_already_taken = db::get_by_field(
        ctx,
        USERS_TABLE,
        "email",
        serde_json::Value::String(email_lower.clone()),
    )
    .await
    .is_ok();
    if email_already_taken {
        return ResponseBuilder::new().status(201).json(&serde_json::json!({
            "email_verified": false,
            "message": "Account created. Please verify your email before signing in.",
            "user": {
                "id": "",
                "email": email_lower,
            }
        }));
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
            Err(e) => return err_internal(&format!("Failed to generate verification token: {e}")),
        }
    } else {
        String::new()
    };

    // Determine the role: admin if the email matches the configured bootstrap
    // admin email (re-uses the same key as bootstrap for consistency).
    let admin_email = config::get_default(
        ctx,
        crate::blocks::auth::config::BOOTSTRAP_ADMIN_EMAIL_KEY,
        "",
    )
    .await;
    let role = if !admin_email.is_empty() && email_lower.eq_ignore_ascii_case(&admin_email) {
        "admin"
    } else {
        "user"
    };

    // Insert via typed repo — no password_hash on the users row.
    let user = match users::insert(
        ctx,
        users::NewUser {
            email: email_lower.clone(),
            display_name: body.name.unwrap_or_default(),
            avatar_url: None,
            role: role.to_string(),
        },
    )
    .await
    {
        Ok(u) => u,
        Err(e) => return err_internal(&format!("Failed to create user: {e}")),
    };

    if let Err(e) = local_credentials::insert(ctx, &user.id, &password_hash, false).await {
        return err_internal(&format!("Failed to store credentials: {e}"));
    }

    // Set email_verified and verification_token on the legacy USERS_TABLE row
    // (Plan A2 users table stores email_verified too — keep them in sync).
    {
        let mut upd = json_map(serde_json::json!({
            "email_verified": !require_verification,
            "verification_token": verification_token.clone(),
        }));
        crate::blocks::helpers::stamp_updated(&mut upd);
        if let Err(e) = db::update(ctx, USERS_TABLE, &user.id, upd).await {
            tracing::warn!("Failed to set email_verified on signup: {e}");
        }
    }

    let roles = vec![role.to_string()];

    // Send verification email if required
    if require_verification {
        send_verification_email(ctx, &email_lower, &verification_token).await;
        // Do NOT issue tokens before email is verified
        return ResponseBuilder::new().status(201).json(&serde_json::json!({
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
        match generate_tokens(ctx, &user.id, &email_lower, &roles, "password").await {
            Ok(t) => t,
            Err(r) => return r,
        };

    store_refresh_token(ctx, &user.id, &refresh_token, &family).await;

    if let Err(e) = sessions::create_for_user(ctx, &user.id, hash_token(&access_token), 1).await {
        tracing::warn!(
            user_id = %user.id,
            "failed to persist session row for JWT signup: {e}"
        );
    }

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
                "name": user.display_name
            }
        }))
}

/// [SEC-041] Top-25 most common passwords from the NordPass 2023 list.
/// Comparison is case-insensitive — `Password1` and `password1` are both
/// rejected. Embedded rather than pulled from a crate to keep dependencies
/// minimal; the list rarely drifts year-over-year and a refresh is cheap.
const COMMON_PASSWORDS: &[&str] = &[
    "123456",
    "admin",
    "12345678",
    "123456789",
    "1234",
    "12345",
    "password",
    "123",
    "aa123456",
    "1234567890",
    "user",
    "unknown",
    "1234567",
    "tmp",
    "test",
    "111111",
    "qwerty123",
    "abc123",
    "1q2w3e4r5t",
    "qwertyuiop",
    "654321",
    "iloveyou",
    "dragon",
    "monkey",
    "qwerty",
    // Common Solobase-flavored additions that always show up in password lists
    // for new self-hosted apps. Cheap to include here.
    "password1",
    "admin123",
    "solobase",
];

fn is_common_password(pw: &str) -> bool {
    let lower = pw.to_ascii_lowercase();
    COMMON_PASSWORDS.iter().any(|p| *p == lower)
}

/// Send verification email via the suppers-ai/email block.
async fn send_verification_email(ctx: &dyn Context, email: &str, token: &str) {
    let req = serde_json::json!({
        "template": "verification",
        "to": email,
        "token": token,
    });
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
    if let Err(e) = out.collect_buffered().await {
        tracing::warn!("Failed to send verification email to {}: {:?}", email, e);
    }
}
