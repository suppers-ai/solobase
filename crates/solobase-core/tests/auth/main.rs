//! Integration test crate for the `suppers-ai/auth` block.

mod block_dispatch;
mod common;
mod migrations_001;
mod migrations_002;
mod repo_pats;
mod repo_sessions;
mod repo_users;
mod service_require_role;
mod service_require_token;
mod service_require_user;
mod service_user_profile;
mod session_issue;
