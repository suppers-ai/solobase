//! GET /b/auth/login — relocated from auth/pages/mod.rs::login_page in Task 5.

use wafer_run::{context::Context, types::Message, OutputStream};

pub async fn handle(_ctx: &dyn Context, _msg: &Message) -> OutputStream {
    unimplemented!("relocated in Task 5")
}
