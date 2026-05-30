use maud::{html, Markup, PreEscaped};
use wafer_core::clients::config;
use wafer_run::{context::Context, types::*, InputStream, OutputStream};

use crate::{
    blocks::{
        email::DEFAULT_MAILGUN_BASE_URL,
        helpers::{err_bad_request, ok_json},
    },
    ui::icons,
};

/// One row in the Mailgun settings form. Was a positional 5-tuple
/// `(key, label, help, default, sensitive)` — names make the call site
/// readable and let future fields land without re-counting tuple positions.
struct EmailSettingField {
    key: &'static str,
    label: &'static str,
    help: &'static str,
    default: &'static str,
    sensitive: bool,
}

const EMAIL_SETTINGS_KEYS: &[EmailSettingField] = &[
    EmailSettingField {
        key: "SUPPERS_AI__EMAIL__MAILGUN_API_KEY",
        label: "Mailgun API Key",
        help: "API key from your Mailgun account.",
        default: "",
        sensitive: true,
    },
    EmailSettingField {
        key: "SUPPERS_AI__EMAIL__MAILGUN_DOMAIN",
        label: "Mailgun Domain",
        help: "Sending domain configured in Mailgun (e.g. mg.example.com).",
        default: "",
        sensitive: false,
    },
    EmailSettingField {
        key: "SUPPERS_AI__EMAIL__MAILGUN_FROM",
        label: "From Address",
        help: "Sender address for emails. Leave empty for default (noreply@domain).",
        default: "",
        sensitive: false,
    },
    EmailSettingField {
        key: "SUPPERS_AI__EMAIL__MAILGUN_REPLY_TO",
        label: "Reply-To Address",
        help: "Reply-to address for emails. Leave empty to omit.",
        default: "",
        sensitive: false,
    },
    EmailSettingField {
        key: "SUPPERS_AI__EMAIL__MAILGUN_BASE_URL",
        label: "Mailgun Base URL",
        help: "API base URL. Leave empty for US (https://api.mailgun.net); use https://api.eu.mailgun.net for EU accounts.",
        default: DEFAULT_MAILGUN_BASE_URL,
        sensitive: false,
    },
];

/// Render JUST the email settings form body. The parent `settings_page`
/// handler wraps this in `form_page` + the shell.
pub async fn settings_body(ctx: &dyn Context, _msg: &Message) -> Markup {
    let mut values: Vec<(&EmailSettingField, String)> = Vec::new();
    for field in EMAIL_SETTINGS_KEYS {
        let value = config::get_default(ctx, field.key, field.default).await;
        values.push((field, value));
    }

    html! {
        h3 style="font-size:1rem;font-weight:600;margin:0 0 1rem;padding-bottom:0.5rem;border-bottom:1px solid var(--border-color)" {
            (icons::globe()) " Mailgun Configuration"
        }

        @for (field, value) in &values {
            div .form-group style="margin-bottom:1.25rem" {
                label .form-label for=(field.key) { (field.label) }
                @if field.sensitive {
                    div style="display:flex;align-items:center;gap:0.5rem" {
                        input .form-input #(field.key) name=(field.key) type="password" value=(value)
                            placeholder=(if value.is_empty() { "Not configured" } else { "******** (set)" })
                            style="flex:1";
                        button type="button" .btn .btn-ghost .btn-sm
                            onclick={"var i=document.getElementById('" (field.key) "');i.type=i.type==='password'?'text':'password'"}
                        { (icons::eye()) }
                    }
                } @else {
                    input .form-input #(field.key) name=(field.key) type="text" value=(value) placeholder=(field.default);
                }
                p .text-muted style="font-size:0.8rem;margin-top:0.25rem" { (field.help) }
            }
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
    .finally(function() { btn.disabled = false; btn.textContent = 'Save'; });
    return false;
}
"#)) }
    }
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
    for field in EMAIL_SETTINGS_KEYS {
        if let Some(value) = body.get(field.key) {
            let _ = config::set(ctx, field.key, value).await;
        }
    }
    ok_json(&serde_json::json!({"message": "Email settings saved"}))
}
