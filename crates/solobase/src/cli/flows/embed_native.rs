//! Stub for embed × native flow. Implemented in Task 12.

use std::path::Path;

use anyhow::Result;

pub async fn build(_repo_root: &Path, _release: bool) -> Result<()> {
    anyhow::bail!("embed × native build: not implemented (Task 12)")
}

pub async fn serve(_repo_root: &Path, _release: bool, _port: Option<u16>) -> Result<()> {
    anyhow::bail!("embed × native serve: not implemented (Task 12)")
}
