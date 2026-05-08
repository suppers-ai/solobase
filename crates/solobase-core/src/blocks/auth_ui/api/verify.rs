//! GET/POST /b/auth/api/verify and POST /b/auth/api/resend-verification —
//! relocated from auth/login.rs in Task 5.

use wafer_run::{context::Context, types::Message, InputStream, OutputStream};

pub async fn handle(_ctx: &dyn Context, _msg: &Message, _input: InputStream) -> OutputStream {
    unimplemented!("relocated in Task 5")
}

pub async fn handle_resend(_ctx: &dyn Context, _input: InputStream) -> OutputStream {
    unimplemented!("relocated in Task 5")
}
