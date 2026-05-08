//! POST /b/auth/api/bootstrap — bootstrap admin token redemption.
//!
//! Implementation lands in Task 6 of Plan A2 PR 5. Until then this
//! handler panics — the dispatch entry exists so the route table is
//! complete from the moment auth-ui is registered.

use wafer_run::{context::Context, InputStream, OutputStream};

pub async fn handle(_ctx: &dyn Context, _input: InputStream) -> OutputStream {
    unimplemented!("filled in Task 6")
}
