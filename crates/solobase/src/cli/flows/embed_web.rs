//! Stub for embed × web flow. Implemented in Phase 5.

use anyhow::Result;

pub async fn build(_release: bool) -> Result<()> {
    anyhow::bail!("embed × web build: not implemented (Phase 5)")
}

pub async fn serve(_release: bool, _port: Option<u16>) -> Result<()> {
    anyhow::bail!("embed × web serve: not implemented (Phase 5)")
}
