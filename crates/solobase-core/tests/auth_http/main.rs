//! Layer-3 integration test crate for the `suppers-ai/auth` block.
//!
//! Spins up a real axum HTTP server on an ephemeral port, translates each
//! incoming request into a `wafer_run::Message`, dispatches to
//! `SolobaseAuthBlock::handle`, and maps the output back to a standard HTTP
//! response. Tests drive the server over TCP with `reqwest`, so cookie
//! parsing, header casing, status codes, JSON bodies, and redirect
//! behaviour are all exercised end-to-end.

mod common;
#[path = "../auth/fake_github.rs"]
mod fake_github;
mod login;
mod oauth;
