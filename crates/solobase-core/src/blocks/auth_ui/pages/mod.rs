//! SSR page handlers for the auth-ui block.
//!
//! Each leaf module hosts one page handler relocated from the legacy
//! `auth/pages/` tree in Task 5 of Plan A2 PR 5.

pub mod bootstrap;
pub mod change_password;
pub mod login;
pub mod orgs;
pub mod reset_password;
pub mod settings;
pub mod signup;

use std::collections::HashMap;

use maud::{html, Markup};
use wafer_core::clients::{database as db, database::ListOptions};
use wafer_run::context::Context;

use crate::{
    blocks::helpers::RecordExt,
    ui::{self, SiteConfig},
};

/// Read all key-value pairs from the variables table.
pub(super) async fn load_variables(ctx: &dyn Context) -> HashMap<String, String> {
    let opts = ListOptions {
        limit: 100,
        ..Default::default()
    };
    let mut settings = HashMap::new();
    if let Ok(result) = db::list(ctx, crate::blocks::admin::VARIABLES_COLLECTION, &opts).await {
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

/// Lookup a shared / block-scoped setting from the variables table, falling
/// back to `default` when the key is absent. The variables table is the
/// single source of truth — process env is only read at first cold-start by
/// `admin::settings::seed_admin_variables` to populate this table.
pub(super) fn get<'a>(
    settings: &'a HashMap<String, String>,
    key: &str,
    default: &'a str,
) -> &'a str {
    settings.get(key).map(String::as_str).unwrap_or(default)
}

/// Build SiteConfig from the variables settings map.
pub(super) fn site_config(settings: &HashMap<String, String>) -> SiteConfig {
    SiteConfig {
        app_name: get(settings, "SOLOBASE_SHARED__APP_NAME", "Solobase").to_string(),
        logo_url: {
            let auth_logo = get(settings, "SOLOBASE_SHARED__AUTH_LOGO_URL", "");
            if auth_logo.is_empty() {
                get(
                    settings,
                    "SOLOBASE_SHARED__LOGO_URL",
                    "https://solobase.dev/images/logo_long.png",
                )
                .to_string()
            } else {
                auth_logo.to_string()
            }
        },
        logo_icon_url: get(
            settings,
            "SOLOBASE_SHARED__LOGO_ICON_URL",
            "https://solobase.dev/images/logo.png",
        )
        .to_string(),
        favicon_url: get(settings, "SOLOBASE_SHARED__FAVICON_URL", "").to_string(),
        embedded_scripts: get(settings, "SOLOBASE_SHARED__EMBEDDED_SCRIPTS", "")
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
    }
}

/// True if the provider has all three credentials needed for the modern
/// single-callback OAuth flow (`/auth/oauth/login`, `/auth/oauth/callback`):
///
/// - `SUPPERS_AI__AUTH_UI__OAUTH_<PROVIDER>_CLIENT_ID`
/// - `SUPPERS_AI__AUTH_UI__OAUTH_<PROVIDER>_CLIENT_SECRET`
/// - `SUPPERS_AI__AUTH_UI__OAUTH_REDIRECT_URI` (single URI, provider-agnostic;
///   the provider is encoded in the signed `state` JWT)
///
/// These match what `oauth.rs` actually reads when building the auth_url.
/// Values come from the variables table via `load_variables` on both
/// native (sqlite) and cloudflare (D1).
pub(super) fn oauth_provider_configured(
    settings: &HashMap<String, String>,
    provider: &str,
) -> bool {
    let up = provider.to_ascii_uppercase();
    !get(
        settings,
        &format!("SUPPERS_AI__AUTH_UI__OAUTH_{up}_CLIENT_ID"),
        "",
    )
    .is_empty()
        && !get(
            settings,
            &format!("SUPPERS_AI__AUTH_UI__OAUTH_{up}_CLIENT_SECRET"),
            "",
        )
        .is_empty()
        && !get(settings, "SUPPERS_AI__AUTH_UI__OAUTH_REDIRECT_URI", "").is_empty()
}

/// Display label for an OAuth provider button.
pub(super) fn oauth_provider_label(provider: &str) -> &'static str {
    match provider {
        "github" => "GitHub",
        "google" => "Google",
        "microsoft" => "Microsoft",
        _ => "OAuth",
    }
}

/// Inline SVG glyph for an OAuth provider button. Sized to sit beside text.
pub(super) fn oauth_provider_icon(provider: &str) -> Markup {
    match provider {
        "github" => html! {
            svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true" {
                path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" {}
            }
        },
        // No bespoke marks for google/microsoft yet — fall back to a
        // neutral lock icon so the button still renders visually.
        _ => html! {
            svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" {
                rect width="18" height="11" x="3" y="11" rx="2" ry="2" {}
                path d="M7 11V7a5 5 0 0 1 10 0v4" {}
            }
        },
    }
}

/// Browser-side handler for OAuth buttons. Hits the existing JSON endpoint,
/// reads `auth_url`, and redirects. The fetch path uses same-origin cookies
/// implicitly. On error we surface the message in the existing `#error`
/// area so it's consistent with the email/password flow.
pub(super) fn oauth_button_script() -> &'static str {
    r#"
async function oauthStart(provider){
  var err=document.getElementById('error');
  try{
    var r=await fetch('/b/auth/oauth/login?provider='+encodeURIComponent(provider),{credentials:'same-origin'});
    var d=await r.json();
    if(!r.ok||!d.auth_url){throw new Error((d&&d.error&&d.error.message)||d&&d.message||'OAuth start failed');}
    window.location.href=d.auth_url;
  }catch(ex){
    if(err){err.textContent=ex.message||'Failed to start OAuth flow';err.style.display='flex';}
  }
}
"#
}

/// Password field with visibility toggle.
pub(super) fn pw_field(id: &str, placeholder: &str, minlength: Option<&str>) -> Markup {
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
pub(super) fn pw_toggle_js() -> &'static str {
    r#"function togglePw(b){var i=b.parentElement.querySelector('input');if(i.type==='password'){i.type='text'}else{i.type='password'}}"#
}

/// JS that drives the login + forgot-password forms.
///
/// On browser (wasm32) targets, the server runs inside a Service Worker and
/// browsers do not persist `Set-Cookie` from SW-synthetic responses. We set
/// the auth cookie from the response body client-side in that case. On native
/// targets the server's `Set-Cookie` already works, so we emit a version of
/// this JS without the client-side assignment — no HttpOnly regression.
pub(super) fn login_script() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    {
        r#"
var $=function(id){return document.getElementById(id)};
function showErr(m){var e=$('error');e.textContent=m;e.style.display='flex';$('info').style.display='none'}
function showInfo(m){var i=$('info');i.textContent=m;i.style.display='block';$('error').style.display='none'}
async function handleLogin(ev){
  ev.preventDefault();
  var btn=$('btn');btn.disabled=true;btn.textContent='Signing in...';
  $('error').style.display='none';$('info').style.display='none';
  try{
    var r=await fetch('/b/auth/api/login',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({email:$('email').value,password:$('password').value})});
    var d=await r.json();
    if(!r.ok||d.error){showErr((d.error&&d.error.message)||d.error||d.message||'Invalid credentials');btn.disabled=false;btn.textContent='Sign In';return false}
    // Service-worker synthetic responses don't persist Set-Cookie, so set the
    // auth cookie client-side from the response body.
    if(d.access_token){
      var secure=location.protocol==='https:'?'; Secure':'';
      document.cookie='auth_token='+d.access_token+'; Path=/; SameSite=Lax; Max-Age=86400'+secure;
    }
    var redir=$('redirect').value||$('post_login').value||'/';
    window.location.href=redir;
  }catch(ex){showErr('Something went wrong');btn.disabled=false;btn.textContent='Sign In'}
  return false;
}
async function handleForgot(){
  var email=$('email').value.trim();
  if(!email){showErr('Enter your email address first.');return}
  $('error').style.display='none';$('info').style.display='none';
  try{await fetch('/b/auth/api/forgot-password',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({email:email})})}catch(e){}
  showInfo('If that email is registered, a password reset link has been sent.');
}
"#
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        r#"
var $=function(id){return document.getElementById(id)};
function showErr(m){var e=$('error');e.textContent=m;e.style.display='flex';$('info').style.display='none'}
function showInfo(m){var i=$('info');i.textContent=m;i.style.display='block';$('error').style.display='none'}
async function handleLogin(ev){
  ev.preventDefault();
  var btn=$('btn');btn.disabled=true;btn.textContent='Signing in...';
  $('error').style.display='none';$('info').style.display='none';
  try{
    var r=await fetch('/b/auth/api/login',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({email:$('email').value,password:$('password').value})});
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
  try{await fetch('/b/auth/api/forgot-password',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({email:email})})}catch(e){}
  showInfo('If that email is registered, a password reset link has been sent.');
}
"#
    }
}
