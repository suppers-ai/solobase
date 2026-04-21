//! GitHub OAuth provider stub — real implementation lands in Plan B
//! Cluster 2 Task 5. Cluster 1 introduces the type so the registry
//! (Task 9) can construct one when the three GitHub env vars are set.

use async_trait::async_trait;

use super::{OAuthProvider, ProviderError, ProviderProfile};

/// GitHub OAuth provider. Configured with a `(client_id, client_secret,
/// redirect_url)` triple read from `SOLOBASE_SHARED__AUTH__GITHUB__*`.
pub(super) struct GithubProvider {
    #[allow(dead_code)] // Used by Task 5 (exchange_code + authorize_url).
    client_id: String,
    #[allow(dead_code)]
    client_secret: String,
    #[allow(dead_code)]
    redirect_url: String,
}

impl GithubProvider {
    pub(super) fn new(client_id: String, client_secret: String, redirect_url: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_url,
        }
    }
}

#[async_trait]
impl OAuthProvider for GithubProvider {
    fn name(&self) -> &'static str {
        "github"
    }

    fn authorize_url(&self, _state: &str, _pkce_challenge: &str) -> String {
        // Real impl lands in Task 5.
        unimplemented!("GithubProvider::authorize_url — Plan B Cluster 2 Task 5")
    }

    async fn exchange_code(
        &self,
        _code: &str,
        _pkce_verifier: &str,
    ) -> Result<ProviderProfile, ProviderError> {
        unimplemented!("GithubProvider::exchange_code — Plan B Cluster 2 Task 5")
    }

    async fn check_org_admin(
        &self,
        _access_token: &str,
        _org_ref: &str,
    ) -> Result<bool, ProviderError> {
        unimplemented!("GithubProvider::check_org_admin — Plan B Cluster 2 Task 5")
    }
}
