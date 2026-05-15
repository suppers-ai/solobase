//! JSON API handlers for the auth-ui block. One handler per leaf module;
//! routed from `auth_ui::AuthUiBlock::handle`.

pub mod api_keys;
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
