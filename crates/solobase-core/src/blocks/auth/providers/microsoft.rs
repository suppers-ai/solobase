//! Microsoft OAuth provider stub — real implementation lands in Plan B
//! Cluster 2 Task 7. Cluster 1 introduces the type so the registry
//! (Task 9) can construct one when the three Microsoft env vars are set.

use async_trait::async_trait;

use super::{OAuthProvider, ProviderError, ProviderProfile};

pub(super) struct MicrosoftProvider {
    #[allow(dead_code)]
    client_id: String,
    #[allow(dead_code)]
    client_secret: String,
    #[allow(dead_code)]
    redirect_url: String,
}

impl MicrosoftProvider {
    pub(super) fn new(client_id: String, client_secret: String, redirect_url: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_url,
        }
    }
}

#[async_trait]
impl OAuthProvider for MicrosoftProvider {
    fn name(&self) -> &'static str {
        "microsoft"
    }

    fn authorize_url(&self, _state: &str, _pkce_challenge: &str) -> String {
        unimplemented!("MicrosoftProvider::authorize_url — Plan B Cluster 2 Task 7")
    }

    async fn exchange_code(
        &self,
        _code: &str,
        _pkce_verifier: &str,
    ) -> Result<ProviderProfile, ProviderError> {
        unimplemented!("MicrosoftProvider::exchange_code — Plan B Cluster 2 Task 7")
    }

    async fn check_org_admin(
        &self,
        _access_token: &str,
        _org_ref: &str,
    ) -> Result<bool, ProviderError> {
        Err(ProviderError::NotSupported)
    }
}
