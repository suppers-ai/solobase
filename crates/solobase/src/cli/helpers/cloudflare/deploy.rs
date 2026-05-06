//! Deploy subprocess orchestration: `wrangler deploy`, R2 asset upload,
//! D1 migrations. All commands inherit stdout/stderr; errors propagate
//! verbatim.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use super::assets::mime_for_path;

pub fn wrangler_deploy(wrangler_toml: &Path) -> Result<()> {
    let status = Command::new("wrangler")
        .args(["deploy", "--config"])
        .arg(wrangler_toml)
        .status()
        .context("run wrangler deploy")?;
    if !status.success() {
        bail!("wrangler deploy failed (exit {:?})", status.code());
    }
    Ok(())
}

pub fn r2_upload_dir(bucket: &str, assets_root: &Path) -> Result<usize> {
    if !assets_root.is_dir() {
        return Ok(0);
    }
    let mut uploaded = 0;
    walk_files(assets_root, &mut |abs| {
        let rel = abs.strip_prefix(assets_root).unwrap_or(abs);
        let key = rel.to_string_lossy().replace('\\', "/");
        let mime = mime_for_path(abs);
        let status = Command::new("wrangler")
            .args([
                "r2",
                "object",
                "put",
                &format!("{bucket}/{key}"),
                "--file",
            ])
            .arg(abs)
            .args(["--content-type", mime, "--remote"])
            .status()
            .context("run wrangler r2 object put")?;
        if !status.success() {
            bail!("upload {} failed (exit {:?})", key, status.code());
        }
        uploaded += 1;
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(uploaded)
}

pub fn d1_migrate_remote(database_name: &str, wrangler_toml: &Path) -> Result<()> {
    let status = Command::new("wrangler")
        .args(["d1", "migrations", "apply", database_name, "--remote", "--config"])
        .arg(wrangler_toml)
        .status()
        .context("run wrangler d1 migrations apply --remote")?;
    if !status.success() {
        bail!("wrangler d1 migrations apply --remote failed");
    }
    Ok(())
}

fn walk_files<F>(root: &Path, f: &mut F) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    for entry in std::fs::read_dir(root).with_context(|| format!("read {}", root.display()))? {
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            walk_files(&path, f)?;
        } else if ft.is_file() {
            f(&path)?;
        }
    }
    Ok(())
}
