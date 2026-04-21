//! Microsoft (Entra ID) OAuth provider — authorization-code flow against
//! the `/common` tenant, followed by a Graph `/me` lookup. Wired up by
//! the registry when `SOLOBASE_SHARED__AUTH__MICROSOFT__*` vars are set.
//!
//! Note on `email_verified`: Microsoft Graph's `/me` endpoint does not
//! expose an `email_verified` claim. For Azure AD work/school accounts
//! (the default use case) the tenant has already verified the user out
//! of band, so we mark the email as verified. Personal Microsoft
//! accounts via `/common` are flagged as a later follow-up — see
//! Plan B's noted decision.

use async_trait::async_trait;
use serde::Deserialize;
use url::Url;

use super::{OAuthProvider, ProviderError, ProviderProfile};

const DEFAULT_AUTHORIZE: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";
const DEFAULT_TOKEN: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const DEFAULT_ME: &str = "https://graph.microsoft.com/v1.0/me";
const USER_AGENT: &str = "suppers-ai-auth/1";

pub(super) struct MicrosoftProvider {
    client_id: String,
    client_secret: String,
    redirect_url: String,
    authorize_url_base: String,
    token_url: String,
    me_url: String,
    http: reqwest::Client,
}

impl MicrosoftProvider {
    pub(super) fn new(client_id: String, client_secret: String, redirect_url: String) -> Self {
        Self::with_endpoints(
            client_id,
            client_secret,
            redirect_url,
            DEFAULT_AUTHORIZE.into(),
            DEFAULT_TOKEN.into(),
            DEFAULT_ME.into(),
        )
    }

    pub(super) fn with_endpoints(
        client_id: String,
        client_secret: String,
        redirect_url: String,
        authorize_url_base: String,
        token_url: String,
        me_url: String,
    ) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_url,
            authorize_url_base,
            token_url,
            me_url,
            http: reqwest::Client::builder()
                .user_agent(USER_AGENT)
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("reqwest client"),
        }
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct MeResponse {
    id: String,
    #[serde(default, rename = "userPrincipalName")]
    user_principal_name: Option<String>,
    #[serde(default)]
    mail: Option<String>,
    #[serde(default, rename = "displayName")]
    display_name: Option<String>,
}

#[async_trait]
impl OAuthProvider for MicrosoftProvider {
    fn name(&self) -> &'static str {
        "microsoft"
    }

    fn authorize_url(&self, state: &str, pkce_challenge: &str) -> String {
        let mut u = Url::parse(&self.authorize_url_base).expect("authorize url parseable");
        u.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_url)
            .append_pair("response_type", "code")
            .append_pair("scope", "openid email profile User.Read")
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

        let me: MeResponse = self
            .http
            .get(&self.me_url)
            .header("Authorization", format!("Bearer {}", token.access_token))
            .send()
            .await
            .map_err(|e| ProviderError::Upstream(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError::Upstream(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError::BadResponse(e.to_string()))?;

        let email = me.mail.clone().or_else(|| me.user_principal_name.clone());
        let login = me
            .user_principal_name
            .clone()
            .or_else(|| email.clone())
            .unwrap_or_else(|| me.id.clone());
        let display_name = me.display_name.clone().unwrap_or_else(|| login.clone());

        Ok(ProviderProfile {
            provider_ref: me.id,
            login,
            email,
            // Azure AD work/school tenants pre-verify email out of band; see
            // the module-level note on personal accounts as follow-up.
            email_verified: true,
            display_name,
            avatar_url: None,
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
    use super::*;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn provider_for(server: &MockServer) -> MicrosoftProvider {
        MicrosoftProvider::with_endpoints(
            "cid".into(),
            "csec".into(),
            "https://app.example/cb".into(),
            format!("{}/common/oauth2/v2.0/authorize", server.uri()),
            format!("{}/common/oauth2/v2.0/token", server.uri()),
            format!("{}/v1.0/me", server.uri()),
        )
    }

    #[test]
    fn authorize_url_includes_required_params() {
        let p = MicrosoftProvider::new("cid".into(), "s".into(), "https://app/cb".into());
        let url = p.authorize_url("S", "C");
        assert!(url.contains("client_id=cid"));
        assert!(url.contains("state=S"));
        assert!(url.contains("code_challenge=C"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("scope=openid+email+profile+User.Read"));
        assert!(url.contains("response_type=code"));
    }

    #[tokio::test]
    async fn exchange_code_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/common/oauth2/v2.0/token"))
            .and(body_string_contains("code_verifier=ver"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "ms-token",
                "token_type": "Bearer"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1.0/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "ms-xyz",
                "userPrincipalName": "alice@contoso.com",
                "mail": "alice@contoso.com",
                "displayName": "Alice"
            })))
            .mount(&server)
            .await;

        let p = provider_for(&server)
            .exchange_code("c", "ver")
            .await
            .unwrap();
        assert_eq!(p.provider_ref, "ms-xyz");
        assert_eq!(p.email.as_deref(), Some("alice@contoso.com"));
        // Microsoft /me does not expose email_verified; per spec we treat
        // work/school tenants as verified (see module docs).
        assert!(p.email_verified);
        assert_eq!(p.display_name, "Alice");
    }

    #[tokio::test]
    async fn check_org_admin_not_supported_without_network() {
        let p = MicrosoftProvider::new("cid".into(), "s".into(), "https://app/cb".into());
        let err = p.check_org_admin("tok", "acme").await.unwrap_err();
        assert!(matches!(err, ProviderError::NotSupported));
    }

    #[tokio::test]
    async fn exchange_code_token_500() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/common/oauth2/v2.0/token"))
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
