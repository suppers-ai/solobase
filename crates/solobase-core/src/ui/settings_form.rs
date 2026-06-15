//! Shared, ConfigVar-driven admin settings form.
//!
//! Every block's admin settings page used to carry its own copy of: a
//! stringly-typed tuple table re-declaring `(key, label, help, default,
//! input_type)` that the block *already* declares as [`ConfigVar`] in its
//! `BlockInfo::config_keys` (or that `config_vars.rs` declares centrally for
//! `SOLOBASE_SHARED__*`), a maud form loop with a special-cased color picker
//! and sensitive-field eye toggle, a copy-pasted inline-JS submit function,
//! and a POST handler that walked the same tuple table to `config::set` each
//! key. Five blocks, five drifting copies (verified live drifts: legalpages
//! `BG_COLOR` default, userportal `FAVICON_URL`/logo-URL input types).
//!
//! This module is the single renderer + the single save handler, driven
//! directly by [`ConfigVar`] metadata — the declared single source of truth.
//! The widget is derived from [`InputType`]: `Password` → masked field with an
//! eye toggle, `Color` → text input paired with a color picker, `Toggle` →
//! checkbox, `Url`/`Text` → plain text input. Each block's settings page becomes
//! "pick the ConfigVars to show, group them into sections, render".

use std::collections::HashMap;

use maud::{html, Markup, PreEscaped};
use wafer_core::clients::config;
use wafer_run::{context::Context, InputStream, OutputStream};
pub use wafer_run::{ConfigVar, InputType};

use crate::http::{err_bad_request, ok_json};

/// One titled group of settings within a form (e.g. "Stripe", "OAuth Providers").
pub struct SettingsSection<'a> {
    /// Section heading text.
    pub title: &'a str,
    /// Section heading icon (a maud fragment, e.g. `icons::settings()`).
    pub icon: Markup,
    /// The config variables rendered in this section, in order.
    pub vars: &'a [ConfigVar],
}

impl<'a> SettingsSection<'a> {
    /// Construct a section from a title, icon, and its variables.
    pub fn new(title: &'a str, icon: Markup, vars: &'a [ConfigVar]) -> Self {
        Self { title, icon, vars }
    }
}

/// Load the current value for every var in `sections` via the config client.
async fn load_values(
    ctx: &dyn Context,
    sections: &[SettingsSection<'_>],
) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for section in sections {
        for var in section.vars {
            // `entry`-guard so a key declared in two sections is only fetched once.
            if !values.contains_key(&var.key) {
                let value = config::get_default(ctx, &var.key, &var.default).await;
                values.insert(var.key.clone(), value);
            }
        }
    }
    values
}

/// Render one field, deriving the widget from the var's [`InputType`].
fn render_field(var: &ConfigVar, value: &str) -> Markup {
    let label = if var.name.is_empty() {
        var.key.as_str()
    } else {
        var.name.as_str()
    };
    match var.input_type {
        InputType::Toggle => html! {
            div .form-group style="margin-bottom:1.25rem" {
                label style="display:flex;align-items:center;gap:0.75rem;cursor:pointer" {
                    input type="checkbox" name=(var.key)
                        checked[value == "true"]
                        style="width:1.25rem;height:1.25rem;accent-color:var(--primary)";
                    span .form-label style="margin:0" { (label) }
                }
                @if !var.description.is_empty() {
                    p .text-muted style="font-size:0.8rem;margin-top:0.25rem" { (var.description) }
                }
            }
        },
        InputType::Password => {
            let has_value = !value.is_empty();
            html! {
                div .form-group style="margin-bottom:1.25rem" {
                    label .form-label for=(var.key) { (label) }
                    div style="display:flex;align-items:center;gap:0.5rem" {
                        input .form-input #(var.key) name=(var.key) type="password" value=(value)
                            placeholder=(if has_value { "******** (set)" } else { "Not configured" })
                            style="flex:1";
                        button type="button" .btn .btn-ghost .btn-sm
                            onclick={"var i=document.getElementById('" (var.key) "');i.type=i.type==='password'?'text':'password'"}
                        { (super::icons::eye()) }
                    }
                    @if !var.description.is_empty() {
                        p .text-muted style="font-size:0.8rem;margin-top:0.25rem" { (var.description) }
                    }
                }
            }
        }
        InputType::Color => html! {
            div .form-group style="margin-bottom:1.25rem" {
                label .form-label for=(var.key) { (label) }
                div style="display:flex;align-items:center;gap:0.75rem" {
                    input .form-input #(var.key) name=(var.key) type="text" value=(value)
                        placeholder=(var.default) style="flex:1";
                    input type="color" value=(value)
                        style="width:40px;height:36px;border:1px solid #e2e8f0;border-radius:6px;cursor:pointer;padding:2px"
                        onchange={"document.getElementById('" (var.key) "').value=this.value"};
                }
                @if !var.description.is_empty() {
                    p .text-muted style="font-size:0.8rem;margin-top:0.25rem" { (var.description) }
                }
            }
        },
        InputType::Textarea => html! {
            div .form-group style="margin-bottom:1.25rem" {
                label .form-label for=(var.key) { (label) }
                textarea .form-input #(var.key) name=(var.key) rows="4"
                    placeholder=(var.default) { (value) }
                @if !var.description.is_empty() {
                    p .text-muted style="font-size:0.8rem;margin-top:0.25rem" { (var.description) }
                }
            }
        },
        InputType::Url | InputType::Text => html! {
            div .form-group style="margin-bottom:1.25rem" {
                label .form-label for=(var.key) { (label) }
                input .form-input #(var.key) name=(var.key) type="text" value=(value)
                    placeholder=(var.default);
                @if !var.description.is_empty() {
                    p .text-muted style="font-size:0.8rem;margin-top:0.25rem" { (var.description) }
                }
            }
        },
    }
}

/// The single inline submit snippet shared by every settings form. Posts the
/// form as a JSON object to `post_url` and shows a toast with the result.
/// `post_url` is interpolated via `serde_json` so it can't break out of the
/// JS string literal.
fn submit_js(post_url: &str) -> String {
    let url = serde_json::to_string(post_url).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        r#"
function submitSettings(e) {{
    e.preventDefault();
    var form = document.getElementById('settings-form');
    var data = {{}};
    form.querySelectorAll('input[name], select[name], textarea[name]').forEach(function(el) {{
        if (el.type === 'checkbox') {{ data[el.name] = el.checked ? 'true' : 'false'; }}
        else {{ data[el.name] = el.value; }}
    }});
    var btn = form.querySelector('button[type="submit"]');
    btn.disabled = true; btn.textContent = 'Saving...';
    fetch({url}, {{ method: 'POST', headers: {{ 'Content-Type': 'application/json' }}, body: JSON.stringify(data) }})
    .then(function(r) {{ return r.json(); }})
    .then(function(d) {{ document.body.dispatchEvent(new CustomEvent('showToast', {{ detail: {{ message: d.message || 'Saved', type: d.error ? 'error' : 'success' }} }})); }})
    .catch(function(err) {{ document.body.dispatchEvent(new CustomEvent('showToast', {{ detail: {{ message: 'Error: ' + err.message, type: 'error' }} }})); }})
    .finally(function() {{ btn.disabled = false; btn.textContent = 'Save Settings'; }});
    return false;
}}
"#
    )
}

/// Render the full ConfigVar-driven settings form: a `#settings-form` posting
/// JSON to `post_url`, with one titled section per [`SettingsSection`], a
/// "Save Settings" button, and the shared submit snippet. Current values are
/// loaded from the config client internally.
///
/// `extra` is appended after the last section and before the submit button —
/// used by blocks that want an extra panel inside the form (e.g. legalpages'
/// live-preview links).
pub async fn settings_form(
    ctx: &dyn Context,
    post_url: &str,
    sections: &[SettingsSection<'_>],
    extra: Markup,
) -> Markup {
    let values = load_values(ctx, sections).await;
    let empty = String::new();
    html! {
        form #settings-form onsubmit="return submitSettings(event)" {
            @for (i, section) in sections.iter().enumerate() {
                h3 style=(format!(
                    "font-size:1rem;font-weight:600;margin:{} 0 1rem;padding-bottom:0.5rem;border-bottom:1px solid var(--border-color)",
                    if i == 0 { "0" } else { "1.5rem" }
                )) {
                    (section.icon) " " (section.title)
                }
                @for var in section.vars {
                    (render_field(var, values.get(&var.key).unwrap_or(&empty)))
                }
            }
            (extra)
            button .btn .btn-primary type="submit" style="margin-top:1rem" { "Save Settings" }
        }
        script { (PreEscaped(submit_js(post_url))) }
    }
}

/// Generic settings save handler: parse the JSON body, and for every key in
/// the `allowed` ConfigVar allowlist that the body carries, `config::set` it.
/// Keys outside the allowlist are ignored (a block can only write the vars it
/// declared). Parse failure returns a real `400` (htmx clients branch on the
/// status, not a 200-with-`error`-key body — the residual finding folded in
/// from S1-I).
pub async fn save_settings(
    ctx: &dyn Context,
    input: InputStream,
    allowed: &[ConfigVar],
    block_label: &str,
) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let body: HashMap<String, String> = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid request: {e}")),
    };
    for var in allowed {
        if let Some(value) = body.get(&var.key) {
            if let Err(e) = config::set(ctx, &var.key, value).await {
                tracing::warn!(error = %e, key = %var.key, block = block_label, "failed to set config value");
            }
        }
    }
    ok_json(&serde_json::json!({"message": "Settings saved"}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(key: &str, name: &str, input_type: InputType) -> ConfigVar {
        ConfigVar::new(key, "desc text", "def")
            .name(name)
            .input_type(input_type)
    }

    #[test]
    fn text_field_renders_text_input_with_label_and_help() {
        let v = var("SOLOBASE_SHARED__APP_NAME", "App Name", InputType::Text);
        let s = render_field(&v, "MyApp").into_string();
        assert!(s.contains(r#"name="SOLOBASE_SHARED__APP_NAME""#));
        assert!(s.contains(r#"type="text""#));
        assert!(s.contains(r#"value="MyApp""#));
        assert!(s.contains(">App Name<"));
        assert!(s.contains("desc text"));
    }

    #[test]
    fn password_field_is_masked_with_eye_toggle_and_no_value_echoed_when_empty() {
        let v = var("X__PW", "Secret", InputType::Password);
        let set = render_field(&v, "hunter2").into_string();
        assert!(set.contains(r#"type="password""#));
        assert!(set.contains("(set)"));
        // eye toggle present
        assert!(set.contains("i.type=i.type==='password'?'text':'password'"));

        let empty = render_field(&v, "").into_string();
        assert!(empty.contains("Not configured"));
    }

    #[test]
    fn color_field_pairs_text_input_with_color_picker() {
        let v = var("X__COLOR", "Brand", InputType::Color);
        let s = render_field(&v, "#abcdef").into_string();
        assert!(s.contains(r#"type="color""#));
        assert!(s.contains("value=\"#abcdef\""));
        assert!(s.contains("onchange="));
    }

    #[test]
    fn toggle_field_renders_checkbox_checked_for_true() {
        let v = var("X__FLAG", "Flag", InputType::Toggle);
        let on = render_field(&v, "true").into_string();
        assert!(on.contains(r#"type="checkbox""#));
        assert!(on.contains("checked"));
        let off = render_field(&v, "false").into_string();
        assert!(!off.contains("checked"));
    }

    #[test]
    fn textarea_field_renders_multiline_with_value_as_content() {
        let v = var("X__FOOTER", "Footer Text", InputType::Textarea);
        let s = render_field(&v, "© 2026 Me").into_string();
        assert!(s.contains("<textarea"), "should render a textarea: {s}");
        assert!(s.contains(r#"name="X__FOOTER""#));
        // A textarea carries its value as element content, not a value= attr.
        assert!(s.contains("© 2026 Me"));
        assert!(
            !s.contains(r#"type="text""#),
            "textarea must not be a text input: {s}"
        );
        assert!(s.contains(">Footer Text<"));
    }

    #[test]
    fn field_falls_back_to_key_when_name_empty() {
        let v = ConfigVar::new("X__NONAME", "", "").input_type(InputType::Text);
        let s = render_field(&v, "").into_string();
        assert!(s.contains(">X__NONAME<"));
    }

    #[test]
    fn submit_js_interpolates_post_url_safely() {
        let js = submit_js("/b/products/admin/settings");
        assert!(js.contains(r#"fetch("/b/products/admin/settings""#));
        // a quote in the url must not break out of the string literal
        let js2 = submit_js(r#"/x"); alert(1);//"#);
        assert!(!js2.contains(r#"fetch("/x");"#));
    }
}
