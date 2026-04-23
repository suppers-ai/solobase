//! Shared test double for the [`OAuthProvider`] trait.
//!
//! Used by the OAuth callback integration tests. Per spec §7, this is the
//! same shape the future registry integration tests will reuse — kept
//! decoupled from any particular provider's wire contract.

use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use solobase_core::blocks::auth::providers::{OAuthProvider, ProviderError, ProviderProfile};

/// Test double. `register_code` maps a callback `code` → `ProviderProfile`;
/// `register_admin` controls `check_org_admin` lookups.
pub struct FakeGithub {
    pub name: &'static str,
    pub profiles_by_code: Mutex<HashMap<String, ProviderProfile>>,
    pub admin_of: Mutex<HashMap<(String, String), bool>>,
    pub last_authorize_args: Mutex<Option<(String, String)>>,
}

impl FakeGithub {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            profiles_by_code: Mutex::new(HashMap::new()),
            admin_of: Mutex::new(HashMap::new()),
            last_authorize_args: Mutex::new(None),
        }
    }

    pub fn register_code(&self, code: &str, profile: ProviderProfile) {
        self.profiles_by_code
            .lock()
            .unwrap()
            .insert(code.into(), profile);
    }

    #[allow(dead_code)]
    pub fn register_admin(&self, access_token: &str, org: &str, is_admin: bool) {
        self.admin_of
            .lock()
            .unwrap()
            .insert((access_token.into(), org.into()), is_admin);
    }
}

#[async_trait]
impl OAuthProvider for FakeGithub {
    fn name(&self) -> &'static str {
        self.name
    }

    fn authorize_url(&self, state: &str, pkce_challenge: &str) -> String {
        *self.last_authorize_args.lock().unwrap() = Some((state.into(), pkce_challenge.into()));
        format!("https://fake/authorize?state={state}&cc={pkce_challenge}")
    }

    async fn exchange_code(
        &self,
        code: &str,
        _pkce_verifier: &str,
    ) -> Result<ProviderProfile, ProviderError> {
        self.profiles_by_code
            .lock()
            .unwrap()
            .get(code)
            .cloned()
            .ok_or_else(|| ProviderError::Upstream("no such code".into()))
    }

    async fn check_org_admin(
        &self,
        access_token: &str,
        org_ref: &str,
    ) -> Result<bool, ProviderError> {
        Ok(*self
            .admin_of
            .lock()
            .unwrap()
            .get(&(access_token.into(), org_ref.into()))
            .unwrap_or(&false))
    }
}
