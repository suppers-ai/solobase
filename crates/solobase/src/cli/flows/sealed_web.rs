//! Stub for sealed × web flow. Implemented in Task 10.

use std::path::Path;

use anyhow::Result;

pub async fn build(_repo_root: &Path, _release: bool) -> Result<()> {
    anyhow::bail!("sealed × web build: not implemented (Task 10)")
}

pub async fn serve(_repo_root: &Path, _release: bool, _port: Option<u16>) -> Result<()> {
    anyhow::bail!("sealed × web serve: not implemented (Task 10)")
}
