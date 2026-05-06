//! Embed × Cloudflare flow: cross-compile a consumer crate to wasm32,
//! generate wrangler.toml + stage assets, optionally deploy via wrangler.
//!
//! Real bodies land in subsequent tasks.

use std::path::Path;

use anyhow::{bail, Result};

pub async fn build(_repo_root: &Path, _release: bool) -> Result<()> {
    bail!("embed_cloudflare::build not yet implemented")
}

pub async fn serve(_repo_root: &Path, _release: bool, _port: Option<u16>) -> Result<()> {
    bail!("embed_cloudflare::serve not yet implemented")
}

pub async fn deploy(_repo_root: &Path, _release: bool) -> Result<()> {
    bail!("embed_cloudflare::deploy not yet implemented")
}
