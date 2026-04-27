//! Legacy CLI scaffolding from the absorbed solobase-cli crate.
//!
//! The real verb-based CLI replaces these in Task 7 of the unified-CLI
//! plan. Until then these modules exist behind a `legacy_*` prefix so
//! today's `solobase build` / `dev` / `serve` (browser-only) command
//! handlers stay available as a regression baseline.
pub mod cmd;
pub mod legacy_build;
pub mod legacy_config;
pub mod legacy_serve;
pub mod skills;
