//! Shared request pipeline — the core solobase request handling logic.
//!
//! Both Cloudflare and native adapters call `handle_request()` after
//! converting their platform-specific HTTP types into a WAFER Message.

use futures::StreamExt;
use wafer_block::stream::StreamEvent;
use wafer_core::clients::{config as config_client, database as db};
use wafer_run::{
    block::BlockInfo, context::Context, meta::*, streams::output::TerminalNotResponse, types::*,
    InputStream, OutputStream,
};

use crate::{
    blocks::helpers::ResponseBuilder,
    features::FeatureConfig,
    routing::{self, ExtraRoute},
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
    block_infos: &[BlockInfo],
    extra_routes: &[ExtraRoute],
) -> OutputStream {
    // 0. Discovery endpoints — public, no auth required
    let path = msg.path();
    if path == "/openapi.json" || path == "/.well-known/agent.json" {
        let is_openapi = path == "/openapi.json";
        let host = msg.header("host").to_string();
        let server_url = format!("https://{}", host);
        let project_name = host.split('.').next().unwrap_or("Solobase Project");

        let body = if is_openapi {
            wafer_core::discovery::generate_openapi(block_infos, project_name, "", &server_url)
        } else {
            wafer_core::discovery::generate_agent_card(block_infos, project_name, "", &server_url)
        };

        // [SEC-073] Only emit `Access-Control-Allow-Origin: *` in dev. These
        // endpoints are intentionally public-discovery (no auth, anyone can
        // GET them), but advertising `*` to every cross-origin caller in
        // production lets unauthenticated browser code at any site map the
        // whole solobase API surface — useful reconnaissance for a targeted
        // attack. In prod we just omit the header; non-browser clients
        // (curl, the agent runtime, server-side fetchers) don't care about
        // CORS so they still see the body.
        let environment =
            config_client::get_default(ctx, "SOLOBASE_SHARED__ENVIRONMENT", "development").await;
        let is_dev = environment.eq_ignore_ascii_case("development");

        let mut resp = ResponseBuilder::new().set_header("Cache-Control", "public, max-age=3600");
        if is_dev {
            resp = resp.set_header("Access-Control-Allow-Origin", "*");
        }
        return resp.json(&body);
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

    // 3. Route to block.
    let mut stream = routing::route_to_block(ctx, msg, input, features, extra_routes).await;

    // 3a. If the block declares a streaming Content-Type up front (SSE, raw
    //     byte stream), don't drain the response into memory just to grab a
    //     status code for the audit log. The whole point of those formats is
    //     bytes flowing while the producer is still working — buffering
    //     defeats that. Skip request_logs for these responses; the trade is
    //     intentional and acceptable for v1 (callers reach for SSE for
    //     long-lived progress / chat streams which aren't the audit-worthy
    //     short request/responses that request_logs is built for).
    let (leading_meta, next_event) = drain_leading_meta(&mut stream).await;
    if let Some(ct) = leading_content_type(&leading_meta) {
        if is_streaming_content_type(ct) {
            return rebuild_streaming(leading_meta, next_event, stream);
        }
    }

    let (status_label, status_code, error_message, reply): (
        &'static str,
        i64,
        String,
        OutputStream,
    ) = match collect_buffered_with_prelude(stream, leading_meta, next_event).await {
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
        let _ = db::create(ctx, crate::blocks::admin::REQUEST_LOGS_TABLE, data).await;
    }

    reply
}

/// Rebuild an `OutputStream` from an already-collected buffered response.
/// Used by the pipeline after intercepting the stream for logging.
fn replay_buffered(body: Vec<u8>, meta: Vec<MetaEntry>) -> OutputStream {
    OutputStream::respond_with_meta(body, meta)
}

/// Pull `Meta` events off the front of an `OutputStream`, stopping at the
/// first non-`Meta` event. Returns the accumulated meta and the next event
/// (if any). Lets the pipeline peek the response's headers before deciding
/// whether to stream the body or buffer + log.
async fn drain_leading_meta(stream: &mut OutputStream) -> (Vec<MetaEntry>, Option<StreamEvent>) {
    let mut meta = Vec::new();
    while let Some(ev) = stream.next().await {
        match ev {
            StreamEvent::Meta(entry) => meta.push(entry),
            other => return (meta, Some(other)),
        }
    }
    (meta, None)
}

fn leading_content_type(meta: &[MetaEntry]) -> Option<&str> {
    for entry in meta {
        if entry.key == META_RESP_CONTENT_TYPE || entry.key == "Content-Type" {
            return Some(entry.value.as_str());
        }
    }
    None
}

fn is_streaming_content_type(ct: &str) -> bool {
    let lower = ct.to_ascii_lowercase();
    lower.starts_with("text/event-stream") || lower.starts_with("application/octet-stream")
}

/// Replay leading meta + the peeked event + remaining stream events into a
/// fresh `OutputStream`. Used for streaming responses where the pipeline
/// doesn't want to drain into memory.
fn rebuild_streaming(
    leading_meta: Vec<MetaEntry>,
    next_event: Option<StreamEvent>,
    rest: OutputStream,
) -> OutputStream {
    OutputStream::from_producer(move |sink, _cancel| async move {
        for entry in leading_meta {
            if sink.send_meta(entry).await.is_err() {
                return;
            }
        }
        let mut rest = rest;
        match next_event {
            Some(StreamEvent::Chunk(b)) => {
                if sink.send_chunk(b).await.is_err() {
                    return;
                }
            }
            Some(StreamEvent::Meta(_)) => {
                unreachable!("drain_leading_meta consumed all leading Meta")
            }
            Some(StreamEvent::Complete { meta }) => {
                let _ = sink.complete(meta).await;
                return;
            }
            Some(StreamEvent::Error(err)) => {
                let _ = sink.error(*err).await;
                return;
            }
            Some(StreamEvent::Drop) => {
                let _ = sink.drop_request().await;
                return;
            }
            Some(StreamEvent::Continue(m)) => {
                let _ = sink.continue_with(m).await;
                return;
            }
            None => {
                let _ = sink.complete(vec![]).await;
                return;
            }
        }
        // Pump the rest of the events.
        while let Some(ev) = rest.next().await {
            match ev {
                StreamEvent::Chunk(b) => {
                    if sink.send_chunk(b).await.is_err() {
                        return;
                    }
                }
                StreamEvent::Meta(entry) => {
                    if sink.send_meta(entry).await.is_err() {
                        return;
                    }
                }
                StreamEvent::Complete { meta } => {
                    let _ = sink.complete(meta).await;
                    return;
                }
                StreamEvent::Error(err) => {
                    let _ = sink.error(*err).await;
                    return;
                }
                StreamEvent::Drop => {
                    let _ = sink.drop_request().await;
                    return;
                }
                StreamEvent::Continue(m) => {
                    let _ = sink.continue_with(m).await;
                    return;
                }
            }
        }
    })
}

/// Drain remaining stream events into a buffer, prepending the leading meta
/// + the already-peeked next event. Returns the same shape as
/// `OutputStream::collect_buffered` so the buffered branch in handle_request
/// can keep working unchanged.
async fn collect_buffered_with_prelude(
    rest: OutputStream,
    leading_meta: Vec<MetaEntry>,
    next_event: Option<StreamEvent>,
) -> Result<wafer_run::streams::output::BufferedResponse, TerminalNotResponse> {
    use wafer_run::streams::output::BufferedResponse;

    let mut body = Vec::new();
    let mut meta = leading_meta;
    let mut rest = rest;

    // Two phases share the same accumulator+terminal logic: process the
    // already-peeked event, then drain the rest. Capture the "next event to
    // consider" in a small generator that yields the peek first then pulls
    // from `rest`.
    let mut peeked = next_event;
    loop {
        let ev = match peeked.take() {
            Some(e) => Some(e),
            None => rest.next().await,
        };
        match ev {
            Some(StreamEvent::Chunk(b)) => body.extend(b),
            Some(StreamEvent::Meta(entry)) => meta.push(entry),
            Some(StreamEvent::Complete { meta: m }) => {
                meta.extend(m);
                return Ok(BufferedResponse { body, meta });
            }
            Some(StreamEvent::Error(err)) => return Err(TerminalNotResponse::Error(*err)),
            Some(StreamEvent::Drop) => return Err(TerminalNotResponse::Drop),
            Some(StreamEvent::Continue(m)) => return Err(TerminalNotResponse::Continue(m)),
            None => return Err(TerminalNotResponse::Malformed),
        }
    }
}
