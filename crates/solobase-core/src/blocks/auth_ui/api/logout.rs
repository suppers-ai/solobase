//! POST /b/auth/api/logout — relocated from auth/login.rs in Task 5.

use wafer_core::clients::database as db;
use wafer_run::{context::Context, types::Message, OutputStream};

use crate::blocks::{
    auth::{helpers::build_auth_cookie, TOKENS_TABLE},
    helpers::ResponseBuilder,
};

pub async fn handle(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let user_id = msg.user_id();
    if !user_id.is_empty() {
        db::delete_by_field(
            ctx,
            TOKENS_TABLE,
            "user_id",
            serde_json::Value::String(user_id.to_string()),
        )
        .await
        .ok();
    }

    let cookie = build_auth_cookie("", 0, ctx).await;
    ResponseBuilder::new()
        .set_cookie(&cookie)
        .status(303)
        .set_header("Location", "/b/auth/login")
        .json(&serde_json::json!({"message": "Logged out successfully"}))
}
