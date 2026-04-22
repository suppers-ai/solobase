//! Plan D maud page templates for the `suppers-ai/auth` block.
//!
//! Each submodule takes a view-model from
//! [`crate::blocks::auth::view_models`] and returns a `Markup` — no I/O,
//! no async, no access to handlers. This keeps templates unit-testable
//! from a stock vm.

pub mod base;
