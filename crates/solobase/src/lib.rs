//! solobase — unified CLI binary.
//!
//! This crate's primary output is the `solobase` binary; the lib exists
//! to expose the `cli` module to integration tests in `tests/`.

pub mod cli;

/// Precompiled solobase-web wasm, baked at build time. The CLI's sealed
/// × web flow uses this as the default when `SOLOBASE_WEB_WASM` is unset.
pub static SOLOBASE_WEB_WASM: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/solobase-web.wasm"));

/// Precompiled solobase-web JS glue, baked at build time. The CLI's sealed
/// × web flow uses this as the default when `SOLOBASE_WEB_JS` is unset.
pub static SOLOBASE_WEB_JS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/solobase-web.js"));
