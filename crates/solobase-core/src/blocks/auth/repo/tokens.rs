//! Table declaration for `suppers_ai__auth__tokens`.
//!
//! Stores refresh tokens, password-reset tokens, and email-verification
//! tokens. Row-level helpers live alongside the callers in
//! `auth/mod.rs` and `auth_ui/api/{refresh,logout,reset_password,…}`.

pub(crate) const TABLE: &str = "suppers_ai__auth__tokens";
