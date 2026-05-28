//! GET/POST /b/auth/api/verify and POST /b/auth/api/resend-verification —
//! relocated from auth/login.rs in Task 5.

use maud::html;
use wafer_core::clients::{crypto, database as db};
use wafer_run::{context::Context, types::Message, InputStream, OutputStream};

use crate::{
    blocks::{
        auth::{brand_panel, USERS_TABLE},
        helpers::{
            err_bad_request, err_internal, hex_encode, json_map, ok_json, sha256_hex, RecordExt,
        },
    },
    ui,
    ui::templates::auth_split,
};

pub async fn handle(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    let logo_url = ctx
        .config_get("SOLOBASE_SHARED__AUTH_LOGO_URL")
        .unwrap_or("")
        .to_string();

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

    // Find user by verification token. The DB column stores
    // `sha256_hex(raw)`; hash the supplied token the same way before
    // comparing.
    let user = match db::get_by_field(
        ctx,
        USERS_TABLE,
        "verification_token",
        serde_json::Value::String(sha256_hex(token.as_bytes())),
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

    if let Err(e) = db::update(ctx, USERS_TABLE, &user.id, data).await {
        return err_internal("Failed to verify email", e);
    }

    html_respond(
        "Email Verified",
        "Your email has been verified successfully. You can now sign in.",
        true,
        &logo_url,
    )
}

pub async fn handle_resend(ctx: &dyn Context, input: InputStream) -> OutputStream {
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
        USERS_TABLE,
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

    // Generate new token. The raw token goes in the email link; only its
    // SHA-256 hex digest is persisted so a row-read leak doesn't grant
    // verification.
    let new_token = match crypto::random_bytes(ctx, 32).await {
        Ok(bytes) => hex_encode(&bytes),
        Err(e) => return err_internal("Token generation failed", e),
    };
    let new_token_hash = sha256_hex(new_token.as_bytes());

    let now = crate::blocks::helpers::now_rfc3339();
    let mut data = json_map(serde_json::json!({
        "verification_token": new_token_hash,
        "last_verification_sent": now
    }));
    crate::blocks::helpers::stamp_updated(&mut data);

    if let Err(e) = db::update(ctx, USERS_TABLE, &user.id, data).await {
        return err_internal("Failed to update token", e);
    }

    send_verification_email(ctx, &email_lower, &new_token).await;

    ok_json(&serde_json::json!({"message": safe_msg}))
}

/// Return an HTML page response (for verify endpoints opened in browser).
fn html_respond(title: &str, message: &str, success: bool, logo_url: &str) -> OutputStream {
    let color = if success { "#10b981" } else { "#ef4444" };
    let config = ui::SiteConfig {
        app_name: "Solobase".into(),
        logo_url: logo_url.to_string(),
        logo_icon_url: String::new(),
        favicon_url: crate::ui::assets::favicon_url().to_string(),
        embedded_scripts: Vec::new(),
    };
    let markup = ui::layout::page(
        title,
        &config,
        auth_split(
            brand_panel(&config),
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
        ),
    );
    ui::html_response(markup)
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
