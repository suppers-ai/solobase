//! Google OAuth provider — OIDC authorization-code flow + `/userinfo`.
//! Wired up by the registry when `SOLOBASE_SHARED__AUTH__GOOGLE__*` vars
//! are present. `check_org_admin` has no Google equivalent and returns
//! [`ProviderError::NotSupported`] without a network call.

use serde::Deserialize;
use url::Url;

use super::{OAuthProvider, ProviderError, ProviderProfile};

const DEFAULT_AUTHORIZE: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const DEFAULT_TOKEN: &str = "https://oauth2.googleapis.com/token";
const DEFAULT_USERINFO: &str = "https://openidconnect.googleapis.com/v1/userinfo";
const USER_AGENT: &str = "suppers-ai-auth/1";

pub(super) struct GoogleProvider {
    client_id: String,
    client_secret: String,
    redirect_url: String,
    authorize_url_base: String,
    token_url: String,
    userinfo_url: String,
    http: reqwest::Client,
}

impl GoogleProvider {
    pub(super) fn new(client_id: String, client_secret: String, redirect_url: String) -> Self {
        Self::with_endpoints(
            client_id,
            client_secret,
            redirect_url,
            DEFAULT_AUTHORIZE.into(),
            DEFAULT_TOKEN.into(),
            DEFAULT_USERINFO.into(),
        )
    }

    pub(super) fn with_endpoints(
        client_id: String,
        client_secret: String,
        redirect_url: String,
        authorize_url_base: String,
        token_url: String,
        userinfo_url: String,
    ) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_url,
            authorize_url_base,
            token_url,
            userinfo_url,
            http: super::oauth_http_client(USER_AGENT, std::time::Duration::from_secs(10))
                .expect("reqwest client"),
        }
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct Userinfo {
    sub: String,
    email: String,
    /// Google omits `email_verified` for accounts it has already verified
    /// upstream (mandatory at signup) — default to `true` so a missing
    /// claim doesn't falsely flag the email as unverified.
    #[serde(default = "default_true")]
    email_verified: bool,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    picture: Option<String>,
}

fn default_true() -> bool {
    true
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl OAuthProvider for GoogleProvider {
    fn name(&self) -> &'static str {
        "google"
    }

    fn authorize_url(&self, state: &str, pkce_challenge: &str) -> String {
        let mut u = Url::parse(&self.authorize_url_base).expect("authorize url parseable");
        u.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_url)
            .append_pair("response_type", "code")
            .append_pair("scope", "openid email profile")
            .append_pair("state", state)
            .append_pair("code_challenge", pkce_challenge)
            .append_pair("code_challenge_method", "S256");
        u.into()
    }

    async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: &str,
    ) -> Result<ProviderProfile, ProviderError> {
        let token: TokenResponse = self
            .http
            .post(&self.token_url)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("code", code),
                ("redirect_uri", self.redirect_url.as_str()),
                ("grant_type", "authorization_code"),
                ("code_verifier", pkce_verifier),
            ])
            .send()
            .await
            .map_err(|e| ProviderError::Upstream(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError::Upstream(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError::BadResponse(e.to_string()))?;

        let info: Userinfo = self
            .http
            .get(&self.userinfo_url)
            .header("Authorization", format!("Bearer {}", token.access_token))
            .send()
            .await
            .map_err(|e| ProviderError::Upstream(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError::Upstream(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError::BadResponse(e.to_string()))?;

        Ok(ProviderProfile {
            provider_ref: info.sub,
            login: info.email.clone(),
            display_name: info.name.unwrap_or_else(|| info.email.clone()),
            avatar_url: info.picture,
            email_verified: info.email_verified,
            email: Some(info.email),
            access_token: token.access_token,
        })
    }

    async fn check_org_admin(
        &self,
        _access_token: &str,
        _org_ref: &str,
    ) -> Result<bool, ProviderError> {
        Err(ProviderError::NotSupported)
    }
}

#[cfg(test)]
mod tests {
    use wiremock::{
        matchers::{body_string_contains, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;

    fn provider_for(server: &MockServer) -> GoogleProvider {
        GoogleProvider::with_endpoints(
            "cid".into(),
            "csec".into(),
            "https://app.example/cb".into(),
            format!("{}/o/oauth2/v2/auth", server.uri()),
            format!("{}/token", server.uri()),
            format!("{}/v1/userinfo", server.uri()),
        )
    }

    #[test]
    fn authorize_url_includes_required_params() {
        let p = GoogleProvider::new("cid".into(), "s".into(), "https://app/cb".into());
        let url = p.authorize_url("S", "C");
        assert!(url.contains("client_id=cid"));
        assert!(url.contains("state=S"));
        assert!(url.contains("code_challenge=C"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("scope=openid+email+profile"));
        assert!(url.contains("response_type=code"));
    }

    #[tokio::test]
    async fn exchange_code_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("code_verifier=ver"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "ya29.xyz",
                "token_type": "Bearer",
                "expires_in": 3600
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sub": "google-42",
                "email": "alice@example.com",
                "email_verified": true,
                "name": "Alice",
                "picture": "https://p/alice.png"
            })))
            .mount(&server)
            .await;

        let p = provider_for(&server)
            .exchange_code("c", "ver")
            .await
            .unwrap();
        assert_eq!(p.provider_ref, "google-42");
        assert_eq!(p.login, "alice@example.com");
        assert_eq!(p.email.as_deref(), Some("alice@example.com"));
        assert!(p.email_verified);
        assert_eq!(p.display_name, "Alice");
        assert_eq!(p.access_token, "ya29.xyz");
    }

    #[tokio::test]
    async fn check_org_admin_not_supported_without_network() {
        // No MockServer needed — the method must short-circuit.
        let p = GoogleProvider::new("cid".into(), "s".into(), "https://app/cb".into());
        let err = p.check_org_admin("tok", "acme").await.unwrap_err();
        assert!(matches!(err, ProviderError::NotSupported));
    }

    #[tokio::test]
    async fn exchange_code_token_500() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let err = provider_for(&server)
            .exchange_code("c", "v")
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Upstream(_)));
    }
}
