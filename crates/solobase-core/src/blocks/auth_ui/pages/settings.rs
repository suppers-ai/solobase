//! GET / POST /b/auth/admin/settings — relocated from auth/pages/mod.rs in Task 5.

use wafer_run::{context::Context, types::Message, InputStream, OutputStream};

pub async fn handle_get(_ctx: &dyn Context, _msg: &Message) -> OutputStream {
    unimplemented!("relocated in Task 5")
}

pub async fn handle_post(_ctx: &dyn Context, _input: InputStream) -> OutputStream {
    unimplemented!("relocated in Task 5")
}
