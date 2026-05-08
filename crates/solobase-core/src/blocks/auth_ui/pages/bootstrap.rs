//! GET /b/auth/bootstrap — bootstrap admin token redemption form.
//!
//! Implementation lands in Task 6 of Plan A2 PR 5. Until then this
//! handler panics — the dispatch entry exists so the route table is
//! complete from the moment auth-ui is registered.

use wafer_run::{context::Context, types::Message, OutputStream};

pub async fn handle_get(_ctx: &dyn Context, _msg: &Message) -> OutputStream {
    unimplemented!("filled in Task 6")
}
