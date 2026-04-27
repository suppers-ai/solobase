//! Legacy CLI scaffolding from the absorbed solobase-cli crate.
//!
//! These modules are reachable as a library via `solobase::cli::legacy_*`
//! and are exercised by the in-process tests in `tests/cli/`. Until Task 7
//! introduces the unified verb-based CLI, the only binary entrypoint into
//! these flows is the temporary `solobase legacy-build` subcommand (used
//! by CI). Task 7 replaces these incrementally; Task 13 deletes the
//! `legacy_*` prefix entirely.
pub mod cmd;
pub mod legacy_build;
pub mod legacy_config;
pub mod legacy_serve;
pub mod skills;
