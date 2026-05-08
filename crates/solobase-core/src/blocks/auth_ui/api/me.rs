//! GET / UPDATE /b/auth/api/me — relocated from auth/login.rs in Task 5.

use wafer_run::{context::Context, types::Message, InputStream, OutputStream};

pub async fn handle_get(_ctx: &dyn Context, _msg: &Message) -> OutputStream {
    unimplemented!("relocated in Task 5")
}

pub async fn handle_update(
    _ctx: &dyn Context,
    _msg: &Message,
    _input: InputStream,
) -> OutputStream {
    unimplemented!("relocated in Task 5")
}
