//! OAuth provider registry — builds an [`OAuthProvider`] for each provider
//! whose three env vars are fully set. Consumed by the block's HTTP layer
//! (Cluster 3) to dispatch `/auth/oauth/{provider}/…` routes.
//!
//! Env var triple per provider (per solobase's `SOLOBASE_SHARED__*` naming):
//!   * `SOLOBASE_SHARED__AUTH__<PROVIDER>__CLIENT_ID`
//!   * `SOLOBASE_SHARED__AUTH__<PROVIDER>__CLIENT_SECRET`
//!   * `SOLOBASE_SHARED__AUTH__<PROVIDER>__REDIRECT_URL`
//!
//! A provider with *any* of its three vars missing is silently dropped —
//! the callback handler surfaces this as a 404 at request time.

use std::{collections::HashMap, sync::Arc};

use super::{
    github::GithubProvider, google::GoogleProvider, microsoft::MicrosoftProvider, OAuthProvider,
};

/// Runtime-visible map of enabled providers, keyed by `provider.name()`.
pub struct ProviderRegistry {
    inner: HashMap<&'static str, Arc<dyn OAuthProvider>>,
}

impl ProviderRegistry {
    /// Construct directly from a map. Used by [`build_providers`] at startup
    /// and by integration tests that want to register a fake provider.
    pub fn from_map(inner: HashMap<&'static str, Arc<dyn OAuthProvider>>) -> Self {
        Self { inner }
    }

    /// Empty registry — no providers enabled. Callbacks return 404.
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

/// Read the three `SOLOBASE_SHARED__AUTH__<KEY>__*` vars for a single
/// provider, returning `None` if any are missing.
fn triple_from(env: &HashMap<String, String>, key: &str) -> Option<(String, String, String)> {
    let cid = env
        .get(&format!("SOLOBASE_SHARED__AUTH__{key}__CLIENT_ID"))
        .cloned()?;
    let secret = env
        .get(&format!("SOLOBASE_SHARED__AUTH__{key}__CLIENT_SECRET"))
        .cloned()?;
    let redirect = env
        .get(&format!("SOLOBASE_SHARED__AUTH__{key}__REDIRECT_URL"))
        .cloned()?;
    Some((cid, secret, redirect))
}

/// Build a [`ProviderRegistry`] from an env-var snapshot. Caller is
/// responsible for constructing `env` (e.g. from
/// `std::env::vars().collect()` or the config block).
pub fn build_providers(env: &HashMap<String, String>) -> ProviderRegistry {
    let mut inner: HashMap<&'static str, Arc<dyn OAuthProvider>> = HashMap::new();
    if let Some((c, s, r)) = triple_from(env, "GITHUB") {
        inner.insert("github", Arc::new(GithubProvider::new(c, s, r)));
    }
    if let Some((c, s, r)) = triple_from(env, "GOOGLE") {
        inner.insert("google", Arc::new(GoogleProvider::new(c, s, r)));
    }
    if let Some((c, s, r)) = triple_from(env, "MICROSOFT") {
        inner.insert("microsoft", Arc::new(MicrosoftProvider::new(c, s, r)));
    }
    ProviderRegistry { inner }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_providers_when_all_unset() {
        let env: HashMap<String, String> = HashMap::new();
        let reg = build_providers(&env);
        assert!(reg.get("github").is_none());
        assert!(reg.get("google").is_none());
        assert!(reg.get("microsoft").is_none());
        assert_eq!(reg.enabled_names().len(), 0);
    }

    #[test]
    fn github_enabled_only_when_all_three_set() {
        let mut env = HashMap::new();
        env.insert(
            "SOLOBASE_SHARED__AUTH__GITHUB__CLIENT_ID".into(),
            "cid".into(),
        );
        // Only one set → not enabled.
        assert!(build_providers(&env).get("github").is_none());

        env.insert(
            "SOLOBASE_SHARED__AUTH__GITHUB__CLIENT_SECRET".into(),
            "sec".into(),
        );
        // Two set → still not enabled.
        assert!(build_providers(&env).get("github").is_none());

        env.insert(
            "SOLOBASE_SHARED__AUTH__GITHUB__REDIRECT_URL".into(),
            "https://app/cb".into(),
        );
        // All three → enabled.
        let reg = build_providers(&env);
        assert!(reg.get("github").is_some());
        assert_eq!(reg.enabled_names(), vec!["github"]);
    }

    #[test]
    fn all_three_enabled_when_each_triple_set() {
        let mut env = HashMap::new();
        for p in ["GITHUB", "GOOGLE", "MICROSOFT"] {
            env.insert(format!("SOLOBASE_SHARED__AUTH__{p}__CLIENT_ID"), "c".into());
            env.insert(
                format!("SOLOBASE_SHARED__AUTH__{p}__CLIENT_SECRET"),
                "s".into(),
            );
            env.insert(
                format!("SOLOBASE_SHARED__AUTH__{p}__REDIRECT_URL"),
                "https://app/cb".into(),
            );
        }
        let reg = build_providers(&env);
        let mut names = reg.enabled_names();
        names.sort();
        assert_eq!(names, vec!["github", "google", "microsoft"]);
    }
}
