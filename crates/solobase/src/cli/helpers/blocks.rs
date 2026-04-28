use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

/// Returns each absolute path to a block crate under `<repo_root>/blocks/`.
/// A block dir is a direct child of `blocks/` containing a `Cargo.toml`.
/// Missing `blocks/` dir returns an empty vec.
pub fn discover_blocks(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let blocks_dir = repo_root.join("blocks");
    if !blocks_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&blocks_dir)
        .map_err(|e| anyhow!("read {blocks_dir:?}: {e}"))?
    {
        let entry = entry.map_err(|e| anyhow!("read_dir entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() && path.join("Cargo.toml").is_file() {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// Run `wafer build` in each discovered block dir. Used by all four flows.
pub fn build_all(repo_root: &Path) -> Result<()> {
    use std::process::Command;
    for block in discover_blocks(repo_root)? {
        let short = block.strip_prefix(repo_root).unwrap_or(&block).display();
        let step = format!("wafer build {short}");
        let mut cmd = Command::new("wafer");
        cmd.arg("build").current_dir(&block);
        crate::cli::cmd::run(&step, cmd)?;
    }
    Ok(())
}
