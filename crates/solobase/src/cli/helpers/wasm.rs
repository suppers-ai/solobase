//! Resolves the solobase-web wasm bytes for the sealed × web flow.

use std::{borrow::Cow, path::PathBuf};

use anyhow::{anyhow, Result};

/// Resolution order:
/// 1. SOLOBASE_WEB_WASM env var (must point at an existing file)
/// 2. include_bytes! baked at build time (always available)
pub fn resolve_solobase_web_wasm() -> Result<Cow<'static, [u8]>> {
    if let Ok(p) = std::env::var("SOLOBASE_WEB_WASM") {
        let path = PathBuf::from(&p);
        if !path.is_file() {
            return Err(anyhow!(
                "SOLOBASE_WEB_WASM points at {p:?} but the file does not exist"
            ));
        }
        let bytes = std::fs::read(&path).map_err(|e| anyhow!("read {p:?}: {e}"))?;
        return Ok(Cow::Owned(bytes));
    }
    Ok(Cow::Borrowed(crate::SOLOBASE_WEB_WASM))
}
