//! Resolve the [cloudflare] section from solobase.toml + env vars.

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

use super::wrangler::CloudflareConfig;

#[derive(Debug, Deserialize)]
struct SolobaseTomlPartial {
    cloudflare: Option<CloudflareConfig>,
}

/// Load `[cloudflare]` from `<repo_root>/solobase.toml`. Errors with a
/// clear message if the file or section is missing.
pub fn load(repo_root: &Path) -> Result<CloudflareConfig> {
    let path = repo_root.join("solobase.toml");
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    let parsed: SolobaseTomlPartial = toml::from_str(&raw)
        .with_context(|| format!("parse {}", path.display()))?;
    parsed.cloudflare.ok_or_else(|| {
        anyhow!(
            "{} is missing a [cloudflare] section — see \
             docs/superpowers/specs/2026-05-06-solobase-cloudflare-cli-flow-design.md",
            path.display()
        )
    })
}

/// Validate that environment variables required for `deploy` are set.
/// Returns the (account_id, api_token) pair on success.
pub fn require_deploy_env(cfg: &CloudflareConfig) -> Result<(String, String)> {
    let account_id = cfg
        .account_id
        .clone()
        .or_else(|| std::env::var("CLOUDFLARE_ACCOUNT_ID").ok())
        .ok_or_else(|| {
            anyhow!(
                "missing account_id: set [cloudflare.account_id] in solobase.toml \
                 or env CLOUDFLARE_ACCOUNT_ID"
            )
        })?;
    let api_token = std::env::var("CLOUDFLARE_API_TOKEN").map_err(|_| {
        anyhow!(
            "missing env CLOUDFLARE_API_TOKEN — required for `solobase deploy --target cloudflare`"
        )
    })?;
    Ok((account_id, api_token))
}
