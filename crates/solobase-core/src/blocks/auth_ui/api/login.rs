//! POST /b/auth/api/login — relocated from auth/login.rs in Task 5.

use wafer_run::{context::Context, InputStream, OutputStream};

pub async fn handle(_ctx: &dyn Context, _input: InputStream) -> OutputStream {
    unimplemented!("relocated in Task 5")
}
