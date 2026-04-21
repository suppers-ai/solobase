//! Google OAuth provider stub — real implementation lands in Plan B
//! Cluster 2 Task 6. Cluster 1 introduces the type so the registry
//! (Task 9) can construct one when the three Google env vars are set.

use async_trait::async_trait;

use super::{OAuthProvider, ProviderError, ProviderProfile};

pub(super) struct GoogleProvider {
    #[allow(dead_code)]
    client_id: String,
    #[allow(dead_code)]
    client_secret: String,
    #[allow(dead_code)]
    redirect_url: String,
}

impl GoogleProvider {
    pub(super) fn new(client_id: String, client_secret: String, redirect_url: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_url,
        }
    }
}

#[async_trait]
impl OAuthProvider for GoogleProvider {
    fn name(&self) -> &'static str {
        "google"
    }

    fn authorize_url(&self, _state: &str, _pkce_challenge: &str) -> String {
        unimplemented!("GoogleProvider::authorize_url — Plan B Cluster 2 Task 6")
    }

    async fn exchange_code(
        &self,
        _code: &str,
        _pkce_verifier: &str,
    ) -> Result<ProviderProfile, ProviderError> {
        unimplemented!("GoogleProvider::exchange_code — Plan B Cluster 2 Task 6")
    }

    async fn check_org_admin(
        &self,
        _access_token: &str,
        _org_ref: &str,
    ) -> Result<bool, ProviderError> {
        // Google doesn't expose org-membership in the same shape as GitHub —
        // Task 6 confirms `NotSupported` is the right answer here.
        Err(ProviderError::NotSupported)
    }
}
