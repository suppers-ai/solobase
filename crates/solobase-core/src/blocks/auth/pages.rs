//! Server-side rendered auth pages (login, signup, change-password).
//!
//! Settings are read from the `variables` table at render time so no
//! separate public settings endpoint is needed.

use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::ResponseBuilder;
use wafer_core::clients::database as db;
use wafer_core::clients::database::ListOptions;
use std::collections::HashMap;
use crate::blocks::helpers::RecordExt;

/// Read all key-value pairs from the variables table.
async fn load_variables(ctx: &dyn Context) -> HashMap<String, String> {
    let opts = ListOptions { limit: 100, ..Default::default() };
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

/// Escape HTML entities.
pub(super) fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

const EYE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" d="M2.036 12.322a1.012 1.012 0 0 1 0-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178Z"/><path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z"/></svg>"#;
const EYE_OFF_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" d="M3.98 8.223A10.477 10.477 0 0 0 1.934 12c1.292 4.338 5.31 7.5 10.066 7.5.993 0 1.953-.138 2.863-.395M6.228 6.228A10.451 10.451 0 0 1 12 4.5c4.756 0 8.773 3.162 10.065 7.498a10.522 10.522 0 0 1-4.293 5.774M6.228 6.228 3 3m3.228 3.228 3.65 3.65m7.894 7.894L21 21m-3.228-3.228-3.65-3.65m0 0a3 3 0 1 0-4.243-4.243m4.242 4.242L9.88 9.88"/></svg>"#;
const PW_TOGGLE_CSS: &str = ".pw-wrap{position:relative}.pw-wrap input{padding-right:2.5rem}.pw-toggle{position:absolute;right:.5rem;top:50%;transform:translateY(-50%);background:none;border:none;cursor:pointer;padding:.25rem;color:#94a3b8;display:flex;align-items:center}.pw-toggle:hover{color:#64748b}.pw-toggle svg{width:1.125rem;height:1.125rem}";
const PW_TOGGLE_JS: &str = r#"function togglePw(b){var i=b.parentElement.querySelector('input');if(i.type==='password'){i.type='text';b.innerHTML='EYE';}else{i.type='password';b.innerHTML='EYEOFF';}}"#;

fn pw_toggle_js() -> String {
    PW_TOGGLE_JS.replace("EYE'", &format!("{EYE_SVG}'")).replace("EYEOFF'", &format!("{EYE_OFF_SVG}'"))
}

fn pw_field(id: &str, placeholder: &str, extra_attrs: &str) -> String {
    format!(
        r#"<div class="pw-wrap"><input type="password" class="pw-input" id="{id}" placeholder="{placeholder}" required {extra_attrs}><button type="button" class="pw-toggle" onclick="togglePw(this)">{EYE_OFF_SVG}</button></div>"#
    )
}

// ---------------------------------------------------------------------------
// Login page
// ---------------------------------------------------------------------------

pub async fn login_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let settings = load_variables(ctx).await;
    let app_name = esc(get(&settings, "APP_NAME", "Solobase"));
    let allow_signup = get(&settings, "ALLOW_SIGNUP", "false") == "true";
    let redirect = msg.get_meta("req.query.redirect").to_string();
    let redirect_esc = esc(&redirect);
    let post_login = esc(get(&settings, "POST_LOGIN_REDIRECT", "/blocks/admin/frontend/"));

    let logo_url = esc(get(&settings, "AUTH_LOGO_URL", "https://solobase.dev/images/logo_long.png"));
    let pw_login = pw_field("password", "Enter your password", "");
    let pw_toggle = pw_toggle_js();

    let signup_link = if allow_signup {
        format!(
            r#"<div style="text-align:center;margin-top:1rem;font-size:.813rem;color:#64748b">Don't have an account? <a href="/auth/signup{}" style="color:#fe6627;font-weight:600;text-decoration:none">Sign up</a></div>"#,
            if redirect.is_empty() { String::new() } else { format!("?redirect={redirect_esc}") }
        )
    } else {
        String::new()
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Sign In - {app_name}</title>
<link rel="icon" type="image/x-icon" href="/favicon.ico">
<style>
*{{box-sizing:border-box}}
body{{margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;background:#f8fafc;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;color:#1e293b}}
.wrap{{width:100%;max-width:420px;padding:2rem}}
.logo{{display:flex;flex-direction:column;align-items:center;margin-bottom:1.5rem}}
.logo img{{height:36px;width:auto;margin-bottom:.75rem}}
.logo p{{font-size:.875rem;color:#64748b;margin:0}}
.card{{background:white;border:1px solid #e2e8f0;border-radius:12px;padding:2rem}}
label{{display:block;font-size:.813rem;font-weight:500;margin-bottom:.375rem}}
input[type=email],input[type=password],input[type=text].pw-input{{width:100%;padding:.625rem .75rem;border:1px solid #e2e8f0;border-radius:8px;font-size:.875rem;outline:none}}
input:focus{{border-color:#fe6627}}
.field{{margin-bottom:1rem}}
.btn{{width:100%;padding:.75rem;background:#fe6627;color:white;border:none;border-radius:8px;font-size:.875rem;font-weight:600;cursor:pointer}}
.btn:hover{{background:#e55a20}}
.btn:disabled{{opacity:.7;cursor:not-allowed}}
.forgot{{text-align:right;margin-bottom:1rem}}
.forgot button{{background:none;border:none;color:#64748b;cursor:pointer;font-size:.75rem}}
.error{{background:#fef2f2;border:1px solid #fecaca;border-radius:8px;padding:.75rem;margin-bottom:1rem;font-size:.813rem;color:#dc2626;display:none}}
.info{{background:#ecfdf5;border:1px solid #a7f3d0;border-radius:8px;padding:.75rem;margin-bottom:1rem;font-size:.813rem;color:#059669;display:none}}
{PW_TOGGLE_CSS}
</style>
</head>
<body>
<div class="wrap">
  <div class="logo">
    <img src="{logo_url}" alt="{app_name}">
    <p>Sign in to {app_name}</p>
  </div>
  <div class="card">
    <div id="error" class="error"></div>
    <div id="info" class="info"></div>
    <form id="form" onsubmit="return handleLogin(event)">
      <input type="hidden" id="redirect" value="{redirect_esc}">
      <input type="hidden" id="post_login" value="{post_login}">
      <div class="field">
        <label for="email">Email</label>
        <input type="email" id="email" placeholder="you@example.com" required>
      </div>
      <div class="field">
        <label for="password">Password</label>
        {pw_login}
      </div>
      <div class="forgot"><button type="button" onclick="handleForgot()">Forgot password?</button></div>
      <button type="submit" class="btn" id="btn">Sign In</button>
    </form>
    {signup_link}
  </div>
</div>
<script>
{pw_toggle}
var $=function(id){{return document.getElementById(id)}};
function showErr(m){{var e=$('error');e.textContent=m;e.style.display='block';$('info').style.display='none'}}
function showInfo(m){{var i=$('info');i.textContent=m;i.style.display='block';$('error').style.display='none'}}
async function handleLogin(ev){{
  ev.preventDefault();
  var btn=$('btn');btn.disabled=true;btn.textContent='Signing in...';
  $('error').style.display='none';$('info').style.display='none';
  try{{
    var r=await fetch('/auth/login',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{email:$('email').value,password:$('password').value}})}});
    var d=await r.json();
    if(!r.ok||d.error){{showErr((d.error&&d.error.message)||d.error||d.message||'Invalid credentials');btn.disabled=false;btn.textContent='Sign In';return false}}
    var redir=$('redirect').value||$('post_login').value||'/';
    window.location.href=redir;
  }}catch(ex){{showErr('Something went wrong');btn.disabled=false;btn.textContent='Sign In'}}
  return false;
}}
async function handleForgot(){{
  var email=$('email').value.trim();
  if(!email){{showErr('Enter your email address first.');return}}
  $('error').style.display='none';$('info').style.display='none';
  try{{await fetch('/auth/forgot-password',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{email:email}})}})}}catch(e){{}}
  showInfo('If that email is registered, a password reset link has been sent.');
}}
</script>
</body></html>"#
    );

    ResponseBuilder::new(msg).body(html.into_bytes(), "text/html; charset=utf-8")
}

// ---------------------------------------------------------------------------
// Signup page
// ---------------------------------------------------------------------------

pub async fn signup_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let settings = load_variables(ctx).await;
    let app_name = esc(get(&settings, "APP_NAME", "Solobase"));
    let allow_signup = get(&settings, "ALLOW_SIGNUP", "false") == "true";
    let redirect = msg.get_meta("req.query.redirect").to_string();
    let redirect_esc = esc(&redirect);

    let logo_url = esc(get(&settings, "AUTH_LOGO_URL", "https://solobase.dev/images/logo_long.png"));
    let pw_signup = pw_field("password", "Min 8 characters", r#"minlength="8""#);
    let pw_toggle = pw_toggle_js();

    if !allow_signup {
        return login_page(ctx, msg).await;
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Sign Up - {app_name}</title>
<link rel="icon" type="image/x-icon" href="/favicon.ico">
<style>
*{{box-sizing:border-box}}
body{{margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;background:#f8fafc;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;color:#1e293b}}
.wrap{{width:100%;max-width:420px;padding:2rem}}
.logo{{display:flex;flex-direction:column;align-items:center;margin-bottom:1.5rem}}
.logo img{{height:36px;width:auto;margin-bottom:.75rem}}
.logo p{{font-size:.875rem;color:#64748b;margin:0}}
.card{{background:white;border:1px solid #e2e8f0;border-radius:12px;padding:2rem}}
label{{display:block;font-size:.813rem;font-weight:500;margin-bottom:.375rem}}
input[type=email],input[type=password],input[type=text].pw-input{{width:100%;padding:.625rem .75rem;border:1px solid #e2e8f0;border-radius:8px;font-size:.875rem;outline:none}}
input:focus{{border-color:#fe6627}}
.field{{margin-bottom:1rem}}
.btn{{width:100%;padding:.75rem;background:#fe6627;color:white;border:none;border-radius:8px;font-size:.875rem;font-weight:600;cursor:pointer}}
.btn:hover{{background:#e55a20}}
.btn:disabled{{opacity:.7;cursor:not-allowed}}
.error{{background:#fef2f2;border:1px solid #fecaca;border-radius:8px;padding:.75rem;margin-bottom:1rem;font-size:.813rem;color:#dc2626;display:none}}
.success{{text-align:center;display:none}}
.success .icon{{width:48px;height:48px;background:#ecfdf5;border-radius:50%;display:flex;align-items:center;justify-content:center;margin:0 auto 1rem;font-size:1.5rem;color:#10b981}}
{PW_TOGGLE_CSS}
</style>
</head>
<body>
<div class="wrap">
  <div class="logo">
    <img src="{logo_url}" alt="{app_name}">
    <p>Create your {app_name} account</p>
  </div>
  <div class="card">
    <div id="error" class="error"></div>
    <div id="success" class="success">
      <div class="icon">&#10003;</div>
      <h2 style="font-size:1.25rem;font-weight:700;margin:0 0 .5rem">Check your email</h2>
      <p id="verify-msg" style="font-size:.875rem;color:#64748b;line-height:1.6;margin:0 0 1.5rem"></p>
      <a href="/auth/login{}" class="btn" style="display:inline-block;text-decoration:none;width:auto;padding:.625rem 1.25rem">Back to Sign In</a>
    </div>
    <form id="form" onsubmit="return handleSignup(event)">
      <input type="hidden" id="redirect" value="{redirect_esc}">
      <div class="field">
        <label for="email">Email</label>
        <input type="email" id="email" placeholder="you@example.com" required>
      </div>
      <div class="field" style="margin-bottom:1.5rem">
        <label for="password">Password</label>
        {pw_signup}
      </div>
      <button type="submit" class="btn" id="btn">Create Account</button>
    </form>
    <div id="signin-link" style="text-align:center;margin-top:1rem;font-size:.813rem;color:#64748b">Already have an account? <a href="/auth/login{}" style="color:#fe6627;font-weight:600;text-decoration:none">Sign in</a></div>
  </div>
</div>
<script>
{pw_toggle}
var $=function(id){{return document.getElementById(id)}};
function showErr(m){{var e=$('error');e.textContent=m;e.style.display='block'}}
async function handleSignup(ev){{
  ev.preventDefault();
  var btn=$('btn');btn.disabled=true;btn.textContent='Creating account...';
  $('error').style.display='none';
  var email=$('email').value,pw=$('password').value;
  try{{
    var r=await fetch('/auth/signup',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{email:email,password:pw}})}});
    var d=await r.json();
    if(!r.ok||d.error){{showErr((d.error&&d.error.message)||d.error||d.message||'Signup failed');btn.disabled=false;btn.textContent='Create Account';return false}}
    if(d.email_verified===false){{
      $('form').style.display='none';$('signin-link').style.display='none';
      $('verify-msg').textContent='We sent a verification link to '+email+'. Click the link to activate your account.';
      $('success').style.display='block';
    }}else{{
      var redir=$('redirect').value||'/auth/login';
      window.location.href=redir;
    }}
  }}catch(ex){{showErr('Something went wrong');btn.disabled=false;btn.textContent='Create Account'}}
  return false;
}}
</script>
</body></html>"#,
        if redirect.is_empty() { String::new() } else { format!("?redirect={redirect_esc}") },
        if redirect.is_empty() { String::new() } else { format!("?redirect={redirect_esc}") },
    );

    ResponseBuilder::new(msg).body(html.into_bytes(), "text/html; charset=utf-8")
}

// ---------------------------------------------------------------------------
// Change password page (requires auth — caller must check)
// ---------------------------------------------------------------------------

pub async fn change_password_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let settings = load_variables(ctx).await;
    let logo_url = esc(get(&settings, "AUTH_LOGO_URL", "https://solobase.dev/images/logo_long.png"));
    let pw_current = pw_field("current", "Enter your current password", "");
    let pw_new = pw_field("newpw", "Min 8 characters", r#"minlength="8""#);
    let pw_confirm = pw_field("confirm", "Repeat new password", r#"minlength="8""#);
    let pw_toggle = pw_toggle_js();

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Change Password</title>
<link rel="icon" type="image/x-icon" href="/favicon.ico">
<style>
*{{box-sizing:border-box}}
body{{margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;background:#f8fafc;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;color:#1e293b}}
.wrap{{width:100%;max-width:420px;padding:2rem}}
.logo{{display:flex;flex-direction:column;align-items:center;margin-bottom:1.5rem}}
.logo img{{height:36px;width:auto;margin-bottom:.75rem}}
.logo p{{font-size:.875rem;color:#64748b;margin:0}}
.card{{background:white;border:1px solid #e2e8f0;border-radius:12px;padding:2rem}}
label{{display:block;font-size:.813rem;font-weight:500;margin-bottom:.375rem}}
input[type=password],input[type=text].pw-input{{width:100%;padding:.625rem .75rem;border:1px solid #e2e8f0;border-radius:8px;font-size:.875rem;outline:none}}
input:focus{{border-color:#fe6627}}
.field{{margin-bottom:1rem}}
.btn{{width:100%;padding:.75rem;background:#fe6627;color:white;border:none;border-radius:8px;font-size:.875rem;font-weight:600;cursor:pointer}}
.btn:hover{{background:#e55a20}}
.btn:disabled{{opacity:.7;cursor:not-allowed}}
.error{{background:#fef2f2;border:1px solid #fecaca;border-radius:8px;padding:.75rem;margin-bottom:1rem;font-size:.813rem;color:#dc2626;display:none}}
.success{{background:#ecfdf5;border:1px solid #a7f3d0;border-radius:8px;padding:1rem;text-align:center;display:none}}
.success p{{font-size:.875rem;color:#16a34a;margin:0 0 1rem;font-weight:500}}
.back{{display:block;text-align:center;margin-top:1rem;font-size:.813rem;color:#64748b;text-decoration:none}}
{PW_TOGGLE_CSS}
</style>
</head>
<body>
<div class="wrap">
  <div class="logo">
    <img src="{logo_url}" alt="Solobase">
    <p>Change your password</p>
  </div>
  <div class="card">
    <div id="error" class="error"></div>
    <div id="success" class="success">
      <p>Password changed successfully!</p>
      <button onclick="history.back()" class="btn" style="width:auto;display:inline-block;padding:.625rem 1.25rem">Go Back</button>
    </div>
    <form id="form" onsubmit="return handleChange(event)">
      <div class="field">
        <label for="current">Current Password</label>
        {pw_current}
      </div>
      <div class="field">
        <label for="newpw">New Password</label>
        {pw_new}
      </div>
      <div class="field" style="margin-bottom:1.5rem">
        <label for="confirm">Confirm New Password</label>
        {pw_confirm}
      </div>
      <button type="submit" class="btn" id="btn">Change Password</button>
    </form>
    <a href="javascript:history.back()" class="back">Cancel</a>
  </div>
</div>
<script>
{pw_toggle}
var $=function(id){{return document.getElementById(id)}};
function showErr(m){{var e=$('error');e.textContent=m;e.style.display='block'}}
async function handleChange(ev){{
  ev.preventDefault();
  var btn=$('btn');$('error').style.display='none';
  var pw=$('newpw').value,cf=$('confirm').value;
  if(pw!==cf){{showErr('New passwords do not match.');return false}}
  if(pw.length<8){{showErr('Password must be at least 8 characters.');return false}}
  btn.disabled=true;btn.textContent='Changing...';
  try{{
    var r=await fetch('/auth/change-password',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{current_password:$('current').value,new_password:pw}})}});
    var d=await r.json();
    if(!r.ok||d.error){{showErr((d.error&&d.error.message)||d.error||'Failed to change password');btn.disabled=false;btn.textContent='Change Password';return false}}
    $('form').style.display='none';$('success').style.display='block';
  }}catch(ex){{showErr('Something went wrong');btn.disabled=false;btn.textContent='Change Password'}}
  return false;
}}
</script>
</body></html>"#
    );

    ResponseBuilder::new(msg).body(html.into_bytes(), "text/html; charset=utf-8")
}
