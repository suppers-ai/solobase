//! POST /b/auth/api/logout — relocated from auth/login.rs in Task 5.

use wafer_run::{context::Context, types::Message, OutputStream};

use crate::blocks::{
    auth::{helpers::build_auth_cookie, repo::tokens},
    helpers::ResponseBuilder,
};

pub async fn handle(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let user_id = msg.user_id();
    if !user_id.is_empty() {
        // SEC-032/039: revoke (don't delete) the user's refresh-token rows
        // so the tombstones remain available for reuse detection. The
        // browser drops its cookie below either way; this just invalidates
        // any in-flight refresh attempts on other clients.
        tokens::revoke_all_for_user(ctx, user_id).await.ok();
    }

    let cookie = build_auth_cookie("", 0, ctx).await;
    ResponseBuilder::new()
        .set_cookie(&cookie)
        .status(303)
        .set_header("Location", "/b/auth/login")
        .json(&serde_json::json!({"message": "Logged out successfully"}))
}
