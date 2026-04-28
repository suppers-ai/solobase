//! Unified CLI scaffolding.
//!
//! `cli_args` + `mode` define the parser and mode/target detection.
//! `flows` contains the four (mode × target) handlers (stubbed in Task 7,
//! implemented in Phase 5). `server` + `server_config` carry the legacy
//! native server-boot body, invoked today by the sealed × native flow.
//! `legacy_*` + `cmd` + `skills` are the absorbed solobase-cli scaffolding,
//! kept for the in-process tests under `tests/cli/` until Task 13 deletes
//! them.
pub mod cli_args;
pub mod cmd;
pub mod flows;
pub mod legacy_build;
pub mod legacy_config;
pub mod legacy_serve;
pub mod mode;
pub mod server;
pub mod server_config;
pub mod skills;
