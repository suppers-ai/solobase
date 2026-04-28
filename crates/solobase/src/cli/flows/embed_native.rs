//! Stub for embed × native flow. Implemented in Phase 5.

use anyhow::Result;

pub async fn build(_release: bool) -> Result<()> {
    anyhow::bail!("embed × native build: not implemented (Phase 5)")
}

pub async fn serve(_release: bool, _port: Option<u16>) -> Result<()> {
    anyhow::bail!("embed × native serve: not implemented (Phase 5)")
}
