//! Thin re-export shim over `cli::helpers::blocks` for legacy callers.
//! `legacy_build` still imports `crate::cli::skills::build_all`. Task 13
//! deletes this file once the legacy modules are gone.

pub use crate::cli::helpers::blocks::{build_all, discover_blocks as discover};
