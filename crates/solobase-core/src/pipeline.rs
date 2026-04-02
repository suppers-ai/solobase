//! Shared request pipeline — the core solobase request handling logic.
//!
//! Both Cloudflare and native adapters call `handle_request()` after
//! converting their platform-specific HTTP types into a WAFER Message.

use wafer_core::clients::database as db;
use wafer_run::context::Context;
use wafer_run::meta::*;
use wafer_run::types::*;

use crate::features::FeatureConfig;
use crate::routing::{self, BlockFactory};

/// Handle a solobase request.
///
/// This is the shared entry point that both CF and native adapters call
/// after building a Message from the incoming HTTP request.
///
/// Steps:
/// 1. Strip `/api` prefix (CF convention — native doesn't use it)
/// 2. Validate JWT and set auth meta
/// 3. Route to the appropriate solobase block
/// 4. Log the request to `request_logs` (async, best-effort)
pub async fn handle_request(
    ctx: &dyn Context,
    msg: &mut Message,
    auth_header: Option<&str>,
    jwt_secret: &str,
    features: &dyn FeatureConfig,
    factory: &dyn BlockFactory,
) -> Result_ {
    // 1. Strip /api prefix from resource path
    let resource = msg.path().to_string();
    if let Some(stripped) = resource.strip_prefix("/api") {
        msg.set_meta(META_REQ_RESOURCE, stripped);
    }

    // 2. Validate JWT and set auth meta
    if let Some(header) = auth_header {
        crate::crypto::extract_auth_meta(header, jwt_secret, msg);
    }

    // Capture request info before routing (for logging)
    let method = msg.action().to_string();
    let path = msg.path().to_string();
    let client_ip = msg.remote_addr().to_string();
    let start_ms = crate::blocks::helpers::now_millis();

    // 3. Route to block
    let result = routing::route_to_block(ctx, msg, features, factory).await;

    // 4. Log the request (best-effort, don't block the response)
    let duration_ms = (crate::blocks::helpers::now_millis() - start_ms) as i64;
    let user_id = msg.user_id().to_string();
    let (status, status_code, error_message) = match result.action {
        Action::Error => {
            let err_msg = result
                .error
                .as_ref()
                .map(|e| e.message.clone())
                .unwrap_or_default();
            ("ERROR", 500i64, err_msg)
        }
        _ => {
            let code = result
                .response
                .as_ref()
                .and_then(|r| r.meta.iter().find(|m| m.key == "resp.status"))
                .and_then(|m| m.value.parse::<i64>().ok())
                .unwrap_or(200);
            ("OK", code, String::new())
        }
    };

    // Skip logging static asset requests to reduce noise
    if !path.starts_with("/static/") && path != "/health" {
        let mut data = std::collections::HashMap::new();
        data.insert("method".to_string(), serde_json::json!(method));
        data.insert("path".to_string(), serde_json::json!(path));
        data.insert("status".to_string(), serde_json::json!(status));
        data.insert("status_code".to_string(), serde_json::json!(status_code));
        data.insert("duration_ms".to_string(), serde_json::json!(duration_ms));
        data.insert(
            "error_message".to_string(),
            serde_json::json!(error_message),
        );
        data.insert("client_ip".to_string(), serde_json::json!(client_ip));
        data.insert("user_id".to_string(), serde_json::json!(user_id));
        crate::blocks::helpers::stamp_created(&mut data);

        // Best-effort: don't fail the request if logging fails
        let _ = db::create(ctx, "request_logs", data).await;
    }

    result
}
