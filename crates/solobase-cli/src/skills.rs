use std::path::{Path, PathBuf};
use std::process::Command;

/// Return each absolute path to a skill-block crate under `<repo_root>/blocks/`.
/// A skill block is a directory directly under `blocks/` that contains a
/// `Cargo.toml`. Non-directories and directories without `Cargo.toml` are
/// skipped silently. Missing `blocks/` dir returns an empty vec.
pub fn discover(repo_root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let blocks_dir = repo_root.join("blocks");
    if !blocks_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&blocks_dir)
        .map_err(|e| anyhow::anyhow!("read {blocks_dir:?}: {e}"))?
    {
        let entry = entry.map_err(|e| anyhow::anyhow!("read_dir entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() && path.join("Cargo.toml").is_file() {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// For each discovered skill-block dir, run `wafer build` in that directory.
/// Fails fast on the first error with a `skill build failed: blocks/<name>`
/// step prefix.
pub fn build_all(repo_root: &Path) -> anyhow::Result<()> {
    let blocks = discover(repo_root)?;
    for block_dir in blocks {
        let short = block_dir
            .strip_prefix(repo_root)
            .unwrap_or(&block_dir)
            .display();
        let step = format!("skill build failed: {short}");
        let mut cmd = Command::new("wafer");
        cmd.arg("build").current_dir(&block_dir);
        crate::cmd::run(&step, cmd)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn no_blocks_dir_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let found = discover(tmp.path()).unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn finds_blocks_with_cargo_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("blocks/alpha")).unwrap();
        fs::write(
            root.join("blocks/alpha/Cargo.toml"),
            "[package]\nname = \"alpha\"\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("blocks/beta")).unwrap();
        fs::write(
            root.join("blocks/beta/Cargo.toml"),
            "[package]\nname = \"beta\"\n",
        )
        .unwrap();

        // Directory with no Cargo.toml is skipped.
        fs::create_dir_all(root.join("blocks/empty")).unwrap();

        let found = discover(root).unwrap();
        assert_eq!(found.len(), 2);
        assert!(found[0].ends_with("blocks/alpha"));
        assert!(found[1].ends_with("blocks/beta"));
    }
}
