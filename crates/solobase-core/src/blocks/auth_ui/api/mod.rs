//! JSON API handlers for the auth-ui block.
//!
//! Each leaf module hosts one handler relocated from the legacy
//! `auth/` block in Task 5 of Plan A2 PR 5. Until then, every
//! function panics with `unimplemented!()`.

pub mod bootstrap;
pub mod change_password;
pub mod forgot_password;
pub mod login;
pub mod logout;
pub mod me;
pub mod refresh;
pub mod reset_password;
pub mod signup;
pub mod sync_user;
pub mod verify;
