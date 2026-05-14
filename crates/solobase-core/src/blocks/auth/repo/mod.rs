//! Row-level data access for the `suppers-ai/auth` block.
//!
//! Each submodule exposes pure-function async helpers that take
//! `&dyn wafer_run::context::Context` and operate on a single table defined
//! by migration 001. Errors collapse into the [`RepoError`] enum so higher
//! layers don't have to reason about the underlying `db` client's error type.

pub mod api_keys;
pub mod bootstrap_tokens;
pub mod cli_codes;
pub mod jwt_blocklist;
pub mod local_credentials;
pub mod orgs;
pub mod pats;
pub mod provider_links;
pub mod rate_limits;
pub mod sessions;
pub mod tokens;
pub mod users;

/// Errors surfaced by the auth repo layer.
#[derive(thiserror::Error, Debug)]
pub enum RepoError {
    /// The requested row was not visible after an insert/update — treat as a
    /// programmer/db-consistency error.
    #[error("not found")]
    NotFound,
    /// Low-level database error (client error, bad row shape, JSON decode, …).
    #[error("db: {0}")]
    Db(String),
}
