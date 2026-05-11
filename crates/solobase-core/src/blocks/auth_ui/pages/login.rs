//! GET /b/auth/login — relocated from auth/pages/mod.rs::login_page in Task 5.

use maud::{html, PreEscaped};
use wafer_run::{context::Context, types::Message, OutputStream};

use super::{
    login_script, oauth_button_script, oauth_provider_configured, oauth_provider_icon,
    oauth_provider_label, pw_field, pw_toggle_js, site_config,
};
use crate::{
    blocks::auth::brand_panel,
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
    let redirect = if raw_redirect.starts_with('/') && !raw_redirect.starts_with("//") {
        raw_redirect
    } else {
        String::new()
    };
    let post_login_raw = ctx
        .config_get("SOLOBASE_SHARED__POST_LOGIN_REDIRECT")
        .unwrap_or("/b/admin/")
        .to_string();
    let post_login = if post_login_raw.starts_with('/') && !post_login_raw.starts_with("//") {
        post_login_raw
    } else {
        "/b/admin/".to_string()
    };
    let logo_url = &config.logo_url;

    let signup_redirect = if redirect.is_empty() {
        String::new()
    } else {
        format!("?redirect={redirect}")
    };

    // OAuth buttons appear only when ENABLE_OAUTH is on AND the provider's
    // full credential triple (CLIENT_ID + CLIENT_SECRET + REDIRECT_URL) is
    // present in env. Avoids rendering a "Continue with GitHub" button that
    // would 4xx as soon as it's clicked.
    let oauth_enabled = ctx
        .config_get("SOLOBASE_SHARED__ENABLE_OAUTH")
        .unwrap_or("false")
        == "true";
    let oauth_providers: Vec<&'static str> = if oauth_enabled {
        ["github", "google", "microsoft"]
            .iter()
            .copied()
            .filter(|p| oauth_provider_configured(ctx, p))
            .collect()
    } else {
        Vec::new()
    };

    let markup = ui::layout::page(
        "Sign In",
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
                        p .login-subtitle { "Sign in to " (app_name) }
                    }

                    div #error .login-error style="display:none" {}
                    div #info style="background:#ecfdf5;border:1px solid #a7f3d0;border-radius:8px;padding:.75rem;margin-bottom:1.5rem;font-size:.813rem;color:#059669;display:none" {}

                    @if !oauth_providers.is_empty() {
                        div .oauth-buttons style="display:flex;flex-direction:column;gap:.5rem;margin-bottom:1rem" {
                            @for provider in &oauth_providers {
                                button
                                    type="button"
                                    class="oauth-button"
                                    data-provider=(provider)
                                    onclick={"oauthStart('"(provider)"')"}
                                    style="display:flex;align-items:center;justify-content:center;gap:.5rem;padding:.625rem 1rem;background:#000;color:#fff;border:1px solid #000;border-radius:.5rem;font-weight:500;font-size:.95rem;cursor:pointer;transition:background .15s" {
                                    (oauth_provider_icon(provider))
                                    "Continue with " (oauth_provider_label(provider))
                                }
                            }
                        }
                        div style="display:flex;align-items:center;gap:.75rem;margin:.5rem 0 1rem;color:var(--sa-text-muted, #6b7280);font-size:.75rem" {
                            div style="flex:1;height:1px;background:var(--sa-border, #e5e7eb)" {}
                            "or"
                            div style="flex:1;height:1px;background:var(--sa-border, #e5e7eb)" {}
                        }
                    }

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
                script { (PreEscaped(login_script())) }
                @if !oauth_providers.is_empty() {
                    script { (PreEscaped(oauth_button_script())) }
                }
            },
        ),
    );

    ui::html_response(markup)
}
