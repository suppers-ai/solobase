//! POST /b/auth/api/oauth/sync-user — relocated from auth/oauth.rs in Task 5.

use wafer_run::{context::Context, types::Message, InputStream, OutputStream};

pub async fn handle(_ctx: &dyn Context, _msg: &Message, _input: InputStream) -> OutputStream {
    unimplemented!("relocated in Task 5")
}
