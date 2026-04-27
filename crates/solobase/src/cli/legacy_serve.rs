use std::{path::PathBuf, process::Command};

use crate::cli::{legacy_build::BuildProfile, legacy_config::Config};

pub fn run(cfg: &Config, repo_root: &PathBuf, port: u16) -> anyhow::Result<()> {
    // Dev build first.
    crate::cli::legacy_build::run(cfg, repo_root, BuildProfile::Dev)?;

    let dist_dir = repo_root.join(&cfg.wasm.out_dir);
    println!(
        "serving {} on http://localhost:{}",
        dist_dir.display(),
        port
    );

    let mut cmd = Command::new("python3");
    cmd.arg("-m")
        .arg("http.server")
        .arg(port.to_string())
        .arg("-d")
        .arg(dist_dir);
    // http.server only exits on signal — this call blocks until the user
    // hits Ctrl+C, which exit-codes 130. Treat that as success.
    match cmd.status() {
        Ok(status) if status.success() || status.code() == Some(130) => Ok(()),
        Ok(status) => Err(anyhow::anyhow!(
            "python3 -m http.server exited {:?}",
            status.code()
        )),
        Err(e) => Err(anyhow::anyhow!("spawn python3: {e}")),
    }
}
