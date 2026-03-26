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

/// Read public settings from the variables table.
async fn load_public_settings(ctx: &dyn Context) -> HashMap<String, String> {
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
fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Login page
// ---------------------------------------------------------------------------

pub async fn login_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let settings = load_public_settings(ctx).await;
    let app_name = esc(get(&settings, "APP_NAME", "Solobase"));
    let allow_signup = get(&settings, "ALLOW_SIGNUP", "false") == "true";
    let redirect = msg.get_meta("req.query.redirect").to_string();
    let redirect_esc = esc(&redirect);
    let post_login = esc(get(&settings, "POST_LOGIN_REDIRECT", "/blocks/admin/frontend/"));

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
input[type=email],input[type=password]{{width:100%;padding:.625rem .75rem;border:1px solid #e2e8f0;border-radius:8px;font-size:.875rem;outline:none}}
input:focus{{border-color:#fe6627}}
.field{{margin-bottom:1rem}}
.btn{{width:100%;padding:.75rem;background:#fe6627;color:white;border:none;border-radius:8px;font-size:.875rem;font-weight:600;cursor:pointer}}
.btn:hover{{background:#e55a20}}
.btn:disabled{{opacity:.7;cursor:not-allowed}}
.forgot{{text-align:right;margin-bottom:1rem}}
.forgot button{{background:none;border:none;color:#64748b;cursor:pointer;font-size:.75rem}}
.error{{background:#fef2f2;border:1px solid #fecaca;border-radius:8px;padding:.75rem;margin-bottom:1rem;font-size:.813rem;color:#dc2626;display:none}}
.info{{background:#ecfdf5;border:1px solid #a7f3d0;border-radius:8px;padding:.75rem;margin-bottom:1rem;font-size:.813rem;color:#059669;display:none}}
</style>
</head>
<body>
<div class="wrap">
  <div class="logo">
    <img src="/images/logo_long.png" alt="{app_name}">
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
        <input type="password" id="password" placeholder="Enter your password" required>
      </div>
      <div class="forgot"><button type="button" onclick="handleForgot()">Forgot password?</button></div>
      <button type="submit" class="btn" id="btn">Sign In</button>
    </form>
    {signup_link}
  </div>
</div>
<script>
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
    let settings = load_public_settings(ctx).await;
    let app_name = esc(get(&settings, "APP_NAME", "Solobase"));
    let allow_signup = get(&settings, "ALLOW_SIGNUP", "false") == "true";
    let redirect = msg.get_meta("req.query.redirect").to_string();
    let redirect_esc = esc(&redirect);

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
input[type=email],input[type=password]{{width:100%;padding:.625rem .75rem;border:1px solid #e2e8f0;border-radius:8px;font-size:.875rem;outline:none}}
input:focus{{border-color:#fe6627}}
.field{{margin-bottom:1rem}}
.btn{{width:100%;padding:.75rem;background:#fe6627;color:white;border:none;border-radius:8px;font-size:.875rem;font-weight:600;cursor:pointer}}
.btn:hover{{background:#e55a20}}
.btn:disabled{{opacity:.7;cursor:not-allowed}}
.error{{background:#fef2f2;border:1px solid #fecaca;border-radius:8px;padding:.75rem;margin-bottom:1rem;font-size:.813rem;color:#dc2626;display:none}}
.success{{text-align:center;display:none}}
.success .icon{{width:48px;height:48px;background:#ecfdf5;border-radius:50%;display:flex;align-items:center;justify-content:center;margin:0 auto 1rem;font-size:1.5rem;color:#10b981}}
</style>
</head>
<body>
<div class="wrap">
  <div class="logo">
    <img src="/images/logo_long.png" alt="{app_name}">
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
        <input type="password" id="password" placeholder="Min 8 characters" required minlength="8">
      </div>
      <button type="submit" class="btn" id="btn">Create Account</button>
    </form>
    <div id="signin-link" style="text-align:center;margin-top:1rem;font-size:.813rem;color:#64748b">Already have an account? <a href="/auth/login{}" style="color:#fe6627;font-weight:600;text-decoration:none">Sign in</a></div>
  </div>
</div>
<script>
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

pub async fn change_password_page(_ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Change Password</title>
<link rel="icon" type="image/x-icon" href="/favicon.ico">
<style>
*{box-sizing:border-box}
body{margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;background:#f8fafc;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;color:#1e293b}
.wrap{width:100%;max-width:420px;padding:2rem}
.logo{display:flex;flex-direction:column;align-items:center;margin-bottom:1.5rem}
.logo img{height:36px;width:auto;margin-bottom:.75rem}
.logo p{font-size:.875rem;color:#64748b;margin:0}
.card{background:white;border:1px solid #e2e8f0;border-radius:12px;padding:2rem}
label{display:block;font-size:.813rem;font-weight:500;margin-bottom:.375rem}
input[type=password]{width:100%;padding:.625rem .75rem;border:1px solid #e2e8f0;border-radius:8px;font-size:.875rem;outline:none}
input:focus{border-color:#fe6627}
.field{margin-bottom:1rem}
.btn{width:100%;padding:.75rem;background:#fe6627;color:white;border:none;border-radius:8px;font-size:.875rem;font-weight:600;cursor:pointer}
.btn:hover{background:#e55a20}
.btn:disabled{opacity:.7;cursor:not-allowed}
.error{background:#fef2f2;border:1px solid #fecaca;border-radius:8px;padding:.75rem;margin-bottom:1rem;font-size:.813rem;color:#dc2626;display:none}
.success{background:#ecfdf5;border:1px solid #a7f3d0;border-radius:8px;padding:1rem;text-align:center;display:none}
.success p{font-size:.875rem;color:#16a34a;margin:0 0 1rem;font-weight:500}
.back{display:block;text-align:center;margin-top:1rem;font-size:.813rem;color:#64748b;text-decoration:none}
</style>
</head>
<body>
<div class="wrap">
  <div class="logo">
    <img src="/images/logo_long.png" alt="Solobase">
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
        <input type="password" id="current" placeholder="Enter your current password" required>
      </div>
      <div class="field">
        <label for="newpw">New Password</label>
        <input type="password" id="newpw" placeholder="Min 8 characters" required minlength="8">
      </div>
      <div class="field" style="margin-bottom:1.5rem">
        <label for="confirm">Confirm New Password</label>
        <input type="password" id="confirm" placeholder="Repeat new password" required minlength="8">
      </div>
      <button type="submit" class="btn" id="btn">Change Password</button>
    </form>
    <a href="javascript:history.back()" class="back">Cancel</a>
  </div>
</div>
<script>
var $=function(id){return document.getElementById(id)};
function showErr(m){var e=$('error');e.textContent=m;e.style.display='block'}
async function handleChange(ev){
  ev.preventDefault();
  var btn=$('btn');$('error').style.display='none';
  var pw=$('newpw').value,cf=$('confirm').value;
  if(pw!==cf){showErr('New passwords do not match.');return false}
  if(pw.length<8){showErr('Password must be at least 8 characters.');return false}
  btn.disabled=true;btn.textContent='Changing...';
  try{
    var r=await fetch('/auth/change-password',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({current_password:$('current').value,new_password:pw})});
    var d=await r.json();
    if(!r.ok||d.error){showErr((d.error&&d.error.message)||d.error||'Failed to change password');btn.disabled=false;btn.textContent='Change Password';return false}
    $('form').style.display='none';$('success').style.display='block';
  }catch(ex){showErr('Something went wrong');btn.disabled=false;btn.textContent='Change Password'}
  return false;
}
</script>
</body></html>"#;

    ResponseBuilder::new(msg).body(html.as_bytes().to_vec(), "text/html; charset=utf-8")
}
