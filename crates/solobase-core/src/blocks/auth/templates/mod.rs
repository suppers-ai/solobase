//! Plan D maud page templates for the `suppers-ai/auth` block.
//!
//! Each submodule takes a view-model from
//! [`crate::blocks::auth::view_models`] and returns a `Markup` — no I/O,
//! no async, no access to handlers. This keeps templates unit-testable
//! from a stock vm.
//!
//! Testing strategy:
//!   - **Unit:** each template's `#[cfg(test)] mod tests` renders a fixed
//!     view-model and asserts on key markers in the rendered string.
//!   - **Layer 3 (HTTP):** `tests/auth_http/pages_http.rs` drives the real
//!     routes via `reqwest` and asserts on full responses.
//!
//! **Layer 4 (Playwright) is deferred per spec §7.** Browser-level tests
//! will be added in the consumer that mounts these pages (future registry
//! spec §9 or a standalone solobase UI test suite) — not in-repo, because
//! the auth block has no default host to exercise end-to-end here.

pub mod base;
pub mod cli_code_fragment;
pub mod cli_login;
pub mod dashboard;
pub mod login;
pub mod orgs_detail;
pub mod signup;
