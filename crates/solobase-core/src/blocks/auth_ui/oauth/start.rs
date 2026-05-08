//! GET /b/auth/oauth/login — relocated from auth/oauth.rs::handle_oauth_login in Task 5.

use wafer_run::{context::Context, types::Message, OutputStream};

pub async fn handle(_ctx: &dyn Context, _msg: &Message) -> OutputStream {
    unimplemented!("relocated in Task 5")
}
