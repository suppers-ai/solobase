//! GET / POST /b/auth/admin/settings — relocated from auth/pages/mod.rs in Task 5.

use std::collections::HashMap;

use maud::{html, Markup, PreEscaped};
use wafer_core::clients::config;
use wafer_run::{context::Context, types::Message, InputStream, OutputStream};

use crate::{
    blocks::helpers::{err_bad_request, ok_json},
    ui::{
        self, components, icons, nav_groups,
        shell::{Crumb, Topbar},
        SiteConfig, UserInfo,
    },
};

/// (key, label, help_text, default, is_sensitive, input_type)
const SETTINGS_KEYS: &[(&str, &str, &str, &str, bool, &str)] = &[
    (
        "SOLOBASE_SHARED__ALLOW_SIGNUP",
        "Allow Signup",
        "Allow new user registration.",
        "true",
        false,
        "toggle",
    ),
    (
        "SUPPERS_AI__AUTH__REQUIRE_VERIFICATION",
        "Require Email Verification",
        "Require users to verify their email before they can log in.",
        "false",
        false,
        "toggle",
    ),
    (
        "SUPPERS_AI__AUTH__ALLOWED_EMAIL_DOMAINS",
        "Allowed Email Domains",
        "Restrict signup to specific email domains (comma-separated, e.g. \"company.com,org.com\"). Leave empty to allow all.",
        "",
        false,
        "text",
    ),
    (
        "SOLOBASE_SHARED__POST_LOGIN_REDIRECT",
        "Post-Login Redirect",
        "Where to send users after successful login.",
        "/b/admin/",
        false,
        "text",
    ),
    (
        "SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_EMAIL",
        "Bootstrap Admin Email",
        "Email of the admin user created on first startup (bootstrap only).",
        "",
        false,
        "text",
    ),
    (
        "SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_PASSWORD",
        "Bootstrap Admin Password",
        "Password for the bootstrap admin account (bootstrap only).",
        "",
        true,
        "password",
    ),
    (
        "SOLOBASE_SHARED__ENABLE_OAUTH",
        "Enable OAuth",
        "Enable OAuth login providers (Google, GitHub).",
        "false",
        false,
        "toggle",
    ),
    (
        "SUPPERS_AI__AUTH_UI__OAUTH_GOOGLE_CLIENT_ID",
        "Google Client ID",
        "OAuth client ID from Google Cloud Console.",
        "",
        false,
        "text",
    ),
    (
        "SUPPERS_AI__AUTH_UI__OAUTH_GOOGLE_CLIENT_SECRET",
        "Google Client Secret",
        "OAuth client secret from Google Cloud Console.",
        "",
        true,
        "password",
    ),
    (
        "SUPPERS_AI__AUTH_UI__OAUTH_GITHUB_CLIENT_ID",
        "GitHub Client ID",
        "OAuth client ID from GitHub developer settings.",
        "",
        false,
        "text",
    ),
    (
        "SUPPERS_AI__AUTH_UI__OAUTH_GITHUB_CLIENT_SECRET",
        "GitHub Client Secret",
        "OAuth client secret from GitHub developer settings.",
        "",
        true,
        "password",
    ),
];

pub async fn handle_get(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let site_config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let mut values = Vec::new();
    for &(key, label, help, default, sensitive, input_type) in SETTINGS_KEYS {
        let value = config::get_default(ctx, key, default).await;
        values.push((key, label, help, default, value, sensitive, input_type));
    }

    let content = html! {
        (components::page_header("Authentication Settings", Some("Configure registration, OAuth providers, and security"), None))

        form #settings-form onsubmit="return submitSettings(event)" {
            // Registration section
            h3 style="font-size:1rem;font-weight:600;margin:0 0 1rem;padding-bottom:0.5rem;border-bottom:1px solid var(--border-color)" {
                (icons::users()) " Registration"
            }
            @for (key, label, help, default, ref value, _sensitive, input_type) in values.iter().take(4) {
                (render_setting_field(key, label, help, default, value, input_type))
            }

            // Admin section
            h3 style="font-size:1rem;font-weight:600;margin:1.5rem 0 1rem;padding-bottom:0.5rem;border-bottom:1px solid var(--border-color)" {
                (icons::shield()) " Admin"
            }
            @for (key, label, help, default, ref value, _sensitive, input_type) in values.iter().skip(4).take(1) {
                (render_setting_field(key, label, help, default, value, input_type))
            }

            // OAuth section
            h3 style="font-size:1rem;font-weight:600;margin:1.5rem 0 1rem;padding-bottom:0.5rem;border-bottom:1px solid var(--border-color)" {
                (icons::globe()) " OAuth Providers"
            }
            @for (key, label, help, _default, ref value, sensitive, input_type) in values.iter().skip(5) {
                @if *sensitive {
                    (render_sensitive_field(key, label, help, value))
                } @else {
                    (render_setting_field(key, label, help, _default, value, input_type))
                }
            }

            button .btn .btn-primary type="submit" style="margin-top:1rem" { "Save Settings" }
        }

        script { (PreEscaped(SETTINGS_JS)) }
    };

    let groups = nav_groups::admin();
    let topbar = Topbar {
        crumbs: vec![Crumb {
            label: "Auth Settings",
            href: None,
        }],
        primary_action: None,
        subtitle: None,
        show_palette: true,
    };
    ui::shelled_response(
        msg,
        "Auth Settings",
        &site_config,
        &groups,
        user.as_ref(),
        "/b/auth/admin/settings",
        topbar,
        content,
    )
}

const SETTINGS_JS: &str = r#"
function submitSettings(e) {
    e.preventDefault();
    var form = document.getElementById('settings-form');
    var data = {};
    form.querySelectorAll('input[name], select[name], textarea[name]').forEach(function(el) {
        if (el.type === 'checkbox') {
            data[el.name] = el.checked ? 'true' : 'false';
        } else {
            data[el.name] = el.value;
        }
    });
    var btn = form.querySelector('button[type="submit"]');
    btn.disabled = true; btn.textContent = 'Saving...';
    fetch('/b/auth/admin/settings', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data)
    })
    .then(function(r) { return r.json(); })
    .then(function(d) {
        document.body.dispatchEvent(new CustomEvent('showToast', {
            detail: { message: d.message || 'Saved', type: d.error ? 'error' : 'success' }
        }));
    })
    .catch(function(err) {
        document.body.dispatchEvent(new CustomEvent('showToast', {
            detail: { message: 'Error: ' + err.message, type: 'error' }
        }));
    })
    .finally(function() { btn.disabled = false; btn.textContent = 'Save Settings'; });
    return false;
}
"#;

pub async fn handle_post(ctx: &dyn Context, input: InputStream) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let body: HashMap<String, String> = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid request: {e}")),
    };

    for &(key, _, _, _, _, _) in SETTINGS_KEYS {
        if let Some(value) = body.get(key) {
            let _ = config::set(ctx, key, value).await;
        }
    }

    ok_json(&serde_json::json!({"message": "Settings saved"}))
}

fn render_setting_field(
    key: &str,
    label: &str,
    help: &str,
    default: &str,
    value: &str,
    input_type: &str,
) -> Markup {
    html! {
        div .form-group style="margin-bottom:1.25rem" {
            @if input_type == "toggle" {
                label style="display:flex;align-items:center;gap:0.75rem;cursor:pointer" {
                    input type="checkbox" name=(key)
                        checked[value == "true"]
                        style="width:1.25rem;height:1.25rem;accent-color:var(--primary)";
                    span .form-label style="margin:0" { (label) }
                }
            } @else {
                label .form-label for=(key) { (label) }
                input .form-input #(key) name=(key)
                    type="text"
                    value=(value)
                    placeholder=(default);
            }
            p .text-muted style="font-size:0.8rem;margin-top:0.25rem" { (help) }
        }
    }
}

fn render_sensitive_field(key: &str, label: &str, help: &str, value: &str) -> Markup {
    let has_value = !value.is_empty();
    html! {
        div .form-group style="margin-bottom:1.25rem" {
            label .form-label for=(key) { (label) }
            div style="display:flex;align-items:center;gap:0.5rem" {
                input .form-input #(key) name=(key)
                    type="password"
                    value=(value)
                    placeholder=(if has_value { "******** (set)" } else { "Not configured" })
                    style="flex:1";
                button type="button" .btn .btn-ghost .btn-sm
                    onclick={"var i=document.getElementById('" (key) "');i.type=i.type==='password'?'text':'password'"}
                {
                    (icons::eye())
                }
            }
            p .text-muted style="font-size:0.8rem;margin-top:0.25rem" { (help) }
        }
    }
}
