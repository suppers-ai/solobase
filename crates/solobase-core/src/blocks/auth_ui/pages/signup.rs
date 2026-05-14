//! GET /b/auth/signup — relocated from auth/pages/mod.rs::signup_page in Task 5.

use maud::{html, PreEscaped};
use wafer_run::{context::Context, types::Message, OutputStream};

use super::{pw_field, pw_toggle_js, site_config};
use crate::{
    blocks::{auth::brand_panel, auth_ui::redirect::is_safe_local_redirect},
    ui::{self, templates::auth_split},
};

pub async fn handle(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let config = site_config(ctx);
    let app_name = &config.app_name;
    let allow_signup = ctx
        .config_get("SOLOBASE_SHARED__ALLOW_SIGNUP")
        .unwrap_or("true")
        == "true";
    let raw_redirect = msg.get_meta("req.query.redirect").to_string();
    // Validate redirect — only allow relative paths (prevent open redirect)
    let redirect = if is_safe_local_redirect(&raw_redirect) {
        raw_redirect
    } else {
        String::new()
    };
    let logo_url = &config.logo_url;

    if !allow_signup {
        return super::login::handle(ctx, msg).await;
    }

    let redirect_qs = if redirect.is_empty() {
        String::new()
    } else {
        format!("?redirect={redirect}")
    };

    let markup = ui::layout::page(
        "Sign Up",
        &config,
        auth_split(
            brand_panel(&config),
            html! {
                div .login-container {
                    div .login-logo {
                        @if !logo_url.is_empty() {
                            img .logo-image src=(logo_url) alt=(app_name);
                        } @else {
                            span .login-app-name { (app_name) }
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
    var r=await fetch('/b/auth/api/signup',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({email:email,password:pw})});
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
        ),
    );

    ui::html_response(markup)
}
