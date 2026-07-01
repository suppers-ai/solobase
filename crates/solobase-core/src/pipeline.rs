//! Shared request pipeline — the core solobase request handling logic.
//!
//! Both Cloudflare and native adapters call `handle_request()` after
//! converting their platform-specific HTTP types into a WAFER Message.

use std::cell::{Cell, RefCell};

use futures::StreamExt;
use wafer_block::{
    http_codec::{self, ResponseMetaPart},
    stream::StreamEvent,
};
use wafer_core::clients::{config as config_client, database as db};
use wafer_run::{
    context::Context,
    streams::output::{BufferedResponse, OutputSink, TerminalNotResponse},
    BlockInfo, ErrorCode, InputStream, Message, MetaEntry, OutputStream, WaferError,
    META_REQ_RESOURCE,
};

use crate::{
    features::FeatureConfig,
    http::ResponseBuilder,
    routing::{self, ExtraRoute},
};

/// How the pipeline persists the per-request audit row.
///
/// `Inline` (default; native): `db::create` awaited on the response path —
/// today's behavior. `Queued` (Cloudflare): the completed row is pushed to a
/// thread-local queue; the platform entry drains it after dispatch and
/// attaches the write to `ctx.wait_until`, so responses stop paying one D1
/// write of latency. Rows are plain data, so it does not matter which
/// interleaved request's drain flushes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestLogMode {
    Inline,
    Queued,
}

/// One queued audit row (table + column map), ready for `DatabaseService::create`.
pub struct QueuedRequestLog {
    pub table: &'static str,
    pub data: std::collections::HashMap<String, serde_json::Value>,
}

thread_local! {
    static REQUEST_LOG_MODE: Cell<RequestLogMode> = const { Cell::new(RequestLogMode::Inline) };
    static REQUEST_LOG_QUEUE: RefCell<Vec<QueuedRequestLog>> = const { RefCell::new(Vec::new()) };
}

/// Select the request-log persistence mode for this thread (isolate).
/// Called once at platform init; native never calls it.
pub fn set_request_log_mode(mode: RequestLogMode) {
    REQUEST_LOG_MODE.with(|m| m.set(mode));
}

fn request_log_mode() -> RequestLogMode {
    REQUEST_LOG_MODE.with(Cell::get)
}

fn enqueue_request_log(
    table: &'static str,
    data: std::collections::HashMap<String, serde_json::Value>,
) {
    REQUEST_LOG_QUEUE.with(|q| q.borrow_mut().push(QueuedRequestLog { table, data }));
}

/// Take every queued row, clearing the queue. The platform entry calls this
/// after each dispatch and persists the rows off the response path.
pub fn drain_queued_request_logs() -> Vec<QueuedRequestLog> {
    REQUEST_LOG_QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()))
}

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
///
/// # Errors
///
/// Never returns an error directly — errors are encoded inside the
/// returned `OutputStream` as `StreamEvent::Error`. Request-log
/// persistence failures are intentionally swallowed (best-effort) so a
/// failing audit-log table never breaks the response.
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
            // [SEC-038] Read the deployment's expected issuer once per request
            // so JWTs minted under a different deployment's FRONTEND_URL get
            // rejected even if their HMAC secret matches. [SEC-042] also
            // consults the JWT blocklist via the ctx-aware extractor.
            let expected_iss = crate::blocks::auth::helpers::expected_issuer(ctx).await;
            crate::crypto::extract_auth_meta(ctx, header, jwt_secret, &expected_iss, &mut msg)
                .await;
        } else if header.starts_with("ApiKey ") {
            let api_key = &header["ApiKey ".len()..];
            crate::blocks::auth::authenticate_api_key(ctx, api_key, &mut msg).await;
        }
    }

    // Capture request info before routing (for logging)
    let method = msg.action().to_string();
    let path = msg.path().to_string();
    let client_ip = msg.remote_addr().to_string();
    let user_id = msg.user_id().to_string();
    let start_ms = crate::util::now_millis();

    // 3. Route to block.
    let mut stream =
        routing::route_to_block(ctx, msg, input, features, block_infos, extra_routes).await;

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
            let code = i64::from(http_codec::resolve_status(&buf.meta, 200));
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
        Err(TerminalNotResponse::Halt(buf)) => {
            let code = i64::from(http_codec::resolve_status(&buf.meta, 200));
            (
                "OK",
                code,
                String::new(),
                OutputStream::from_buffered_response(buf),
            )
        }
    };

    // 4. Log the request (best-effort, don't block the response).
    // `now_millis()` reads wall clock — saturating_sub guards against clock
    // skew on suspend/resume from regressing the subtraction, and try_into
    // clamps the unlikely case of an absurdly large delta to `i64::MAX`.
    let duration_ms =
        i64::try_from(crate::util::now_millis().saturating_sub(start_ms)).unwrap_or(i64::MAX);

    // Skip logging static asset requests to reduce noise (one request_logs
    // write per CSS/JS/font/logo fetch otherwise). The prefix is the shared
    // `routing::STATIC_PREFIX` const so it can't drift from the routing
    // table and the `ui::assets` URL builders again.
    if !path.starts_with(routing::STATIC_PREFIX) && path != "/health" {
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
        crate::util::stamp_created(&mut data);

        match request_log_mode() {
            RequestLogMode::Inline => {
                // Best-effort: don't fail the request if logging fails
                let _ = db::create(ctx, crate::blocks::admin::REQUEST_LOGS_TABLE, data).await;
            }
            RequestLogMode::Queued => {
                enqueue_request_log(crate::blocks::admin::REQUEST_LOGS_TABLE, data);
            }
        }
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
/// whether to stream the body or buffer it.
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

/// The canonical `resp.content_type` among the leading meta entries, if any.
/// Legacy aliases (a literal `Content-Type` meta key) are not honored — the
/// canonical-keys-only policy is pinned by `wafer_block::http_codec`.
fn leading_content_type(meta: &[MetaEntry]) -> Option<&str> {
    http_codec::response_meta_parts(meta).find_map(|part| match part {
        ResponseMetaPart::ContentType(ct) => Some(ct),
        _ => None,
    })
}

/// True for content-types that should stream body chunks to the client as
/// they're produced rather than buffer the entire response. Today: SSE and
/// generic byte streams (which feature blocks use for downloads / archives).
fn is_streaming_content_type(ct: &str) -> bool {
    let lower = ct.to_ascii_lowercase();
    lower.starts_with("text/event-stream") || lower.starts_with("application/octet-stream")
}

/// Forward one `StreamEvent` into an `OutputSink`. Returns the sink back for
/// non-terminal events so the caller can keep pumping; terminal events (and
/// a hung-up consumer) consume it and return `None`.
async fn forward_event(sink: OutputSink, ev: StreamEvent) -> Option<OutputSink> {
    match ev {
        StreamEvent::Chunk(bytes) => sink.send_chunk(bytes).await.ok().map(|()| sink),
        StreamEvent::Meta(entry) => sink.send_meta(entry).await.ok().map(|()| sink),
        StreamEvent::Complete { meta } => {
            let _ = sink.complete(meta).await;
            None
        }
        StreamEvent::Error(err) => {
            let _ = sink.error(*err).await;
            None
        }
        StreamEvent::Drop => {
            let _ = sink.drop_request().await;
            None
        }
        StreamEvent::Continue(msg) => {
            let _ = sink.continue_with(msg).await;
            None
        }
        StreamEvent::Halt { body, meta } => {
            let _ = sink.halt(body, meta).await;
            None
        }
    }
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
        let Some(next_event) = next_event else {
            // The stream ended right after its leading meta with no
            // terminal; close out as an empty Complete.
            let _ = sink.complete(Vec::new()).await;
            return;
        };
        let Some(mut sink) = forward_event(sink, next_event).await else {
            return;
        };
        let mut rest = rest;
        while let Some(ev) = rest.next().await {
            match forward_event(sink, ev).await {
                Some(s) => sink = s,
                None => return,
            }
        }
        // `rest` ended without a terminal; `from_producer` auto-Completes.
    })
}

/// Collect the remaining stream events into a buffer, prepending the leading
/// meta + the already-peeked next event. Equivalent to running
/// [`OutputStream::collect_buffered`] over the reassembled stream — including
/// its contract that a `Halt` terminal **replaces** any previously streamed
/// chunks/meta (mixing them is a producer bug `collect_buffered` warns
/// about), so the prelude is discarded on that path.
///
/// `next_event` must come from [`drain_leading_meta`] (i.e. it is never
/// `StreamEvent::Meta`).
async fn collect_buffered_with_prelude(
    rest: OutputStream,
    leading_meta: Vec<MetaEntry>,
    next_event: Option<StreamEvent>,
) -> Result<BufferedResponse, TerminalNotResponse> {
    match next_event {
        // Body already started: drain the remainder with the standard
        // collector and stitch the prelude onto the front of a successful
        // response. Non-Complete terminals pass through unchanged, exactly
        // as `collect_buffered` would have produced them.
        Some(StreamEvent::Chunk(first)) => match rest.collect_buffered().await {
            Ok(buf) => {
                let mut body = first;
                body.extend(buf.body);
                let mut meta = leading_meta;
                meta.extend(buf.meta);
                Ok(BufferedResponse { body, meta })
            }
            Err(terminal) => Err(terminal),
        },
        Some(StreamEvent::Meta(_)) => unreachable!("drain_leading_meta consumes Meta events"),
        Some(StreamEvent::Complete { meta }) => {
            let mut all_meta = leading_meta;
            all_meta.extend(meta);
            Ok(BufferedResponse {
                body: Vec::new(),
                meta: all_meta,
            })
        }
        Some(StreamEvent::Halt { body, meta }) => {
            // Halt carries a complete response; per the `collect_buffered`
            // contract any prior streamed events — the prelude included —
            // are replaced by its payload.
            if !leading_meta.is_empty() {
                tracing::warn!(
                    discarded_meta_entries = leading_meta.len(),
                    "Halt terminal arrived after leading Meta; discarding prelude (producer must not mix Halt with streamed events)"
                );
            }
            Err(TerminalNotResponse::Halt(BufferedResponse { body, meta }))
        }
        Some(StreamEvent::Error(err)) => Err(TerminalNotResponse::Error(*err)),
        Some(StreamEvent::Drop) => Err(TerminalNotResponse::Drop),
        Some(StreamEvent::Continue(msg)) => Err(TerminalNotResponse::Continue(msg)),
        None => Err(TerminalNotResponse::Malformed),
    }
}

#[cfg(test)]
mod request_log_mode_tests {
    use crate::blocks::admin;

    use super::{
        drain_queued_request_logs, enqueue_request_log, set_request_log_mode, RequestLogMode,
    };

    #[test]
    fn default_mode_is_inline_and_drain_is_empty() {
        assert!(drain_queued_request_logs().is_empty());
    }

    #[test]
    fn queued_mode_accumulates_and_drain_clears() {
        set_request_log_mode(RequestLogMode::Queued);
        let mut data = std::collections::HashMap::new();
        data.insert("path".to_string(), serde_json::json!("/x"));
        enqueue_request_log(admin::REQUEST_LOGS_TABLE, data.clone());
        enqueue_request_log(admin::REQUEST_LOGS_TABLE, data);

        let drained = drain_queued_request_logs();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].table, admin::REQUEST_LOGS_TABLE);
        assert!(drain_queued_request_logs().is_empty(), "drain must clear");

        set_request_log_mode(RequestLogMode::Inline); // restore for other tests
    }
}
