//! GitHub OAuth provider — authorize URL assembly, code exchange, and
//! organisation-admin check. The registry constructs one of these per
//! deployment when the three `SOLOBASE_SHARED__AUTH__GITHUB__*` env vars
//! are populated.

use async_trait::async_trait;
use serde::Deserialize;
use url::Url;

use super::{OAuthProvider, ProviderError, ProviderProfile};

const DEFAULT_AUTHORIZE: &str = "https://github.com/login/oauth/authorize";
const DEFAULT_TOKEN: &str = "https://github.com/login/oauth/access_token";
const DEFAULT_USER: &str = "https://api.github.com/user";
const DEFAULT_USER_EMAILS: &str = "https://api.github.com/user/emails";
const DEFAULT_USER_MEMBERSHIPS: &str = "https://api.github.com/user/memberships/orgs";
const USER_AGENT: &str = "suppers-ai-auth/1";

pub(super) struct GithubProvider {
    client_id: String,
    client_secret: String,
    redirect_url: String,
    authorize_url_base: String,
    token_url: String,
    user_url: String,
    user_emails_url: String,
    user_memberships_url: String,
    http: reqwest::Client,
}

impl GithubProvider {
    pub(super) fn new(client_id: String, client_secret: String, redirect_url: String) -> Self {
        Self::with_endpoints(
            client_id,
            client_secret,
            redirect_url,
            DEFAULT_AUTHORIZE.into(),
            DEFAULT_TOKEN.into(),
            DEFAULT_USER.into(),
            DEFAULT_USER_EMAILS.into(),
            DEFAULT_USER_MEMBERSHIPS.into(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn with_endpoints(
        client_id: String,
        client_secret: String,
        redirect_url: String,
        authorize_url_base: String,
        token_url: String,
        user_url: String,
        user_emails_url: String,
        user_memberships_url: String,
    ) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_url,
            authorize_url_base,
            token_url,
            user_url,
            user_emails_url,
            user_memberships_url,
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
struct UserResponse {
    /// GitHub returns `id` as a JSON number; we stringify it for storage.
    id: serde_json::Value,
    login: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct EmailEntry {
    email: String,
    #[serde(default)]
    primary: bool,
    #[serde(default)]
    verified: bool,
}

#[async_trait]
impl OAuthProvider for GithubProvider {
    fn name(&self) -> &'static str {
        "github"
    }

    fn authorize_url(&self, state: &str, pkce_challenge: &str) -> String {
        let mut u = Url::parse(&self.authorize_url_base).expect("authorize url parseable");
        u.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_url)
            .append_pair("state", state)
            .append_pair("code_challenge", pkce_challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("scope", "read:org user:email");
        u.into()
    }

    async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: &str,
    ) -> Result<ProviderProfile, ProviderError> {
        // 1. Exchange code for access token.
        let token_resp = self
            .http
            .post(&self.token_url)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("code", code),
                ("redirect_uri", self.redirect_url.as_str()),
                ("code_verifier", pkce_verifier),
            ])
            .send()
            .await
            .map_err(|e| ProviderError::Upstream(e.to_string()))?;
        if !token_resp.status().is_success() {
            return Err(ProviderError::Upstream(format!(
                "token endpoint status {}",
                token_resp.status()
            )));
        }
        let token: TokenResponse = token_resp
            .json()
            .await
            .map_err(|e| ProviderError::BadResponse(e.to_string()))?;

        // 2. Fetch user profile.
        let user: UserResponse = self
            .http
            .get(&self.user_url)
            .header("Authorization", format!("token {}", token.access_token))
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| ProviderError::Upstream(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError::Upstream(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError::BadResponse(e.to_string()))?;

        // 3. Fetch emails; prefer primary+verified, else primary, else first.
        let emails: Vec<EmailEntry> = self
            .http
            .get(&self.user_emails_url)
            .header("Authorization", format!("token {}", token.access_token))
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| ProviderError::Upstream(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError::Upstream(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError::BadResponse(e.to_string()))?;
        let chosen = emails
            .iter()
            .find(|e| e.primary && e.verified)
            .or_else(|| emails.iter().find(|e| e.primary))
            .or_else(|| emails.first());

        let provider_ref = match &user.id {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            other => {
                return Err(ProviderError::BadResponse(format!(
                    "unexpected user.id type: {other}"
                )));
            }
        };

        Ok(ProviderProfile {
            provider_ref,
            login: user.login.clone(),
            email: chosen.map(|e| e.email.clone()),
            email_verified: chosen.map(|e| e.verified).unwrap_or(false),
            display_name: user.name.unwrap_or(user.login),
            avatar_url: user.avatar_url,
            access_token: token.access_token,
        })
    }

    async fn check_org_admin(
        &self,
        access_token: &str,
        org_ref: &str,
    ) -> Result<bool, ProviderError> {
        let url = format!(
            "{}/{}",
            self.user_memberships_url.trim_end_matches('/'),
            org_ref
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("token {access_token}"))
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| ProviderError::Upstream(e.to_string()))?;
        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Ok(false);
        }
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(ProviderError::Unauthorized);
        }
        if !status.is_success() {
            return Err(ProviderError::Upstream(format!(
                "memberships endpoint status {status}"
            )));
        }
        #[derive(Deserialize)]
        struct Membership {
            state: String,
            role: String,
        }
        let m: Membership = resp
            .json()
            .await
            .map_err(|e| ProviderError::BadResponse(e.to_string()))?;
        Ok(m.state == "active" && m.role == "admin")
    }
}

#[cfg(test)]
mod tests {
    use wiremock::{
        matchers::{body_string_contains, header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;

    fn provider_for(server: &MockServer) -> GithubProvider {
        GithubProvider::with_endpoints(
            "cid".into(),
            "csec".into(),
            "https://app.example/cb".into(),
            format!("{}/login/oauth/authorize", server.uri()),
            format!("{}/login/oauth/access_token", server.uri()),
            format!("{}/user", server.uri()),
            format!("{}/user/emails", server.uri()),
            format!("{}/user/memberships/orgs", server.uri()),
        )
    }

    #[test]
    fn authorize_url_includes_required_params() {
        let p = GithubProvider::new("cid".into(), "csec".into(), "https://app.example/cb".into());
        let url = p.authorize_url("STATE123", "CHAL456");
        assert!(url.contains("client_id=cid"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fapp.example%2Fcb"));
        assert!(url.contains("state=STATE123"));
        assert!(url.contains("code_challenge=CHAL456"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("scope=read%3Aorg+user%3Aemail"));
    }

    #[tokio::test]
    async fn exchange_code_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .and(body_string_contains("code=abc"))
            .and(body_string_contains("code_verifier=ver"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "gho_xyz",
                "token_type": "bearer",
                "scope": "read:org,user:email"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/user"))
            .and(header("authorization", "token gho_xyz"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 42,
                "login": "alice",
                "name": "Alice A",
                "avatar_url": "https://a/avatar.png"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/user/emails"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "email": "alice@example.com", "primary": true, "verified": true },
                { "email": "other@example.com", "primary": false, "verified": true }
            ])))
            .mount(&server)
            .await;

        let profile = provider_for(&server)
            .exchange_code("abc", "ver")
            .await
            .unwrap();
        assert_eq!(profile.provider_ref, "42");
        assert_eq!(profile.login, "alice");
        assert_eq!(profile.email.as_deref(), Some("alice@example.com"));
        assert!(profile.email_verified);
        assert_eq!(profile.display_name, "Alice A");
        assert_eq!(profile.avatar_url.as_deref(), Some("https://a/avatar.png"));
        assert_eq!(profile.access_token, "gho_xyz");
    }

    #[tokio::test]
    async fn exchange_code_token_endpoint_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let err = provider_for(&server)
            .exchange_code("c", "v")
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Upstream(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn exchange_code_no_primary_verified_email() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "tok", "token_type": "bearer"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/user"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 1, "login": "bob", "name": "Bob"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/user/emails"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "email": "bob@example.com", "primary": true, "verified": false }
            ])))
            .mount(&server)
            .await;

        let profile = provider_for(&server).exchange_code("c", "v").await.unwrap();
        assert_eq!(profile.email.as_deref(), Some("bob@example.com"));
        assert!(!profile.email_verified);
    }

    #[tokio::test]
    async fn check_org_admin_active_admin_is_true() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user/memberships/orgs/acme"))
            .and(header("authorization", "token tok"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "state": "active", "role": "admin"
            })))
            .mount(&server)
            .await;

        let got = provider_for(&server)
            .check_org_admin("tok", "acme")
            .await
            .unwrap();
        assert!(got);
    }

    #[tokio::test]
    async fn check_org_admin_member_is_false() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user/memberships/orgs/acme"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "state": "active", "role": "member"
            })))
            .mount(&server)
            .await;
        assert!(!provider_for(&server)
            .check_org_admin("tok", "acme")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn check_org_admin_pending_is_false() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user/memberships/orgs/acme"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "state": "pending", "role": "admin"
            })))
            .mount(&server)
            .await;
        assert!(!provider_for(&server)
            .check_org_admin("tok", "acme")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn check_org_admin_404_is_false() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user/memberships/orgs/acme"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        assert!(!provider_for(&server)
            .check_org_admin("tok", "acme")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn check_org_admin_401_is_unauthorized() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user/memberships/orgs/acme"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;
        let err = provider_for(&server)
            .check_org_admin("tok", "acme")
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Unauthorized));
    }

    #[tokio::test]
    async fn check_org_admin_500_is_upstream() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user/memberships/orgs/acme"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let err = provider_for(&server)
            .check_org_admin("tok", "acme")
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Upstream(_)));
    }
}
