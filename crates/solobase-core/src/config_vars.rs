//! Central config variable definitions.
//!
//! Shared (`SOLOBASE_SHARED__`) variables are defined here — the single source
//! of truth. Block-scoped variables are declared in each block's `BlockInfo`.
//!
//! Use `collect_all_config_vars()` to get the complete set of all known config
//! variables (shared + block-declared) for seeding, validation, and UI rendering.

use wafer_run::types::{ConfigVar, InputType};

/// Shared config variables readable by all blocks, writable only by admin.
///
/// These are NOT owned by any block — they're platform-level settings.
/// Blocks should NOT declare `SOLOBASE_SHARED__` vars in their `config_keys`.
pub fn shared_config_vars() -> Vec<ConfigVar> {
    vec![
        ConfigVar::new(
            "SOLOBASE_SHARED__APP_NAME",
            "Display name shown in UI and emails",
            "Solobase",
        )
        .name("App Name")
        .input_type(InputType::Text),
        ConfigVar::new(
            "SOLOBASE_SHARED__ALLOW_SIGNUP",
            "Allow new user registration",
            "true",
        )
        .name("Allow Signup")
        .input_type(InputType::Toggle),
        ConfigVar::new(
            "SOLOBASE_SHARED__ENABLE_OAUTH",
            "Enable third-party OAuth login",
            "false",
        )
        .name("Enable OAuth")
        .input_type(InputType::Toggle),
        ConfigVar::new(
            "SOLOBASE_SHARED__PRIMARY_COLOR",
            "Brand color used in the UI",
            "#6366f1",
        )
        .name("Primary Color")
        .input_type(InputType::Color),
        ConfigVar::new(
            "SOLOBASE_SHARED__POST_LOGIN_REDIRECT",
            "URL to redirect to after login",
            "/b/admin/",
        )
        .name("Post-Login Redirect")
        .input_type(InputType::Text),
        ConfigVar::new(
            "SOLOBASE_SHARED__FRONTEND_URL",
            "Frontend URL for checkout redirects",
            "http://localhost:5173",
        )
        .name("Frontend URL")
        .input_type(InputType::Url),
        ConfigVar::new(
            "SOLOBASE_SHARED__LOGO_URL",
            "Logo shown in header and emails",
            "",
        )
        .name("Logo URL")
        .input_type(InputType::Url),
        ConfigVar::new("SOLOBASE_SHARED__LOGO_ICON_URL", "Small icon logo", "")
            .name("Logo Icon URL")
            .input_type(InputType::Url),
        ConfigVar::new(
            "SOLOBASE_SHARED__AUTH_LOGO_URL",
            "Logo on login/signup pages (falls back to Logo URL)",
            "",
        )
        .name("Auth Logo URL")
        .input_type(InputType::Url),
        ConfigVar::new("SOLOBASE_SHARED__FAVICON_URL", "Browser tab icon", "")
            .name("Favicon URL")
            .input_type(InputType::Url),
        ConfigVar::new(
            "SOLOBASE_SHARED__FEATURE_USER_PRODUCTS",
            "Allow users to create their own products",
            "false",
        )
        .name("User Products")
        .input_type(InputType::Toggle),
        ConfigVar::new(
            "SOLOBASE_SHARED__ENVIRONMENT",
            "Runtime environment (development/production)",
            "development",
        )
        .name("Environment")
        .input_type(InputType::Text),
        ConfigVar::new(
            "SOLOBASE_SHARED__HAS_DISPATCHER_BINDING",
            "Whether this project has a dispatcher service binding",
            "false",
        )
        .name("Dispatcher Binding")
        .input_type(InputType::Toggle),
    ]
}

/// Collect all known config variables: shared + all block-declared.
pub fn collect_all_config_vars(block_infos: &[wafer_run::block::BlockInfo]) -> Vec<ConfigVar> {
    let mut all = shared_config_vars();
    for info in block_infos {
        all.extend(info.config_keys.iter().cloned());
    }
    all
}
