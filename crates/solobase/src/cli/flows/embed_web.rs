//! Stub for embed × web flow. Implemented in Task 11.

use std::path::Path;

use anyhow::Result;

pub async fn build(_repo_root: &Path, _release: bool) -> Result<()> {
    anyhow::bail!("embed × web build: not implemented (Task 11)")
}

pub async fn serve(_repo_root: &Path, _release: bool, _port: Option<u16>) -> Result<()> {
    anyhow::bail!("embed × web serve: not implemented (Task 11)")
}
