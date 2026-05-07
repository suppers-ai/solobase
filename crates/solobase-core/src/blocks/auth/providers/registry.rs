//! OAuth provider registry — runtime map of `Arc<dyn OAuthProvider>` keyed
//! by provider name. Looked up by [`AuthService::verify_org_admin`] to
//! dispatch upstream membership checks.
//!
//! Construction is via [`ProviderRegistry::from_map`] (or
//! [`ProviderRegistry::empty`] for the no-provider default in
//! `BlockState::new`). Tests inject fake providers through `from_map`.

use std::{collections::HashMap, sync::Arc};

use super::OAuthProvider;

/// Runtime-visible map of enabled providers, keyed by `provider.name()`.
pub struct ProviderRegistry {
    inner: HashMap<&'static str, Arc<dyn OAuthProvider>>,
}

impl ProviderRegistry {
    /// Construct directly from a map. Used by integration tests that want
    /// to register a fake provider.
    pub fn from_map(inner: HashMap<&'static str, Arc<dyn OAuthProvider>>) -> Self {
        Self { inner }
    }

    /// Empty registry — no providers enabled.
    pub fn empty() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Look up a provider by its stable name (`"github"`, `"google"`, …).
    pub fn get(&self, name: &str) -> Option<Arc<dyn OAuthProvider>> {
        self.inner.get(name).cloned()
    }

    /// List names of currently-enabled providers. Order is unspecified.
    pub fn enabled_names(&self) -> Vec<&'static str> {
        self.inner.keys().copied().collect()
    }
}
