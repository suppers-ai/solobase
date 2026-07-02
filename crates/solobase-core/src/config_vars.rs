//! Central config variable definitions.
//!
//! Shared (`SOLOBASE_SHARED__`) variables are defined here — the single source
//! of truth. Block-scoped variables are declared in each block's `BlockInfo`.
//!
//! Use `collect_all_config_vars()` to get the complete set of all known config
//! variables (shared + block-declared) for seeding, validation, and UI rendering.

use wafer_run::{ConfigVar, InputType};

/// Worker-secret name for the deploy-time `/_deploy/init` bearer token.
///
/// One canonical name shared by both sides of the deploy handshake: the CLI
/// (`solobase deploy` / `solobase deploy secret`) reads it from the
/// same-named env var and provisions it via `wrangler secret put`, and the
/// Cloudflare worker reads it via `env.secret(DEPLOY_TOKEN_KEY)` to gate the
/// endpoint. Not a `ConfigVar` (never lives in D1 or the admin UI) — it is a
/// deploy-time worker secret, so it is a plain const rather than a
/// `SOLOBASE_SHARED__*` entry.
pub const DEPLOY_TOKEN_KEY: &str = "SOLOBASE_DEPLOY_TOKEN";

/// Shared config variables readable by all blocks, writable only by admin.
///
/// These are NOT owned by any block — they're platform-level settings.
/// Blocks should NOT declare `SOLOBASE_SHARED__` vars in their `config_keys`.
pub fn shared_config_vars() -> Vec<ConfigVar> {
    let mut vars = vec![
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
            "SOLOBASE_SHARED__SITE_URL",
            "Marketing site URL for docs and pricing links",
            "https://solobase.dev",
        )
        .name("Site URL")
        .input_type(InputType::Url),
        ConfigVar::new(
            "SOLOBASE_SHARED__LOGO_URL",
            "Logo shown in header and emails",
            crate::ui::assets::logo_long_url(),
        )
        .name("Logo URL")
        .input_type(InputType::Url),
        ConfigVar::new(
            "SOLOBASE_SHARED__LOGO_ICON_URL",
            "Small icon logo (used when sidebar is collapsed)",
            crate::ui::assets::logo_icon_url(),
        )
        .name("Logo Icon URL")
        .input_type(InputType::Url),
        ConfigVar::new(
            "SOLOBASE_SHARED__AUTH_LOGO_URL",
            "Logo on login/signup pages (falls back to Logo URL)",
            "",
        )
        .name("Auth Logo URL")
        .input_type(InputType::Url),
        ConfigVar::new(
            "SOLOBASE_SHARED__FAVICON_URL",
            "Browser tab icon",
            crate::ui::assets::favicon_url(),
        )
        .name("Favicon URL")
        .input_type(InputType::Url),
        ConfigVar::new(
            "SOLOBASE_SHARED__ALLOW_USER_PRODUCTS",
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
        ConfigVar::new(
            "SOLOBASE_SHARED__HAS_LANDING_PAGE",
            "Serve a static landing page (wafer-run/web) at `/` instead of \
             redirecting anonymous visitors to the login page",
            "false",
        )
        .name("Has Landing Page")
        .input_type(InputType::Toggle),
        ConfigVar::new(
            "SOLOBASE_SHARED__EMBEDDED_SCRIPTS",
            "Comma-separated module-script URLs injected into every SSR page \
             (e.g. /webllm-engine.js for browser WebLLM). Native deployments \
             leave this empty.",
            "",
        )
        .name("Embedded Scripts")
        .input_type(InputType::Text),
    ];
    // Auth-scoped shared vars (suppers-ai/auth reads these; admin writes them).
    // Declared here rather than in the auth block's BlockInfo::config_keys because
    // SOLOBASE_SHARED__* vars must not be claimed by any single block.
    vars.extend(crate::blocks::auth::config::auth_config_vars());
    vars
}

/// Look up a single `SOLOBASE_SHARED__*` config var by key.
///
/// The settings pages assemble their sections by pulling the exact
/// [`ConfigVar`] metadata they want to show — shared vars come from here,
/// block-owned vars come from the block's own `info().config_keys` (via
/// [`var_in`]). This keeps [`ConfigVar`] the single source of truth: no page
/// re-declares a key's label/default/input_type in a parallel tuple table.
///
/// Panics in debug if the key isn't a known shared var — that's a programming
/// error (a settings page asking for a var that was never declared), caught at
/// the first test run rather than silently rendering an empty field.
pub fn shared_var(key: &str) -> ConfigVar {
    shared_config_vars()
        .into_iter()
        .find(|v| v.key == key)
        .unwrap_or_else(|| {
            debug_assert!(false, "settings page requested unknown shared var: {key}");
            ConfigVar::new(key, "", "")
        })
}

/// Look up a single config var by key within a block's own declared
/// `config_keys`. The companion to [`shared_var`] for block-owned vars.
///
/// Panics in debug if the key isn't declared by the block.
pub fn var_in(vars: &[ConfigVar], key: &str) -> ConfigVar {
    vars.iter()
        .find(|v| v.key == key)
        .cloned()
        .unwrap_or_else(|| {
            debug_assert!(false, "settings page requested undeclared block var: {key}");
            ConfigVar::new(key, "", "")
        })
}

/// Collect all known config variables: shared + all block-declared.
pub fn collect_all_config_vars(block_infos: &[wafer_run::BlockInfo]) -> Vec<ConfigVar> {
    let mut all = shared_config_vars();
    for info in block_infos {
        all.extend(info.config_keys.iter().cloned());
    }
    all
}

/// Derive the SCREAMING_SNAKE block prefix written to the
/// `suppers_ai__admin__variables.block` column from a `{org}/{block}` name.
///
/// This is the single source of truth for the `block` column value: the
/// boot-time auto-generated-secret seeder ([`crate::boot::seed_auto_generated`])
/// writes it, the `D1ConfigSource` queries by it, and admin migration 002
/// backfills the same shape from the `key` column's first two `__`-delimited
/// segments. All three must agree, so they all funnel through here.
///
/// Conversion rules:
/// - `-` → `_` (within each segment)
/// - `/` → `__` (segment separator)
/// - uppercase
///
/// Examples:
/// - `"suppers-ai/auth"` → `"SUPPERS_AI__AUTH"`
/// - `"wafer-run/sqlite"` → `"WAFER_RUN__SQLITE"`
/// - `"suppers-ai"` (org only) → `"SUPPERS_AI"`
pub fn screaming_block(name: &str) -> String {
    let (org, block) = name.split_once('/').unwrap_or((name, ""));
    let org_upper = org.replace('-', "_").to_uppercase();
    if block.is_empty() {
        org_upper
    } else {
        let block_upper = block.replace('-', "_").to_uppercase();
        format!("{org_upper}__{block_upper}")
    }
}

/// Derive the `variables.block` column value from a *config key* (rather than
/// a block name), matching the SQL backfill in admin migration 002.
///
/// The block prefix is the key's first two `__`-delimited segments — e.g.
/// `SUPPERS_AI__AUTH__JWT_SECRET` → `SUPPERS_AI__AUTH`. A key with fewer than
/// two `__` separators (a shared `SOLOBASE_SHARED__*` var, or any legacy
/// single-segment key) has no block and returns `""`. The empty string is the
/// in-memory stand-in for the migration's `NULL`: the boot seeder omits the
/// `block` column entirely when this is empty, leaving the row's `block` NULL,
/// exactly as the backfill would.
///
/// This MUST stay byte-for-byte equivalent to migration 002's `CASE` so a
/// row seeded by [`crate::boot`] and a row backfilled by the migration land on
/// the same `block` value (and therefore the same `D1ConfigSource` per-block
/// cache key).
pub fn key_block_prefix(key: &str) -> String {
    let Some(first) = key.find("__") else {
        return String::new();
    };
    // Look for a second `__` after the first separator.
    match key[first + 2..].find("__") {
        Some(rel) => key[..first + 2 + rel].to_string(),
        None => String::new(),
    }
}

#[cfg(test)]
mod screaming_block_tests {
    use super::{key_block_prefix, screaming_block};

    #[test]
    fn two_segment_name() {
        assert_eq!(screaming_block("suppers-ai/auth"), "SUPPERS_AI__AUTH");
        assert_eq!(screaming_block("wafer-run/sqlite"), "WAFER_RUN__SQLITE");
    }

    #[test]
    fn org_only_name() {
        assert_eq!(screaming_block("suppers-ai"), "SUPPERS_AI");
    }

    #[test]
    fn key_block_prefix_two_segments() {
        // Block-scoped key → first two `__`-segments, matching migration 002.
        assert_eq!(
            key_block_prefix("SUPPERS_AI__AUTH__JWT_SECRET"),
            "SUPPERS_AI__AUTH"
        );
        assert_eq!(
            key_block_prefix("SUPPERS_AI__PRODUCTS__WEBHOOK_SECRET"),
            "SUPPERS_AI__PRODUCTS"
        );
    }

    #[test]
    fn key_block_prefix_shared_and_legacy_are_null() {
        // One `__` (shared var) → NULL/empty.
        assert_eq!(key_block_prefix("SOLOBASE_SHARED__ALLOW_SIGNUP"), "");
        // No `__` → NULL/empty.
        assert_eq!(key_block_prefix("LEGACY_KEY"), "");
    }

    #[test]
    fn key_block_prefix_matches_screaming_block_for_owned_keys() {
        // A block's auto-gen key prefix derived from the key must equal the
        // prefix derived from the block name, so the seeder and the migration
        // backfill agree.
        assert_eq!(
            key_block_prefix("SUPPERS_AI__AUTH__JWT_SECRET"),
            screaming_block("suppers-ai/auth")
        );
    }
}
