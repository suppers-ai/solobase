//! OAuth provider abstraction for the `suppers-ai/auth` block.
//!
//! Each concrete provider (GitHub, Google, Microsoft — landed in Cluster 2)
//! implements [`OAuthProvider`]. Callbacks in the block's HTTP layer
//! (Cluster 3) dispatch to the trait via the [`registry::ProviderRegistry`]
//! built at startup from env config.

use async_trait::async_trait;
use thiserror::Error;

pub(super) mod pkce;

/// Normalised profile returned by [`OAuthProvider::exchange_code`]. Upstream
/// GitHub/Google/Microsoft shapes collapse into this single struct so the
/// callback handler (Cluster 3 Task 12) never has to branch on provider.
#[derive(Debug, Clone)]
pub(crate) struct ProviderProfile {
    /// Stable per-provider identifier (e.g. GitHub numeric `id`, Google `sub`,
    /// Microsoft `oid`). Must be unique within the provider.
    pub provider_ref: String,
    /// Human-readable handle (GitHub login, Google email local part, etc.).
    pub login: String,
    /// Primary verified email if available. Providers that don't return an
    /// email leave this `None`.
    pub email: Option<String>,
    /// Whether the provider asserts the email address has been verified.
    pub email_verified: bool,
    /// Name suitable for UI display.
    pub display_name: String,
    /// URL to the user's avatar image if the provider returns one.
    pub avatar_url: Option<String>,
    /// The provider-issued bearer access token. Stored in `provider_links`
    /// for follow-up calls (org membership checks, …).
    pub access_token: String,
}

/// Error surface exposed by all provider implementations.
#[derive(Debug, Error)]
pub(crate) enum ProviderError {
    /// The operation isn't supported by this provider (e.g. calling
    /// `check_org_admin` on Google). Callers should treat as a definitive
    /// "no" without retry.
    #[error("operation not supported by this provider")]
    NotSupported,
    /// The provider rejected our credentials (401/403) — typically a revoked
    /// or expired access token.
    #[error("unauthorized from provider")]
    Unauthorized,
    /// Upstream HTTP or parsing failure we couldn't attribute to user error.
    #[error("upstream OAuth provider failure: {0}")]
    Upstream(String),
    /// Callback's `state` parameter didn't match the cookie value — likely
    /// CSRF or expired session.
    #[error("state mismatch")]
    BadState,
    /// The `code` parameter was missing or the provider refused the exchange.
    #[error("bad authorization code")]
    BadCode,
}

/// Abstraction over a single OAuth provider. The registry (Task 9) owns one
/// `Arc<dyn OAuthProvider>` per enabled provider and hands them out by name.
#[async_trait]
pub(crate) trait OAuthProvider: Send + Sync {
    /// Stable provider identifier as used in URLs and the registry map
    /// (e.g. `"github"`, `"google"`, `"microsoft"`).
    fn name(&self) -> &'static str;

    /// Build the full authorize URL to redirect the user to. The caller
    /// supplies the anti-CSRF `state` and PKCE `code_challenge` (S256).
    fn authorize_url(&self, state: &str, pkce_challenge: &str) -> String;

    /// Exchange the provider-returned `code` + our stored `pkce_verifier`
    /// for an access token and a normalised [`ProviderProfile`].
    async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: &str,
    ) -> Result<ProviderProfile, ProviderError>;

    /// For providers that support org-membership checks (GitHub), assert the
    /// access-token holder is an admin/owner of `org_ref`. Others return
    /// [`ProviderError::NotSupported`].
    async fn check_org_admin(
        &self,
        access_token: &str,
        org_ref: &str,
    ) -> Result<bool, ProviderError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockProvider;

    #[async_trait]
    impl OAuthProvider for MockProvider {
        fn name(&self) -> &'static str {
            "mock"
        }
        fn authorize_url(&self, state: &str, pkce_challenge: &str) -> String {
            format!("https://mock/authorize?state={state}&cc={pkce_challenge}")
        }
        async fn exchange_code(
            &self,
            _code: &str,
            _pkce_verifier: &str,
        ) -> Result<ProviderProfile, ProviderError> {
            Ok(ProviderProfile {
                provider_ref: "m1".into(),
                login: "alice".into(),
                email: Some("alice@example.com".into()),
                email_verified: true,
                display_name: "Alice".into(),
                avatar_url: None,
                access_token: "tok".into(),
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

    #[test]
    fn mock_provider_name() {
        assert_eq!(MockProvider.name(), "mock");
    }

    #[test]
    fn authorize_url_includes_state_and_challenge() {
        let url = MockProvider.authorize_url("S", "C");
        assert!(url.contains("state=S"));
        assert!(url.contains("cc=C"));
    }

    #[tokio::test]
    async fn exchange_code_returns_profile() {
        let p = MockProvider.exchange_code("c", "v").await.unwrap();
        assert_eq!(p.provider_ref, "m1");
        assert!(p.email_verified);
    }

    #[tokio::test]
    async fn check_org_admin_not_supported_by_default() {
        let err = MockProvider.check_org_admin("t", "o").await.unwrap_err();
        assert!(matches!(err, ProviderError::NotSupported));
    }
}
