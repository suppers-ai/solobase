//! Shared request pipeline — the core solobase request handling logic.
//!
//! Both Cloudflare and native adapters call `handle_request()` after
//! converting their platform-specific HTTP types into a WAFER Message.

use wafer_core::clients::database as db;
use wafer_run::{
    context::Context, meta::*, streams::output::TerminalNotResponse, types::*, InputStream,
    OutputStream,
};

use crate::{
    blocks::helpers::ResponseBuilder,
    features::FeatureConfig,
    routing::{self, BlockFactory},
};

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
    mut msg: Message,
    input: InputStream,
    auth_header: Option<&str>,
    jwt_secret: &str,
    features: &dyn FeatureConfig,
    factory: &dyn BlockFactory,
) -> OutputStream {
    // 0. Discovery endpoints — public, no auth required
    let path = msg.path();
    if path == "/openapi.json" || path == "/.well-known/agent.json" {
        let is_openapi = path == "/openapi.json";
        let blocks = factory.all_block_infos();
        let host = msg.header("host").to_string();
        let server_url = format!("https://{}", host);
        let project_name = host.split('.').next().unwrap_or("Solobase Project");

        let body = if is_openapi {
            wafer_core::discovery::generate_openapi(&blocks, project_name, "", &server_url)
        } else {
            wafer_core::discovery::generate_agent_card(&blocks, project_name, "", &server_url)
        };

        return ResponseBuilder::new()
            .set_header("Cache-Control", "public, max-age=3600")
            .set_header("Access-Control-Allow-Origin", "*")
            .json(&body);
    }

    // 1. Strip /api prefix from resource path
    let resource = msg.path().to_string();
    if let Some(stripped) = resource.strip_prefix("/api") {
        msg.set_meta(META_REQ_RESOURCE, stripped);
    }

    // 2. Validate JWT or API key and set auth meta
    if let Some(header) = auth_header {
        if header.starts_with("Bearer ") {
            crate::crypto::extract_auth_meta(header, jwt_secret, &mut msg);
        } else if header.starts_with("ApiKey ") {
            let api_key = &header["ApiKey ".len()..];
            crate::blocks::auth::authenticate_api_key(ctx, api_key, &mut msg).await;
        }
    }

    // A2A JSON-RPC endpoint
    if msg.path() == "/a2a" {
        return crate::blocks::messages::a2a::handle_a2a(ctx, msg, input).await;
    }

    // Capture request info before routing (for logging)
    let method = msg.action().to_string();
    let path = msg.path().to_string();
    let client_ip = msg.remote_addr().to_string();
    let user_id = msg.user_id().to_string();
    let start_ms = crate::blocks::helpers::now_millis();

    // 3. Route to block — collect the stream so we can log status/body metadata.
    let stream = routing::route_to_block(ctx, msg, input, features, factory).await;
    let (status_label, status_code, error_message, reply): (
        &'static str,
        i64,
        String,
        OutputStream,
    ) = match stream.collect_buffered().await {
        Ok(buf) => {
            let code = buf
                .meta
                .iter()
                .find(|m| m.key == META_RESP_STATUS || m.key == "http.status")
                .and_then(|m| m.value.parse::<i64>().ok())
                .unwrap_or(200);
            (
                "OK",
                code,
                String::new(),
                replay_buffered(buf.body, buf.meta),
            )
        }
        Err(TerminalNotResponse::Error(err)) => {
            let message = err.message.clone();
            ("ERROR", 500, message, OutputStream::error(err))
        }
        Err(TerminalNotResponse::Drop) => ("OK", 204, String::new(), OutputStream::drop_request()),
        Err(TerminalNotResponse::Continue(m)) => {
            ("OK", 200, String::new(), OutputStream::continue_with(m))
        }
        Err(TerminalNotResponse::Malformed) => (
            "ERROR",
            500,
            "stream ended without terminal event".to_string(),
            OutputStream::error(WaferError {
                code: ErrorCode::Internal,
                message: "stream ended without terminal event".to_string(),
                meta: vec![],
            }),
        ),
    };

    // 4. Log the request (best-effort, don't block the response)
    let duration_ms = (crate::blocks::helpers::now_millis() - start_ms) as i64;

    // Skip logging static asset requests to reduce noise
    if !path.starts_with("/static/") && path != "/health" {
        let mut data = std::collections::HashMap::new();
        data.insert("method".to_string(), serde_json::json!(method));
        data.insert("path".to_string(), serde_json::json!(path));
        data.insert("status".to_string(), serde_json::json!(status_label));
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
        let _ = db::create(ctx, crate::blocks::admin::REQUEST_LOGS_COLLECTION, data).await;
    }

    reply
}

/// Rebuild an `OutputStream` from an already-collected buffered response.
/// Used by the pipeline after intercepting the stream for logging.
fn replay_buffered(body: Vec<u8>, meta: Vec<MetaEntry>) -> OutputStream {
    OutputStream::respond_with_meta(body, meta)
}
