//! Re-exports of the asset-overlay logic from legacy_build for use by the
//! new flows. The function is unchanged; only the name is.

use std::path::Path;

use anyhow::{anyhow, Result};

use crate::cli::legacy_config::Config;

pub fn apply_overlays(cfg: &Config, repo_root: &Path, dist_dir: &Path) -> Result<()> {
    for overlay in &cfg.assets.overlay {
        let src = repo_root.join(&overlay.from);
        let dst = dist_dir.join(&overlay.to);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow!("create dir {parent:?}: {e}"))?;
        }
        std::fs::copy(&src, &dst)
            .map_err(|e| anyhow!("overlay {src:?} → {dst:?}: {e}"))?;
    }
    Ok(())
}

pub use crate::cli::legacy_config::OverlayEntry as Overlay;
