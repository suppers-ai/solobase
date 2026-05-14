//! POST /b/auth/api/oauth/sync-user — relocated from auth/login.rs in Task 5.

use wafer_core::clients::{config, database as db};
use wafer_run::{context::Context, types::Message, InputStream, OutputStream};

use crate::blocks::{
    auth::USERS_TABLE,
    helpers::{err_bad_request, err_forbidden, err_internal, err_unauthorized, json_map, ok_json},
};

pub async fn handle(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    // Internal endpoint for OAuth user sync — requires INTERNAL_SECRET
    let expected_secret = config::get_default(ctx, "SUPPERS_AI__AUTH__INTERNAL_SECRET", "").await;
    if expected_secret.is_empty() {
        return err_forbidden("INTERNAL_SECRET not configured — internal endpoints are disabled");
    }
    let provided_secret = msg.header("x-internal-secret");
    if !crate::crypto::constant_time_eq(provided_secret.as_bytes(), expected_secret.as_bytes()) {
        return err_unauthorized("Invalid internal secret");
    }

    #[derive(serde::Deserialize)]
    struct SyncReq {
        email: String,
        name: Option<String>,
        provider: Option<String>,
    }
    let raw = input.collect_to_bytes().await;
    let body: SyncReq = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    let email_lower = body.email.trim().to_lowercase();
    let user = match db::get_by_field(
        ctx,
        USERS_TABLE,
        "email",
        serde_json::Value::String(email_lower.clone()),
    )
    .await
    {
        Ok(u) => u,
        Err(_) => {
            let mut data = json_map(serde_json::json!({
                "email": email_lower,
                "name": body.name.unwrap_or_default(),
                "oauth_provider": body.provider.unwrap_or_default(),
                "disabled": false
            }));
            crate::blocks::helpers::stamp_created(&mut data);
            match db::create(ctx, USERS_TABLE, data).await {
                Ok(u) => u,
                Err(e) => return err_internal("Create failed", e),
            }
        }
    };

    ok_json(&serde_json::json!({"id": user.id, "email": user.data.get("email")}))
}
