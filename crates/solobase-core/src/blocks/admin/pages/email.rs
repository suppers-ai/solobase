use maud::{html, PreEscaped};
use wafer_core::clients::config;
use wafer_run::{context::Context, types::*, InputStream, OutputStream};

use super::admin_page;
use crate::{
    blocks::helpers::{err_bad_request, ok_json},
    ui::{components, icons, SiteConfig, UserInfo},
};

const EMAIL_SETTINGS_KEYS: &[(&str, &str, &str, &str, bool)] = &[
    (
        "SUPPERS_AI__EMAIL__MAILGUN_API_KEY",
        "Mailgun API Key",
        "API key from your Mailgun account.",
        "",
        true,
    ),
    (
        "SUPPERS_AI__EMAIL__MAILGUN_DOMAIN",
        "Mailgun Domain",
        "Sending domain configured in Mailgun (e.g. mg.example.com).",
        "",
        false,
    ),
    (
        "SUPPERS_AI__EMAIL__MAILGUN_FROM",
        "From Address",
        "Sender address for emails. Leave empty for default (noreply@domain).",
        "",
        false,
    ),
    (
        "SUPPERS_AI__EMAIL__MAILGUN_REPLY_TO",
        "Reply-To Address",
        "Reply-to address for emails. Leave empty to omit.",
        "",
        false,
    ),
];

pub async fn email_settings_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let site_config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let mut values = Vec::new();
    for &(key, label, help, default, sensitive) in EMAIL_SETTINGS_KEYS {
        let value = config::get_default(ctx, key, default).await;
        values.push((key, label, help, default, value, sensitive));
    }

    let content = html! {
        (components::page_header("Email Settings", Some("Configure email delivery via Mailgun"), None))

        form #settings-form onsubmit="return submitEmailSettings(event)" {
            h3 style="font-size:1rem;font-weight:600;margin:0 0 1rem;padding-bottom:0.5rem;border-bottom:1px solid var(--border-color)" {
                (icons::globe()) " Mailgun Configuration"
            }

            @for (key, label, help, default, ref value, sensitive) in &values {
                div .form-group style="margin-bottom:1.25rem" {
                    label .form-label for=(key) { (label) }
                    @if *sensitive {
                        div style="display:flex;align-items:center;gap:0.5rem" {
                            input .form-input #(key) name=(key) type="password" value=(value)
                                placeholder=(if value.is_empty() { "Not configured" } else { "******** (set)" })
                                style="flex:1";
                            button type="button" .btn .btn-ghost .btn-sm
                                onclick={"var i=document.getElementById('" (key) "');i.type=i.type==='password'?'text':'password'"}
                            { (icons::eye()) }
                        }
                    } @else {
                        input .form-input #(key) name=(key) type="text" value=(value) placeholder=(default);
                    }
                    p .text-muted style="font-size:0.8rem;margin-top:0.25rem" { (help) }
                }
            }

            button .btn .btn-primary type="submit" style="margin-top:1rem" { "Save Settings" }
        }

        script { (PreEscaped(r#"
function submitEmailSettings(e) {
    e.preventDefault();
    var form = document.getElementById('settings-form');
    var data = {};
    form.querySelectorAll('input[name]').forEach(function(el) { data[el.name] = el.value; });
    var btn = form.querySelector('button[type="submit"]');
    btn.disabled = true; btn.textContent = 'Saving...';
    fetch('/b/admin/email', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(data) })
    .then(function(r) { return r.json(); })
    .then(function(d) { document.body.dispatchEvent(new CustomEvent('showToast', { detail: { message: d.message || 'Saved', type: d.error ? 'error' : 'success' } })); })
    .catch(function(err) { document.body.dispatchEvent(new CustomEvent('showToast', { detail: { message: 'Error: ' + err.message, type: 'error' } })); })
    .finally(function() { btn.disabled = false; btn.textContent = 'Save Settings'; });
    return false;
}
"#)) }
    };

    admin_page(
        "Email",
        &site_config,
        "/b/admin/email",
        user.as_ref(),
        content,
        msg,
    )
}

pub async fn handle_save_email_settings(
    ctx: &dyn Context,
    _msg: &Message,
    input: InputStream,
) -> OutputStream {
    let bytes = input.collect_to_bytes().await;
    let body: std::collections::HashMap<String, String> = match serde_json::from_slice(&bytes) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid request: {e}")),
    };
    for &(key, _, _, _, _) in EMAIL_SETTINGS_KEYS {
        if let Some(value) = body.get(key) {
            let _ = config::set(ctx, key, value).await;
        }
    }
    ok_json(&serde_json::json!({"message": "Email settings saved"}))
}
