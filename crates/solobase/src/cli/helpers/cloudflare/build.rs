//! Cross-compile the consumer crate via `worker-build`.

use std::{path::Path, process::Command};

use anyhow::{bail, Context, Result};

/// Install `worker-build` pinned to `^0.7`. We always run this — `cargo
/// install` is a no-op when the requested version is already present, and
/// will downgrade if a newer (incompatible) version was installed.
///
/// Pin reason: worker-build 0.8.x rejects `worker < 0.8` and changed its
/// output layout. Matches the version pin in `wrangler::base_toml`.
pub fn ensure_worker_build_installed() -> Result<()> {
    let install = Command::new("cargo")
        .args(["install", "worker-build", "--version", "^0.7", "--quiet"])
        .status()
        .context("run `cargo install worker-build --version ^0.7`")?;
    if !install.success() {
        bail!(
            "cargo install worker-build --version ^0.7 failed (exit {:?})",
            install.code()
        );
    }
    Ok(())
}

pub fn run(repo_root: &Path, release: bool) -> Result<()> {
    ensure_worker_build_installed()?;

    let mut cmd = Command::new("worker-build");
    cmd.current_dir(repo_root)
        .args(["--no-default-features", "--features", "target-cloudflare"]);
    if release {
        cmd.arg("--release");
    }
    let status = cmd.status().context("run worker-build")?;
    if !status.success() {
        bail!("worker-build failed (exit {:?})", status.code());
    }
    Ok(())
}
