//! Email block — sends emails via Mailgun HTTP API.
//!
//! Routes:
//! - `email.send` — Send a raw email (to, subject, html, text)
//! - `email.send_template` — Send a templated email (template name + variables)
//!
//! Uses the `wafer-run/network` block to make HTTP requests to Mailgun,
//! and `wafer-run/config` for MAILGUN_API_KEY, MAILGUN_DOMAIN, MAILGUN_FROM.

use serde::{Deserialize, Serialize};

use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

pub struct EmailBlock;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for EmailBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "suppers-ai/email".to_string(),
            version: "1.0.0".to_string(),
            interface: "service@v1".to_string(),
            summary: "Email sending via Mailgun".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
            runtime: wafer_run::types::BlockRuntime::Native,
            requires: Vec::new(),
            collections: Vec::new(),
        }
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        match msg.kind.as_str() {
            "email.send" => handle_send(ctx, msg).await,
            "email.send_template" => handle_send_template(ctx, msg).await,
            _ => err_not_found(msg, &format!("unknown email op: {}", msg.kind)),
        }
    }

    async fn lifecycle(
        &self,
        _ctx: &dyn Context,
        _event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// email.send
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct SendReq {
    to: String,
    subject: String,
    html: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Serialize)]
struct SendResp {
    sent: bool,
}

async fn handle_send(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let req: SendReq = match msg.decode() {
        Ok(r) => r,
        Err(e) => return err_invalid(msg, &format!("invalid email.send: {e}")),
    };

    let sent = send_email(ctx, &req.to, &req.subject, &req.html, req.text.as_deref()).await;
    json_respond(msg, &SendResp { sent })
}

// ---------------------------------------------------------------------------
// email.send_template
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct TemplateReq {
    template: String,
    to: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    days_remaining: Option<u32>,
}

async fn handle_send_template(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let req: TemplateReq = match msg.decode() {
        Ok(r) => r,
        Err(e) => return err_invalid(msg, &format!("invalid email.send_template: {e}")),
    };

    let (subject, html, text) = match req.template.as_str() {
        "verification" => {
            let token = req.token.as_deref().unwrap_or("");
            let url = format!("https://cloud.solobase.dev/auth/verify?token={}", url_encode(token));
            (
                "Verify your Solobase email".to_string(),
                format!(r#"<div style="font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;max-width:500px;margin:0 auto;padding:2rem">
<h2 style="color:#1e293b">Verify your email</h2>
<p style="color:#64748b;line-height:1.6">Click the button below to verify your email address. This link expires in 24 hours.</p>
<a href="{url}" style="display:inline-block;background:#0ea5e9;color:white;padding:0.75rem 1.5rem;border-radius:8px;text-decoration:none;font-weight:600;margin:1rem 0">Verify Email</a>
<p style="color:#94a3b8;font-size:0.813rem">If you didn't create an account, you can ignore this email.</p>
</div>"#),
                format!("Verify your Solobase email: {url}"),
            )
        }
        "password_reset" => {
            let token = req.token.as_deref().unwrap_or("");
            let url = format!("https://cloud.solobase.dev/auth/reset-password?token={}", url_encode(token));
            (
                "Reset your Solobase password".to_string(),
                format!(r#"<div style="font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;max-width:500px;margin:0 auto;padding:2rem">
<h2 style="color:#1e293b">Reset your password</h2>
<p style="color:#64748b;line-height:1.6">Click the button below to reset your password. This link expires in 1 hour.</p>
<a href="{url}" style="display:inline-block;background:#0ea5e9;color:white;padding:0.75rem 1.5rem;border-radius:8px;text-decoration:none;font-weight:600;margin:1rem 0">Reset Password</a>
<p style="color:#94a3b8;font-size:0.813rem">If you didn't request a password reset, you can ignore this email.</p>
</div>"#),
                format!("Reset your Solobase password: {url}"),
            )
        }
        "payment_failed" => {
            let days = req.days_remaining.unwrap_or(7);
            (
                "Solobase: Payment failed — action required".to_string(),
                format!(r#"<div style="font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;max-width:500px;margin:0 auto;padding:2rem">
<h2 style="color:#dc2626">Payment failed</h2>
<p style="color:#64748b;line-height:1.6">We were unable to process your subscription payment. Your service will remain active for <strong>{days} more days</strong>. After that, your projects will be suspended.</p>
<a href="https://cloud.solobase.dev/blocks/dashboard/#settings" style="display:inline-block;background:#dc2626;color:white;padding:0.75rem 1.5rem;border-radius:8px;text-decoration:none;font-weight:600;margin:1rem 0">Update Payment Method</a>
<p style="color:#94a3b8;font-size:0.813rem">If you've already updated your payment method, you can ignore this email.</p>
</div>"#),
                format!("Your Solobase payment failed. Update your payment method within {} days.", days),
            )
        }
        "welcome" => {
            let name = req.name.as_deref().unwrap_or("");
            let greeting = if name.is_empty() { "Welcome!".to_string() } else { format!("Welcome, {}!", name) };
            (
                "Welcome to Solobase!".to_string(),
                format!(r#"<div style="font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;max-width:500px;margin:0 auto;padding:2rem">
<h2 style="color:#1e293b">{greeting}</h2>
<p style="color:#64748b;line-height:1.6">Your Solobase account is ready. Here's how to get started:</p>
<ol style="color:#64748b;line-height:1.8">
<li>Choose a plan on the <a href="https://solobase.dev/pricing/" style="color:#0ea5e9">pricing page</a></li>
<li>Create your first project from the <a href="https://cloud.solobase.dev/blocks/dashboard/" style="color:#0ea5e9">dashboard</a></li>
<li>Read the <a href="https://solobase.dev/docs/" style="color:#0ea5e9">documentation</a></li>
</ol>
</div>"#),
                format!("Welcome to Solobase! Get started: https://cloud.solobase.dev/blocks/dashboard/"),
            )
        }
        other => {
            return err_invalid(msg, &format!("unknown email template: {other}"));
        }
    };

    let sent = send_email(ctx, &req.to, &subject, &html, Some(&text)).await;
    json_respond(msg, &SendResp { sent })
}

// ---------------------------------------------------------------------------
// Mailgun HTTP API
// ---------------------------------------------------------------------------

async fn send_email(
    ctx: &dyn Context,
    to: &str,
    subject: &str,
    html: &str,
    text: Option<&str>,
) -> bool {
    let api_key = ctx.config_get("MAILGUN_API_KEY").unwrap_or("").to_string();
    let domain = ctx.config_get("MAILGUN_DOMAIN").unwrap_or("").to_string();
    let from = match ctx.config_get("MAILGUN_FROM") {
        Some(f) if !f.is_empty() => f.to_string(),
        _ => format!("Solobase <noreply@{domain}>"),
    };

    if api_key.is_empty() || domain.is_empty() {
        // Email not configured — log but don't fail
        return false;
    }

    // Build form-encoded body
    let mut parts = vec![
        format!("from={}", url_encode(&from)),
        format!("to={}", url_encode(to)),
        format!("subject={}", url_encode(subject)),
        format!("html={}", url_encode(html)),
    ];
    if let Some(reply_to) = ctx.config_get("MAILGUN_REPLY_TO") {
        if !reply_to.is_empty() {
            parts.push(format!("h:Reply-To={}", url_encode(reply_to)));
        }
    }
    if let Some(text) = text {
        parts.push(format!("text={}", url_encode(text)));
    }
    let body = parts.join("&");

    // Base64 encode "api:{api_key}"
    let credentials = base64_encode(&format!("api:{}", api_key));

    // Call network block
    let url = format!("https://api.mailgun.net/v3/{}/messages", domain);
    let network_req = serde_json::json!({
        "method": "POST",
        "url": url,
        "headers": {
            "Authorization": format!("Basic {}", credentials),
            "Content-Type": "application/x-www-form-urlencoded",
        },
        "body": body.as_bytes().to_vec(),
    });

    let mut network_msg = Message {
        kind: "network.do".to_string(),
        data: serde_json::to_vec(&network_req).unwrap_or_default(),
        meta: Vec::new(),
    };

    let result = ctx.call_block("wafer-run/network", &mut network_msg).await;

    match result.action {
        Action::Respond => {
            if let Some(ref resp) = result.response {
                // Check if status was 200
                if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&resp.data) {
                    let status = v.get("status_code").and_then(|s| s.as_u64()).unwrap_or(0);
                    return status >= 200 && status < 300;
                }
            }
            false
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn url_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}

fn base64_encode(s: &str) -> String {
    use std::io::Write;
    let mut buf = Vec::new();
    {
        let mut encoder = Base64Encoder::new(&mut buf);
        encoder.write_all(s.as_bytes()).ok();
        encoder.finish();
    }
    String::from_utf8(buf).unwrap_or_default()
}

/// Minimal base64 encoder (no external dependency needed).
struct Base64Encoder<'a> {
    out: &'a mut Vec<u8>,
    buf: [u8; 3],
    len: usize,
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

impl<'a> Base64Encoder<'a> {
    fn new(out: &'a mut Vec<u8>) -> Self {
        Self { out, buf: [0; 3], len: 0 }
    }

    fn flush_block(&mut self) {
        let b = &self.buf;
        self.out.push(B64[(b[0] >> 2) as usize]);
        self.out.push(B64[((b[0] & 0x03) << 4 | b[1] >> 4) as usize]);
        if self.len > 1 {
            self.out.push(B64[((b[1] & 0x0f) << 2 | b[2] >> 6) as usize]);
        } else {
            self.out.push(b'=');
        }
        if self.len > 2 {
            self.out.push(B64[(b[2] & 0x3f) as usize]);
        } else {
            self.out.push(b'=');
        }
    }

    fn finish(&mut self) {
        if self.len > 0 {
            for i in self.len..3 { self.buf[i] = 0; }
            self.flush_block();
        }
    }
}

impl<'a> std::io::Write for Base64Encoder<'a> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        for &byte in data {
            self.buf[self.len] = byte;
            self.len += 1;
            if self.len == 3 {
                self.flush_block();
                self.len = 0;
                self.buf = [0; 3];
            }
        }
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn err_invalid(_msg: &Message, message: &str) -> Result_ {
    Result_::error(WaferError::new("invalid_argument", message))
}
