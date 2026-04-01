//! Server-side rendered auth pages (login, signup, change-password).
//!
//! Uses maud for compile-time HTML generation. Settings are read from
//! the `variables` table at render time.

use crate::blocks::helpers::RecordExt;
use crate::ui::{self, SiteConfig};
use maud::{html, Markup, PreEscaped};
use std::collections::HashMap;
use wafer_core::clients::database as db;
use wafer_core::clients::database::ListOptions;
use wafer_run::context::Context;
use wafer_run::types::*;

/// Read all key-value pairs from the variables table.
async fn load_variables(ctx: &dyn Context) -> HashMap<String, String> {
    let opts = ListOptions {
        limit: 100,
        ..Default::default()
    };
    let mut settings = HashMap::new();
    if let Ok(result) = db::list(ctx, "variables", &opts).await {
        for record in &result.records {
            let key = record.str_field("key").to_string();
            let value = record.str_field("value").to_string();
            if !key.is_empty() {
                settings.insert(key, value);
            }
        }
    }
    settings
}

fn get<'a>(settings: &'a HashMap<String, String>, key: &str, default: &'a str) -> &'a str {
    settings.get(key).map(|s| s.as_str()).unwrap_or(default)
}

/// Build SiteConfig from the variables settings map.
fn site_config(settings: &HashMap<String, String>) -> SiteConfig {
    SiteConfig {
        app_name: get(settings, "APP_NAME", "Solobase").to_string(),
        logo_url: {
            let auth_logo = get(settings, "AUTH_LOGO_URL", "");
            if auth_logo.is_empty() {
                get(settings, "LOGO_URL", "").to_string()
            } else {
                auth_logo.to_string()
            }
        },
        logo_icon_url: get(settings, "LOGO_ICON_URL", "").to_string(),
        favicon_url: get(settings, "FAVICON_URL", "").to_string(),
    }
}

/// Password field with visibility toggle.
fn pw_field(id: &str, placeholder: &str, minlength: Option<&str>) -> Markup {
    html! {
        div .pw-wrap {
            input
                type="password"
                class="form-input"
                id=(id)
                placeholder=(placeholder)
                required
                minlength=[minlength];
            button type="button" class="pw-toggle" onclick={"togglePw(this)"} {
                (ui::icons::eye_off())
            }
        }
    }
}

/// JS for password visibility toggle.
fn pw_toggle_js() -> &'static str {
    r#"function togglePw(b){var i=b.parentElement.querySelector('input');if(i.type==='password'){i.type='text'}else{i.type='password'}}"#
}

// ---------------------------------------------------------------------------
// Login page
// ---------------------------------------------------------------------------

pub async fn login_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let settings = load_variables(ctx).await;
    let config = site_config(&settings);
    let app_name = get(&settings, "APP_NAME", "Solobase");
    let allow_signup = get(&settings, "ALLOW_SIGNUP", "false") == "true";
    let raw_redirect = msg.get_meta("req.query.redirect").to_string();
    // Validate redirect — only allow relative paths (prevent open redirect)
    let redirect = if raw_redirect.starts_with('/') && !raw_redirect.starts_with("//") {
        raw_redirect
    } else {
        String::new()
    };
    let post_login = get(&settings, "POST_LOGIN_REDIRECT", "/b/admin/").to_string();
    let logo_url = &config.logo_url;

    let signup_redirect = if redirect.is_empty() {
        String::new()
    } else {
        format!("?redirect={redirect}")
    };

    let markup = ui::layout::centered_page(
        "Sign In",
        &config,
        html! {
            div .login-container {
                div .login-logo {
                    @if !logo_url.is_empty() {
                        img .logo-image src=(logo_url) alt=(app_name);
                    }
                    p .login-subtitle { "Sign in to " (app_name) }
                }

                div #error .login-error style="display:none" {}
                div #info style="background:#ecfdf5;border:1px solid #a7f3d0;border-radius:8px;padding:.75rem;margin-bottom:1.5rem;font-size:.813rem;color:#059669;display:none" {}

                form #form .login-form onsubmit="return handleLogin(event)" {
                    input type="hidden" #redirect value=(redirect);
                    input type="hidden" #post_login value=(post_login);

                    div .form-group {
                        label .form-label for="email" { "Email" }
                        input .form-input type="email" #email placeholder="you@example.com" required;
                    }

                    div .form-group {
                        label .form-label for="password" { "Password" }
                        (pw_field("password", "Enter your password", None))
                    }

                    div style="text-align:right;margin-bottom:1rem" {
                        button type="button" class="btn btn-ghost btn-sm" onclick="handleForgot()" {
                            "Forgot password?"
                        }
                    }

                    button .login-button type="submit" #btn { "Sign In" }
                }

                @if allow_signup {
                    div .signup-link {
                        "Don't have an account? "
                        a href={"/b/auth/signup" (signup_redirect)} { "Sign up" }
                    }
                }
            }

            script { (PreEscaped(pw_toggle_js())) }
            script { (PreEscaped(r#"
var $=function(id){return document.getElementById(id)};
function showErr(m){var e=$('error');e.textContent=m;e.style.display='flex';$('info').style.display='none'}
function showInfo(m){var i=$('info');i.textContent=m;i.style.display='block';$('error').style.display='none'}
async function handleLogin(ev){
  ev.preventDefault();
  var btn=$('btn');btn.disabled=true;btn.textContent='Signing in...';
  $('error').style.display='none';$('info').style.display='none';
  try{
    var r=await fetch('/b/auth/login',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({email:$('email').value,password:$('password').value})});
    var d=await r.json();
    if(!r.ok||d.error){showErr((d.error&&d.error.message)||d.error||d.message||'Invalid credentials');btn.disabled=false;btn.textContent='Sign In';return false}
    var redir=$('redirect').value||$('post_login').value||'/';
    window.location.href=redir;
  }catch(ex){showErr('Something went wrong');btn.disabled=false;btn.textContent='Sign In'}
  return false;
}
async function handleForgot(){
  var email=$('email').value.trim();
  if(!email){showErr('Enter your email address first.');return}
  $('error').style.display='none';$('info').style.display='none';
  try{await fetch('/b/auth/forgot-password',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({email:email})})}catch(e){}
  showInfo('If that email is registered, a password reset link has been sent.');
}
"#)) }
        },
    );

    ui::html_response(msg, markup)
}

// ---------------------------------------------------------------------------
// Signup page
// ---------------------------------------------------------------------------

pub async fn signup_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let settings = load_variables(ctx).await;
    let config = site_config(&settings);
    let app_name = get(&settings, "APP_NAME", "Solobase");
    let allow_signup = get(&settings, "ALLOW_SIGNUP", "false") == "true";
    let raw_redirect = msg.get_meta("req.query.redirect").to_string();
    // Validate redirect — only allow relative paths (prevent open redirect)
    let redirect = if raw_redirect.starts_with('/') && !raw_redirect.starts_with("//") {
        raw_redirect
    } else {
        String::new()
    };
    let logo_url = &config.logo_url;

    if !allow_signup {
        return login_page(ctx, msg).await;
    }

    let redirect_qs = if redirect.is_empty() {
        String::new()
    } else {
        format!("?redirect={redirect}")
    };

    let markup = ui::layout::centered_page(
        "Sign Up",
        &config,
        html! {
            div .login-container {
                div .login-logo {
                    @if !logo_url.is_empty() {
                        img .logo-image src=(logo_url) alt=(app_name);
                    }
                    p .login-subtitle { "Create your " (app_name) " account" }
                }

                div #error .login-error style="display:none" {}

                div #success style="text-align:center;display:none" {
                    div style="width:48px;height:48px;background:#ecfdf5;border-radius:50%;display:flex;align-items:center;justify-content:center;margin:0 auto 1rem;font-size:1.5rem;color:#10b981" { "✓" }
                    h2 style="font-size:1.25rem;font-weight:700;margin:0 0 .5rem" { "Check your email" }
                    p #verify-msg style="font-size:.875rem;color:#64748b;line-height:1.6;margin:0 0 1.5rem" {}
                    a .login-button href={"/b/auth/login" (redirect_qs)} style="display:inline-block;width:auto;padding:.625rem 1.25rem;text-decoration:none" {
                        "Back to Sign In"
                    }
                }

                form #form .login-form onsubmit="return handleSignup(event)" {
                    input type="hidden" #redirect value=(redirect);

                    div .form-group {
                        label .form-label for="email" { "Email" }
                        input .form-input type="email" #email placeholder="you@example.com" required;
                    }

                    div .form-group {
                        label .form-label for="password" { "Password" }
                        (pw_field("password", "Min 8 characters", Some("8")))
                    }

                    button .login-button type="submit" #btn { "Create Account" }
                }

                div #signin-link .signup-link {
                    "Already have an account? "
                    a href={"/b/auth/login" (redirect_qs)} { "Sign in" }
                }
            }

            script { (PreEscaped(pw_toggle_js())) }
            script { (PreEscaped(r#"
var $=function(id){return document.getElementById(id)};
function showErr(m){var e=$('error');e.textContent=m;e.style.display='flex'}
async function handleSignup(ev){
  ev.preventDefault();
  var btn=$('btn');btn.disabled=true;btn.textContent='Creating account...';
  $('error').style.display='none';
  var email=$('email').value,pw=$('password').value;
  try{
    var r=await fetch('/b/auth/signup',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({email:email,password:pw})});
    var d=await r.json();
    if(!r.ok||d.error){showErr((d.error&&d.error.message)||d.error||d.message||'Signup failed');btn.disabled=false;btn.textContent='Create Account';return false}
    if(d.email_verified===false){
      $('form').style.display='none';$('signin-link').style.display='none';
      $('verify-msg').textContent='We sent a verification link to '+email+'. Click the link to activate your account.';
      $('success').style.display='block';
    }else{
      var redir=$('redirect').value||'/b/auth/login';
      window.location.href=redir;
    }
  }catch(ex){showErr('Something went wrong');btn.disabled=false;btn.textContent='Create Account'}
  return false;
}
"#)) }
        },
    );

    ui::html_response(msg, markup)
}

// ---------------------------------------------------------------------------
// Change password page (requires auth — caller must check)
// ---------------------------------------------------------------------------

pub async fn change_password_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let settings = load_variables(ctx).await;
    let config = site_config(&settings);
    let logo_url = &config.logo_url;

    let markup = ui::layout::centered_page(
        "Change Password",
        &config,
        html! {
            div .login-container {
                div .login-logo {
                    @if !logo_url.is_empty() {
                        img .logo-image src=(logo_url) alt="Solobase";
                    }
                    p .login-subtitle { "Change your password" }
                }

                div #error .login-error style="display:none" {}

                div #success style="background:#ecfdf5;border:1px solid #a7f3d0;border-radius:8px;padding:1rem;text-align:center;display:none" {
                    p style="font-size:.875rem;color:#16a34a;margin:0 0 1rem;font-weight:500" {
                        "Password changed successfully!"
                    }
                    button .login-button onclick="history.back()" style="width:auto;display:inline-block;padding:.625rem 1.25rem" {
                        "Go Back"
                    }
                }

                form #form .login-form onsubmit="return handleChange(event)" {
                    div .form-group {
                        label .form-label for="current" { "Current Password" }
                        (pw_field("current", "Enter your current password", None))
                    }

                    div .form-group {
                        label .form-label for="newpw" { "New Password" }
                        (pw_field("newpw", "Min 8 characters", Some("8")))
                    }

                    div .form-group {
                        label .form-label for="confirm" { "Confirm New Password" }
                        (pw_field("confirm", "Repeat new password", Some("8")))
                    }

                    button .login-button type="submit" #btn { "Change Password" }
                }

                div style="text-align:center;margin-top:1rem" {
                    a .btn .btn-ghost href="javascript:history.back()" { "Cancel" }
                }
            }

            script { (PreEscaped(pw_toggle_js())) }
            script { (PreEscaped(r#"
var $=function(id){return document.getElementById(id)};
function showErr(m){var e=$('error');e.textContent=m;e.style.display='flex'}
async function handleChange(ev){
  ev.preventDefault();
  var btn=$('btn');$('error').style.display='none';
  var pw=$('newpw').value,cf=$('confirm').value;
  if(pw!==cf){showErr('New passwords do not match.');return false}
  if(pw.length<8){showErr('Password must be at least 8 characters.');return false}
  btn.disabled=true;btn.textContent='Changing...';
  try{
    var r=await fetch('/b/auth/change-password',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({current_password:$('current').value,new_password:pw})});
    var d=await r.json();
    if(!r.ok||d.error){showErr((d.error&&d.error.message)||d.error||'Failed to change password');btn.disabled=false;btn.textContent='Change Password';return false}
    $('form').style.display='none';$('success').style.display='block';
  }catch(ex){showErr('Something went wrong');btn.disabled=false;btn.textContent='Change Password'}
  return false;
}
"#)) }
        },
    );

    ui::html_response(msg, markup)
}
