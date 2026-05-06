//! Cross-compile the consumer crate via `worker-build`.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

pub fn ensure_worker_build_installed() -> Result<()> {
    let status = Command::new("which")
        .arg("worker-build")
        .status()
        .context("run `which worker-build`")?;
    if status.success() {
        return Ok(());
    }
    eprintln!("installing worker-build (one-time)...");
    let install = Command::new("cargo")
        .args(["install", "worker-build", "--quiet"])
        .status()
        .context("run `cargo install worker-build`")?;
    if !install.success() {
        bail!("cargo install worker-build failed (exit {:?})", install.code());
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
