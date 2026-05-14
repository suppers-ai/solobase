//! GET /b/auth/oauth/login — relocated from auth/oauth.rs::handle_oauth_login in Task 5.

use std::{collections::HashMap, time::Duration};

use sha2::{Digest, Sha256};
use wafer_core::clients::{config, crypto};
use wafer_run::{context::Context, types::Message, OutputStream};

use crate::blocks::{
    auth::helpers::urlencode,
    helpers::{err_bad_request, err_forbidden, err_internal, ok_json},
};

/// Generate a PKCE code verifier (43-128 chars, URL-safe).
fn generate_pkce_verifier() -> Result<String, String> {
    let bytes = crate::crypto::random_bytes(32)?;
    Ok(crate::crypto::base64_url_encode(&bytes))
}

/// Compute S256 code challenge from a verifier.
fn pkce_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    crate::crypto::base64_url_encode(&hash)
}

pub async fn handle(ctx: &dyn Context, msg: &Message) -> OutputStream {
    // Check ENABLE_OAUTH flag
    let enable_oauth = config::get_default(ctx, "SOLOBASE_SHARED__ENABLE_OAUTH", "false").await;
    if enable_oauth != "true" && enable_oauth != "1" {
        return err_forbidden("OAuth login is not enabled");
    }

    let provider = msg.query("provider");
    if provider.is_empty() {
        return err_bad_request("Missing provider parameter");
    }

    let client_id_key = format!(
        "SUPPERS_AI__AUTH_UI__OAUTH_{}_CLIENT_ID",
        provider.to_uppercase()
    );
    let client_id = match config::get(ctx, &client_id_key).await {
        Ok(id) => id,
        Err(_) => return err_bad_request(&format!("OAuth provider '{}' not configured", provider)),
    };

    let redirect_uri = config::get_default(
        ctx,
        "SUPPERS_AI__AUTH_UI__OAUTH_REDIRECT_URI",
        "http://localhost:8090/b/auth/oauth/callback",
    )
    .await;

    // Generate PKCE code verifier and challenge
    let code_verifier = match generate_pkce_verifier() {
        Ok(v) => v,
        Err(e) => return err_internal("Failed to generate PKCE verifier", e),
    };
    let code_challenge = pkce_challenge(&code_verifier);

    // Generate CSRF state token (signed JWT containing the provider name + PKCE verifier)
    let mut state_claims = HashMap::new();
    state_claims.insert(
        "provider".to_string(),
        serde_json::Value::String(provider.to_string()),
    );
    state_claims.insert(
        "type".to_string(),
        serde_json::Value::String("oauth_state".to_string()),
    );
    state_claims.insert(
        "code_verifier".to_string(),
        serde_json::Value::String(code_verifier),
    );
    let state = match crypto::sign(ctx, &state_claims, Duration::from_secs(600)).await {
        Ok(s) => s,
        Err(e) => return err_internal("Failed to generate state", e),
    };

    let auth_url = match provider {
        "google" => format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&state={}&code_challenge={}&code_challenge_method=S256",
            client_id, redirect_uri, urlencode(&state), urlencode(&code_challenge)
        ),
        "github" => format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=user:email&state={}",
            client_id, redirect_uri, urlencode(&state)
        ),
        "microsoft" => format!(
            "https://login.microsoftonline.com/common/oauth2/v2.0/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&state={}&code_challenge={}&code_challenge_method=S256",
            client_id, redirect_uri, urlencode(&state), urlencode(&code_challenge)
        ),
        _ => return err_bad_request(&format!("Unsupported provider: {}", provider)),
    };

    ok_json(&serde_json::json!({
        "auth_url": auth_url,
        "provider": provider
    }))
}
