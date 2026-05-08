//! GET /b/auth/dashboard — relocated from auth/pages/dashboard.rs in Task 5.

use wafer_run::{context::Context, types::Message, OutputStream};

pub async fn handle(_ctx: &dyn Context, _msg: &Message) -> OutputStream {
    unimplemented!("relocated in Task 5")
}
