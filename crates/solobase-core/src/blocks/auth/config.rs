//! Config vars and runtime config struct for the `suppers-ai/auth` block.
//!
//! Two complementary surfaces:
//!
//! - [`auth_config_vars`] declares the `ConfigVar`s the block contributes to
//!   `BlockInfo::config_keys`, so the admin UI and validator see them. These
//!   are appended to the existing (legacy JWT-based) vars in `mod.rs`.
//! - [`AuthConfig`] is the runtime view — populated from `wafer-run/config` at
//!   `Init` time (or from a `HashMap` in tests via [`AuthConfig::from_env_for_test`]).
//!   Downstream handlers consume it without re-reading config on every call.
//!
//! Naming follows CLAUDE.md's three-tier convention:
//! - `SOLOBASE_SHARED__AUTH__*` — shared auth config, admin-writable.

use std::collections::HashMap;

use wafer_run::types::{ConfigVar, InputType};

/// `SOLOBASE_SHARED__AUTH__SESSION_LIFETIME_DAYS` — how many days a freshly
/// issued session cookie stays valid.
pub const SESSION_LIFETIME_DAYS_KEY: &str = "SOLOBASE_SHARED__AUTH__SESSION_LIFETIME_DAYS";

/// `SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_EMAIL` — email of the admin user to
/// create on first startup.
pub const BOOTSTRAP_ADMIN_EMAIL_KEY: &str = "SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_EMAIL";

/// `SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_PASSWORD` — password for the
/// bootstrap admin. Paired with the email key.
pub const BOOTSTRAP_ADMIN_PASSWORD_KEY: &str = "SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_PASSWORD";

/// `SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_TOKEN` — one-time token used when
/// no email/password is provided. Hashed and stored in `bootstrap_tokens`;
/// the holder redeems it to create the first admin.
pub const BOOTSTRAP_ADMIN_TOKEN_KEY: &str = "SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_TOKEN";

/// Default session lifetime when the config var is unset.
pub const SESSION_LIFETIME_DAYS_DEFAULT: u32 = 30;

/// Config vars contributed by the Plan A2 auth block additions.
///
/// Appended to the existing legacy `config_keys` list; do not duplicate or
/// re-order with the existing vars.
pub fn auth_config_vars() -> Vec<ConfigVar> {
    vec![
        ConfigVar::new(
            SESSION_LIFETIME_DAYS_KEY,
            "Lifetime of a session cookie in days (applied at issuance)",
            &SESSION_LIFETIME_DAYS_DEFAULT.to_string(),
        )
        .name("Session Lifetime (days)"),
        ConfigVar::new(
            BOOTSTRAP_ADMIN_EMAIL_KEY,
            "Email of the admin user created on first startup",
            "",
        )
        .name("Bootstrap Admin Email")
        .optional(),
        ConfigVar::new(
            BOOTSTRAP_ADMIN_PASSWORD_KEY,
            "Password for the bootstrap admin account",
            "",
        )
        .name("Bootstrap Admin Password")
        .input_type(InputType::Password)
        .optional(),
        ConfigVar::new(
            BOOTSTRAP_ADMIN_TOKEN_KEY,
            "One-time token for provisioning the first admin (sha256 stored)",
            "",
        )
        .name("Bootstrap Admin Token")
        .input_type(InputType::Password)
        .optional(),
    ]
}

/// Runtime view of the auth block's config.
///
/// Populated once at `Init` time from `wafer-run/config` or, in tests, from
/// a `HashMap` via [`AuthConfig::from_env_for_test`]. Consuming handlers read
/// from this struct rather than reaching back to the config client per-call.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub session_lifetime_days: u32,
    pub bootstrap_admin_email: Option<String>,
    pub bootstrap_admin_password: Option<String>,
    pub bootstrap_admin_token: Option<String>,
}

impl AuthConfig {
    /// Construct from a fully-populated `HashMap`. Missing keys fall back to
    /// declared defaults; empty strings are treated as absent for the optional
    /// bootstrap vars (so shell exports like `FOO=""` do not accidentally
    /// trigger the bootstrap email+password path).
    pub fn from_map(env: &HashMap<String, String>) -> Self {
        let session_lifetime_days = env
            .get(SESSION_LIFETIME_DAYS_KEY)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(SESSION_LIFETIME_DAYS_DEFAULT);
        Self {
            session_lifetime_days,
            bootstrap_admin_email: non_empty(env.get(BOOTSTRAP_ADMIN_EMAIL_KEY)),
            bootstrap_admin_password: non_empty(env.get(BOOTSTRAP_ADMIN_PASSWORD_KEY)),
            bootstrap_admin_token: non_empty(env.get(BOOTSTRAP_ADMIN_TOKEN_KEY)),
        }
    }

    /// Test helper: build an [`AuthConfig`] from a slice of `(key, value)`
    /// pairs. Unlisted keys pick up the declared defaults, matching what the
    /// config client does in production.
    pub fn from_env_for_test(pairs: &[(&str, &str)]) -> Self {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        Self::from_map(&map)
    }
}

fn non_empty<S: AsRef<str>>(s: Option<S>) -> Option<String> {
    s.map(|s| s.as_ref().to_string()).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_lifetime_days_defaults_to_30_when_unset() {
        let cfg = AuthConfig::from_env_for_test(&[]);
        assert_eq!(cfg.session_lifetime_days, 30);
    }

    #[test]
    fn session_lifetime_days_parses_int() {
        let cfg = AuthConfig::from_env_for_test(&[(SESSION_LIFETIME_DAYS_KEY, "7")]);
        assert_eq!(cfg.session_lifetime_days, 7);
    }

    #[test]
    fn bootstrap_admin_vars_are_captured() {
        let cfg = AuthConfig::from_env_for_test(&[
            (BOOTSTRAP_ADMIN_EMAIL_KEY, "a@b.c"),
            (BOOTSTRAP_ADMIN_PASSWORD_KEY, "pw"),
        ]);
        assert_eq!(cfg.bootstrap_admin_email.as_deref(), Some("a@b.c"));
        assert_eq!(cfg.bootstrap_admin_password.as_deref(), Some("pw"));
        assert!(cfg.bootstrap_admin_token.is_none());
    }

    #[test]
    fn bootstrap_token_only() {
        let cfg = AuthConfig::from_env_for_test(&[(BOOTSTRAP_ADMIN_TOKEN_KEY, "secret")]);
        assert_eq!(cfg.bootstrap_admin_token.as_deref(), Some("secret"));
        assert!(cfg.bootstrap_admin_email.is_none());
    }

    #[test]
    fn empty_string_bootstrap_vars_are_treated_as_absent() {
        let cfg = AuthConfig::from_env_for_test(&[
            (BOOTSTRAP_ADMIN_EMAIL_KEY, ""),
            (BOOTSTRAP_ADMIN_PASSWORD_KEY, ""),
            (BOOTSTRAP_ADMIN_TOKEN_KEY, ""),
        ]);
        assert!(cfg.bootstrap_admin_email.is_none());
        assert!(cfg.bootstrap_admin_password.is_none());
        assert!(cfg.bootstrap_admin_token.is_none());
    }

    #[test]
    fn auth_config_vars_declares_all_four_keys() {
        let vars = auth_config_vars();
        let keys: Vec<&str> = vars.iter().map(|v| v.key.as_str()).collect();
        assert!(keys.contains(&SESSION_LIFETIME_DAYS_KEY));
        assert!(keys.contains(&BOOTSTRAP_ADMIN_EMAIL_KEY));
        assert!(keys.contains(&BOOTSTRAP_ADMIN_PASSWORD_KEY));
        assert!(keys.contains(&BOOTSTRAP_ADMIN_TOKEN_KEY));
    }

    #[test]
    fn bootstrap_password_is_marked_secret() {
        let var = auth_config_vars()
            .into_iter()
            .find(|v| v.key == BOOTSTRAP_ADMIN_PASSWORD_KEY)
            .expect("password var declared");
        assert!(var.is_sensitive(), "bootstrap password must be sensitive");
        assert!(var.optional, "bootstrap password must be optional");
    }

    #[test]
    fn bootstrap_token_is_marked_secret_and_optional() {
        let var = auth_config_vars()
            .into_iter()
            .find(|v| v.key == BOOTSTRAP_ADMIN_TOKEN_KEY)
            .expect("token var declared");
        assert!(var.is_sensitive());
        assert!(var.optional);
    }

    #[test]
    fn session_lifetime_var_has_default_of_30() {
        let var = auth_config_vars()
            .into_iter()
            .find(|v| v.key == SESSION_LIFETIME_DAYS_KEY)
            .expect("session var declared");
        assert_eq!(var.default, "30");
        // Not optional — session lifetime is always needed and always has a default.
        assert!(!var.optional);
    }
}
