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

/// `SOLOBASE_SHARED__AUTH__SIGNUP_ENABLED` — gates the signup page and
/// POST /auth/signup handler. When false the signup link is suppressed on
/// the login page and the endpoints return 404.
pub const SIGNUP_ENABLED_KEY: &str = "SOLOBASE_SHARED__AUTH__SIGNUP_ENABLED";

/// `SOLOBASE_SHARED__AUTH__PASSWORD_MIN_LENGTH` — minimum password length
/// enforced at signup. Existing accounts are not re-validated.
pub const PASSWORD_MIN_LENGTH_KEY: &str = "SOLOBASE_SHARED__AUTH__PASSWORD_MIN_LENGTH";

/// Default session lifetime when the config var is unset.
pub const SESSION_LIFETIME_DAYS_DEFAULT: u32 = 30;

/// Default value for [`SIGNUP_ENABLED_KEY`]. Signup is off by default —
/// self-hosters must flip this explicitly.
pub const SIGNUP_ENABLED_DEFAULT: bool = false;

/// Default value for [`PASSWORD_MIN_LENGTH_KEY`].
pub const PASSWORD_MIN_LENGTH_DEFAULT: u32 = 8;

/// Config vars contributed by the Plan A2 auth block additions.
///
/// Appended to the existing legacy `config_keys` list; do not duplicate or
/// re-order with the existing vars.
pub fn auth_config_vars() -> Vec<ConfigVar> {
    let mut vars = vec![
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
        ConfigVar::new(
            SIGNUP_ENABLED_KEY,
            "If true, GET /auth/signup and POST /auth/signup are enabled and a 'create account' link appears on the login page.",
            "false",
        )
        .name("Signup Enabled")
        .input_type(InputType::Toggle),
        ConfigVar::new(
            PASSWORD_MIN_LENGTH_KEY,
            "Minimum password length enforced at signup. Existing accounts are not re-validated.",
            &PASSWORD_MIN_LENGTH_DEFAULT.to_string(),
        )
        .name("Password Minimum Length"),
    ];
    vars.extend(oauth_provider_config_vars());
    vars
}

/// Nine `ConfigVar`s (3 providers × 3 keys each) declaring the
/// `SOLOBASE_SHARED__AUTH__{GITHUB,GOOGLE,MICROSOFT}__{CLIENT_ID,
/// CLIENT_SECRET,REDIRECT_URL}` triple. All optional — a provider whose
/// triple is incomplete is simply absent from the runtime registry.
fn oauth_provider_config_vars() -> Vec<ConfigVar> {
    let mut out = Vec::with_capacity(9);
    for (provider_key, provider_name) in [
        ("GITHUB", "GitHub"),
        ("GOOGLE", "Google"),
        ("MICROSOFT", "Microsoft"),
    ] {
        out.push(
            ConfigVar::new(
                &format!("SOLOBASE_SHARED__AUTH__{provider_key}__CLIENT_ID"),
                &format!("{provider_name} OAuth client ID"),
                "",
            )
            .name(&format!("{provider_name} Client ID"))
            .optional(),
        );
        out.push(
            ConfigVar::new(
                &format!("SOLOBASE_SHARED__AUTH__{provider_key}__CLIENT_SECRET"),
                &format!("{provider_name} OAuth client secret"),
                "",
            )
            .name(&format!("{provider_name} Client Secret"))
            .input_type(InputType::Password)
            .optional(),
        );
        out.push(
            ConfigVar::new(
                &format!("SOLOBASE_SHARED__AUTH__{provider_key}__REDIRECT_URL"),
                &format!("{provider_name} OAuth callback URL"),
                "",
            )
            .name(&format!("{provider_name} Redirect URL"))
            .input_type(InputType::Url)
            .optional(),
        );
    }
    out
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
    pub signup_enabled: bool,
    pub password_min_length: u32,
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
        let signup_enabled = env
            .get(SIGNUP_ENABLED_KEY)
            .map(|s| matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
            .unwrap_or(SIGNUP_ENABLED_DEFAULT);
        let password_min_length = env
            .get(PASSWORD_MIN_LENGTH_KEY)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(PASSWORD_MIN_LENGTH_DEFAULT);
        Self {
            session_lifetime_days,
            bootstrap_admin_email: non_empty(env.get(BOOTSTRAP_ADMIN_EMAIL_KEY)),
            bootstrap_admin_password: non_empty(env.get(BOOTSTRAP_ADMIN_PASSWORD_KEY)),
            bootstrap_admin_token: non_empty(env.get(BOOTSTRAP_ADMIN_TOKEN_KEY)),
            signup_enabled,
            password_min_length,
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

    #[test]
    fn auth_config_vars_declare_all_oauth_triples() {
        let keys: Vec<String> = auth_config_vars().iter().map(|v| v.key.clone()).collect();
        for provider in ["GITHUB", "GOOGLE", "MICROSOFT"] {
            for suffix in ["CLIENT_ID", "CLIENT_SECRET", "REDIRECT_URL"] {
                let expected = format!("SOLOBASE_SHARED__AUTH__{provider}__{suffix}");
                assert!(keys.contains(&expected), "missing {expected}");
            }
        }
    }

    #[test]
    fn oauth_client_secrets_are_marked_sensitive() {
        for provider in ["GITHUB", "GOOGLE", "MICROSOFT"] {
            let key = format!("SOLOBASE_SHARED__AUTH__{provider}__CLIENT_SECRET");
            let var = auth_config_vars()
                .into_iter()
                .find(|v| v.key == key)
                .unwrap_or_else(|| panic!("{key} declared"));
            assert!(var.is_sensitive(), "{key} must be sensitive");
            assert!(var.optional, "{key} must be optional");
        }
    }

    #[test]
    fn auth_config_vars_declares_signup_enabled_and_password_min_length() {
        let keys: Vec<String> = auth_config_vars().iter().map(|v| v.key.clone()).collect();
        assert!(keys.contains(&SIGNUP_ENABLED_KEY.to_string()));
        assert!(keys.contains(&PASSWORD_MIN_LENGTH_KEY.to_string()));
    }

    #[test]
    fn signup_enabled_var_defaults_to_false_as_toggle() {
        let var = auth_config_vars()
            .into_iter()
            .find(|v| v.key == SIGNUP_ENABLED_KEY)
            .expect("signup_enabled declared");
        assert_eq!(var.default, "false");
        assert_eq!(var.input_type, InputType::Toggle);
        assert!(!var.is_sensitive());
    }

    #[test]
    fn password_min_length_var_defaults_to_eight() {
        let var = auth_config_vars()
            .into_iter()
            .find(|v| v.key == PASSWORD_MIN_LENGTH_KEY)
            .expect("password_min_length declared");
        assert_eq!(var.default, "8");
    }

    #[test]
    fn signup_enabled_parses_bool_from_map() {
        let cfg = AuthConfig::from_env_for_test(&[(SIGNUP_ENABLED_KEY, "true")]);
        assert!(cfg.signup_enabled);
        let cfg = AuthConfig::from_env_for_test(&[(SIGNUP_ENABLED_KEY, "false")]);
        assert!(!cfg.signup_enabled);
    }

    #[test]
    fn signup_enabled_defaults_false_when_unset() {
        let cfg = AuthConfig::from_env_for_test(&[]);
        assert!(!cfg.signup_enabled);
    }

    #[test]
    fn password_min_length_parses_int_from_map() {
        let cfg = AuthConfig::from_env_for_test(&[(PASSWORD_MIN_LENGTH_KEY, "12")]);
        assert_eq!(cfg.password_min_length, 12);
    }

    #[test]
    fn password_min_length_defaults_to_eight_when_unset() {
        let cfg = AuthConfig::from_env_for_test(&[]);
        assert_eq!(cfg.password_min_length, 8);
    }

    #[test]
    fn oauth_client_ids_and_redirect_urls_are_optional_and_not_sensitive() {
        for provider in ["GITHUB", "GOOGLE", "MICROSOFT"] {
            for suffix in ["CLIENT_ID", "REDIRECT_URL"] {
                let key = format!("SOLOBASE_SHARED__AUTH__{provider}__{suffix}");
                let var = auth_config_vars()
                    .into_iter()
                    .find(|v| v.key == key)
                    .unwrap_or_else(|| panic!("{key} declared"));
                assert!(var.optional, "{key} must be optional");
                assert!(!var.is_sensitive(), "{key} must not be sensitive");
            }
        }
    }
}
