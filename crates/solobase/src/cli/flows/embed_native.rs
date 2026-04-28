//! Embed × native: cargo build the user's bin crate, then exec it.

use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Result};

use crate::cli::cmd;
use crate::cli::helpers::{blocks, frontend};
use crate::cli::legacy_config;

const RUNTIME_SITE_REL: &str = "data/storage/wafer-run/web/site";

pub async fn build(repo_root: &Path, release: bool) -> Result<()> {
    // 1. wafer build per block.
    blocks::build_all(repo_root)?;

    // 2. Frontend copy (if any).
    if let Some(fe) = frontend::find_frontend_dir(repo_root) {
        let dst = repo_root.join(RUNTIME_SITE_REL);
        frontend::copy_tree(&fe, &dst).map_err(|e| anyhow!("copy frontend: {e}"))?;
    }

    // 3. cargo build the user's bin crate.
    let mut cargo = Command::new("cargo");
    cargo.arg("build");
    if release {
        cargo.arg("--release");
    }
    if let Ok((cfg, _)) = legacy_config::find_and_load(repo_root) {
        if let Some(mp) = &cfg.solobase.manifest_path {
            cargo.arg("--manifest-path").arg(mp);
        }
    }
    cargo.current_dir(repo_root);
    cmd::run("cargo build", cargo)?;

    let profile = if release { "release" } else { "debug" };
    println!("built embed × native ({profile})");
    Ok(())
}

pub async fn serve(repo_root: &Path, release: bool, _port: Option<u16>) -> Result<()> {
    build(repo_root, release).await?;

    // Locate target/<profile>/<bin-name>. Read Cargo.toml to find the bin.
    let cargo_toml = repo_root.join("Cargo.toml");
    let text = std::fs::read_to_string(&cargo_toml)?;
    let parsed: toml::Value = toml::from_str(&text)?;
    let bin_name = parsed
        .get("bin")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|b| b.get("name"))
        .and_then(|n| n.as_str())
        .map(String::from)
        .or_else(|| {
            parsed
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .map(String::from)
        })
        .ok_or_else(|| anyhow!("could not determine bin name from Cargo.toml"))?;

    let profile = if release { "release" } else { "debug" };
    let bin = repo_root.join("target").join(profile).join(&bin_name);
    if !bin.is_file() {
        return Err(anyhow!("expected binary at {bin:?} after cargo build"));
    }
    let mut child = Command::new(&bin).current_dir(repo_root).spawn()?;
    let status = child.wait()?;
    std::process::exit(status.code().unwrap_or(1));
}
