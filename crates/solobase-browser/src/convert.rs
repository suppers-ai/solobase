use futures::{SinkExt, StreamExt};
use js_sys::{ArrayBuffer, Uint8Array};
use wafer_block::{
    meta::{
        META_REQ_ACTION, META_REQ_CLIENT_IP, META_REQ_CONTENT_TYPE, META_REQ_QUERY_PREFIX,
        META_REQ_RESOURCE, META_RESP_CONTENT_TYPE, META_RESP_COOKIE_PREFIX,
        META_RESP_HEADER_PREFIX, META_RESP_STATUS,
    },
    stream::StreamEvent,
    streams::{
        input::InputStream,
        output::{OutputStream, TerminalNotResponse},
    },
    ErrorCode, Message, MetaGet, MetaEntry,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, ResponseInit};

use crate::helpers::urlencoding_decode;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn http_method_to_action(method: &str) -> &'static str {
    match method.to_uppercase().as_str() {
        "GET" | "HEAD" => "retrieve",
        "POST" => "create",
        "PUT" | "PATCH" => "update",
        "DELETE" => "delete",
        _ => "execute",
    }
}

fn error_code_to_http_status(code: &ErrorCode) -> u16 {
    match code {
        ErrorCode::Ok => 200,
        ErrorCode::Cancelled => 499,
        ErrorCode::InvalidArgument => 400,
        ErrorCode::DeadlineExceeded => 504,
        ErrorCode::NotFound => 404,
        ErrorCode::AlreadyExists => 409,
        ErrorCode::PermissionDenied => 403,
        ErrorCode::ResourceExhausted => 429,
        ErrorCode::FailedPrecondition => 412,
        ErrorCode::Aborted => 409,
        ErrorCode::OutOfRange => 400,
        ErrorCode::Unimplemented => 501,
        ErrorCode::Internal => 500,
        ErrorCode::Unavailable => 503,
        ErrorCode::DataLoss => 500,
        ErrorCode::Unauthenticated => 401,
        _ => 500,
    }
}

// ---------------------------------------------------------------------------
// Request conversion
// ---------------------------------------------------------------------------

/// Convert a browser `web_sys::Request` into a WAFER `(Message, InputStream)` pair.
///
/// Mirrors `http_to_message()` from `wafer-block-http-listener`.  The remote
/// address is always `"127.0.0.1"` — in a Service Worker the request comes
/// from the same device.
pub async fn request_to_message(
    request: &web_sys::Request,
) -> Result<(Message, InputStream), JsValue> {
    let method = request.method();
    let url_str = request.url();

    // Parse the URL so we can separate path and query string.
    let url = web_sys::Url::new(&url_str)?;
    let path = url.pathname();
    // search() includes the leading '?' — strip it.
    let search = url.search();
    let raw_query = if search.starts_with('?') {
        search[1..].to_string()
    } else {
        search
    };

    // Read body bytes via ArrayBuffer.
    const MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10MB
    let body: Vec<u8> = {
        let promise = request.array_buffer()?;
        let ab_val = JsFuture::from(promise).await?;
        let ab: ArrayBuffer = ab_val.dyn_into()?;
        let arr = Uint8Array::new(&ab);
        if arr.length() as usize > MAX_BODY_SIZE {
            return Err(JsValue::from_str("Request body too large"));
        }
        arr.to_vec()
    };

    let mut msg = Message::new(format!("{}:{}", method, path));

    // HTTP-specific meta (mirrors the native listener).
    msg.set_meta("http.method", method.clone());
    msg.set_meta("http.path", path.clone());
    msg.set_meta("http.raw_query", raw_query.clone());
    msg.set_meta("http.remote_addr", "127.0.0.1");

    // Collect headers into meta.
    let headers: Headers = request.headers();
    // Iterate the headers JS iterator.
    let iter =
        js_sys::try_iter(&headers)?.ok_or_else(|| JsValue::from_str("headers not iterable"))?;
    let mut content_type = String::new();
    let mut host = String::new();
    for item in iter {
        let item = item?;
        // Each entry is a JS Array [name, value].
        let arr: js_sys::Array = item.dyn_into()?;
        let key = arr.get(0).as_string().unwrap_or_default().to_lowercase();
        let val = arr.get(1).as_string().unwrap_or_default();
        if key == "content-type" {
            content_type = val.clone();
        }
        if key == "host" {
            host = val.clone();
        }
        msg.set_meta(format!("http.header.{}", key), val);
    }

    msg.set_meta("http.content_type", content_type.clone());
    msg.set_meta("http.host", host);

    // Re-inject the `Cookie` header from the SW's CookieStore.
    // `FetchEvent.request.headers` filters `Cookie` out per the SW spec, so
    // the header-iteration loop above never sees it even though the browser
    // sends cookies on same-origin requests. CookieStore is the only way to
    // read them back inside the SW.
    let cookie_val = crate::bridge::read_cookie_header().await;
    if let Some(s) = cookie_val.as_string() {
        if !s.is_empty() {
            msg.set_meta("http.header.cookie", s);
        }
    }

    // Normalised request meta.
    msg.set_meta(META_REQ_ACTION, http_method_to_action(&method));
    msg.set_meta(META_REQ_RESOURCE, path.clone());
    msg.set_meta(META_REQ_CLIENT_IP, "127.0.0.1");
    msg.set_meta(META_REQ_CONTENT_TYPE, content_type);

    // Decode query parameters into both `http.query.*` and `req.query.*`.
    if !raw_query.is_empty() {
        for pair in raw_query.split('&') {
            let mut parts = pair.splitn(2, '=');
            if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
                let decoded_val = urlencoding_decode(val);
                msg.set_meta(format!("http.query.{}", key), decoded_val.clone());
                msg.set_meta(format!("{}{}", META_REQ_QUERY_PREFIX, key), decoded_val);
            }
        }
    }

    Ok((msg, InputStream::from_bytes(body)))
}

// ---------------------------------------------------------------------------
// Response conversion
// ---------------------------------------------------------------------------

/// Apply response meta entries to a `web_sys::Headers` object.
///
/// Mirrors `apply_response_meta()` from the native listener.
fn apply_response_meta(headers: &Headers, meta: &[wafer_block::MetaEntry]) -> Result<(), JsValue> {
    for entry in meta {
        let k = entry.key.as_str();
        let v = &entry.value;
        match k {
            k if k == META_RESP_STATUS || k == "http.status" => {
                // Status is handled separately; skip.
            }
            k if k.starts_with(META_RESP_COOKIE_PREFIX)
                || k.starts_with("http.resp.set-cookie.") =>
            {
                headers.append("Set-Cookie", v)?;
            }
            k if k.starts_with(META_RESP_HEADER_PREFIX) => {
                let header_name = &k[META_RESP_HEADER_PREFIX.len()..];
                headers.set(header_name, v)?;
            }
            k if k.starts_with("http.resp.header.") => {
                let header_name = &k[17..];
                headers.set(header_name, v)?;
            }
            k if k == META_RESP_CONTENT_TYPE || k == "Content-Type" => {
                headers.set("Content-Type", v)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn get_status_code(meta: &[wafer_block::MetaEntry], default_code: u16) -> u16 {
    if let Some(code) = MetaGet::get(meta, META_RESP_STATUS) {
        if let Ok(n) = code.parse::<u16>() {
            return n;
        }
    }
    if let Some(code) = MetaGet::get(meta, "http.status") {
        if let Ok(n) = code.parse::<u16>() {
            return n;
        }
    }
    default_code
}

fn get_error_status_code(
    error: Option<&wafer_block::WaferError>,
    meta: &[wafer_block::MetaEntry],
) -> u16 {
    let from_meta = get_status_code(meta, 0);
    if from_meta > 0 {
        return from_meta;
    }
    if let Some(err) = error {
        return error_code_to_http_status(&err.code);
    }
    500
}

/// Build a `web_sys::Response` from raw bytes, a status code, and a
/// `web_sys::Headers` object.
fn make_response(
    body: Vec<u8>,
    status: u16,
    headers: Headers,
) -> Result<web_sys::Response, JsValue> {
    let init = ResponseInit::new();
    init.set_status(status);
    init.set_headers(&headers);

    if body.is_empty() {
        web_sys::Response::new_with_opt_str_and_init(None, &init)
    } else {
        // Copy into a Uint8Array then pass as BufferSource.
        let arr = Uint8Array::new_with_length(body.len() as u32);
        arr.copy_from(&body);
        let ab: ArrayBuffer = arr.buffer();
        web_sys::Response::new_with_opt_buffer_source_and_init(Some(&ab.into()), &init)
    }
}

/// True for content-types that should stream body chunks to the browser as
/// they're produced rather than buffer the entire response. Today: SSE and
/// generic byte streams (which feature blocks use for downloads / archives).
fn is_streaming_content_type(ct: &str) -> bool {
    let lower = ct.to_ascii_lowercase();
    lower.starts_with("text/event-stream") || lower.starts_with("application/octet-stream")
}

/// Pull `Meta` events off the front of an `OutputStream`, stopping at the
/// first non-Meta event. Returns the accumulated meta and the next event
/// (if any). Used by `output_to_response` to peek the response's headers
/// before deciding whether to stream the body or buffer it.
async fn drain_leading_meta(output: &mut OutputStream) -> (Vec<MetaEntry>, Option<StreamEvent>) {
    let mut meta = Vec::new();
    while let Some(ev) = output.next().await {
        match ev {
            StreamEvent::Meta(entry) => meta.push(entry),
            other => return (meta, Some(other)),
        }
    }
    (meta, None)
}

/// Build a `ReadableStream` body that pipes remaining `Chunk` events from an
/// `OutputStream` to the browser. The first chunk (already pulled from the
/// stream) is pushed first, then the loop drains the rest.
///
/// Terminal events (`Complete`, `Error`, `Drop`, `Continue`) close the stream.
/// `Error` after we've already started writing the body is the stream just
/// being aborted — there's no way to flip the HTTP status mid-response, so
/// we close the stream and let the browser surface the truncation.
/// Build a JS `ReadableStream` that yields `first_chunk` and then every
/// subsequent `Chunk` event from `remaining`. Mid-body `Meta` is dropped
/// (too late to apply to HTTP headers); any terminal closes the stream.
fn make_streaming_body(
    first_chunk: Vec<u8>,
    mut remaining: OutputStream,
) -> wasm_streams::ReadableStream {
    use futures::channel::mpsc;
    let (mut tx, rx) = mpsc::channel::<Result<JsValue, JsValue>>(8);

    // Channel cap is 8 and we have one item to send — try_send fits. If it
    // fails (caller dropped the stream before consuming) we just discard.
    let _ = tx.try_send(Ok(JsValue::from(Uint8Array::from(first_chunk.as_slice()))));

    wasm_bindgen_futures::spawn_local(async move {
        while let Some(ev) = remaining.next().await {
            match ev {
                StreamEvent::Chunk(bytes) => {
                    let val: Result<JsValue, JsValue> =
                        Ok(JsValue::from(Uint8Array::from(bytes.as_slice())));
                    if tx.send(val).await.is_err() {
                        // Browser dropped the response stream — stop pumping.
                        return;
                    }
                }
                // Mid-body Meta is too late to apply to HTTP headers; drop it.
                StreamEvent::Meta(_) => {}
                // Any terminal closes the body. Error after partial body has
                // already streamed bytes can't change the HTTP status — log
                // and close cleanly so the browser sees a normal end-of-body.
                StreamEvent::Error(err) => {
                    web_sys::console::warn_1(
                        &format!(
                            "solobase-browser: streaming response aborted: {}",
                            err.message
                        )
                        .into(),
                    );
                    return;
                }
                StreamEvent::Complete { .. }
                | StreamEvent::Drop
                | StreamEvent::Continue(_)
                | StreamEvent::Halt { .. } => {
                    // Mid-body Halt cannot change the HTTP status (headers
                    // already flushed); treat it as another terminal that
                    // closes the body cleanly. The browser sees a normal
                    // end-of-stream.
                    return;
                }
            }
        }
    });

    wasm_streams::ReadableStream::from_stream(rx)
}

/// Convert a WAFER `OutputStream` into a browser `web_sys::Response`.
///
/// Two paths:
/// 1. **Buffered** (default) — for blocks that emit `Chunk(bytes), Complete{meta}`
///    via `respond_with_meta`. Status, headers, and body all live in the
///    terminal `Complete{meta}` for those blocks, so we have to read the
///    whole stream before we can build the `Response`. Same behaviour as
///    before this commit.
/// 2. **Streaming** — for blocks that emit leading `Meta` events declaring
///    `Content-Type: text/event-stream` (or `application/octet-stream`)
///    BEFORE the first `Chunk`. We build a `Response` with a `ReadableStream`
///    body and pipe subsequent chunks straight to the browser, so a
///    multi-minute SSE response doesn't get held back behind a buffer that
///    flushes at the very end (which Chrome's idle keep-alive treats as a
///    hung fetch and drops with `net::ERR_FAILED`).
pub async fn output_to_response(mut output: OutputStream) -> Result<web_sys::Response, JsValue> {
    // Peek leading Meta events without consuming Chunks. The streaming path
    // is signalled by an early Content-Type meta; buffered blocks send no
    // Meta before their first Chunk, so this just returns an empty vec for
    // them and the loop below falls through to `collect_buffered`.
    let (leading_meta, next_event) = drain_leading_meta(&mut output).await;

    let leading_ct = MetaGet::get(&leading_meta, META_RESP_CONTENT_TYPE)
        .or_else(|| MetaGet::get(&leading_meta, "Content-Type"));

    if let (Some(ct), Some(StreamEvent::Chunk(first))) = (leading_ct, &next_event) {
        if is_streaming_content_type(ct) {
            return build_streaming_response(leading_meta, first.clone(), output);
        }
    }

    // Buffered path — drain the rest, prepending whatever we've already
    // pulled (leading meta + the next event we peeked).
    let (terminal_result, mut prelude_meta, mut prelude_body) = match next_event {
        Some(StreamEvent::Chunk(b)) => {
            // Continue collecting from where we are. Build a synthetic stream
            // that yields the rest of `output` and replays the chunk we just
            // peeked at the front.
            (output.collect_buffered().await, leading_meta, b)
        }
        Some(StreamEvent::Complete { meta }) => {
            // Terminal landed before any chunk — this is a header-only OK.
            // Combine prelude + terminal meta and emit an empty body.
            let mut all_meta = leading_meta;
            all_meta.extend(meta);
            return finalise_buffered(Ok(buffered_view(Vec::new(), all_meta)));
        }
        Some(StreamEvent::Error(err)) => {
            return finalise_buffered(Err(TerminalNotResponse::Error(*err)));
        }
        Some(StreamEvent::Drop) => {
            return finalise_buffered(Err(TerminalNotResponse::Drop));
        }
        Some(StreamEvent::Continue(msg)) => {
            return finalise_buffered(Err(TerminalNotResponse::Continue(msg)));
        }
        Some(StreamEvent::Meta(_)) => unreachable!("drain_leading_meta consumes Meta events"),
        Some(StreamEvent::Halt { body, meta }) => {
            // Halt before any chunk — short-circuit terminal carrying its
            // own body+meta. Combine the prelude meta we drained with the
            // Halt's meta and finalise as a buffered Halt terminal.
            use wafer_run::streams::output::BufferedResponse;
            let mut all_meta = leading_meta;
            all_meta.extend(meta);
            return finalise_buffered(Err(TerminalNotResponse::Halt(BufferedResponse {
                body,
                meta: all_meta,
            })));
        }
        None => {
            // Stream ended after meta-only with no terminal — malformed.
            return finalise_buffered(Err(TerminalNotResponse::Malformed));
        }
    };

    match terminal_result {
        Ok(buf) => {
            // Merge the leading meta we drained earlier with the buffered tail.
            prelude_meta.extend(buf.meta);
            prelude_body.extend(buf.body);
            finalise_buffered(Ok(buffered_view(prelude_body, prelude_meta)))
        }
        Err(other) => finalise_buffered(Err(other)),
    }
}

/// Local mirror of `wafer_block::streams::output::BufferedOutput` so we can
/// re-enter the buffered finaliser with a synthetic value built from the
/// leading-meta drain. Carries body bytes + accumulated meta.
struct BufferedView {
    body: Vec<u8>,
    meta: Vec<MetaEntry>,
}

fn buffered_view(body: Vec<u8>, meta: Vec<MetaEntry>) -> BufferedView {
    BufferedView { body, meta }
}

fn finalise_buffered(
    result: Result<BufferedView, TerminalNotResponse>,
) -> Result<web_sys::Response, JsValue> {
    match result {
        Ok(buf) => {
            let status = get_status_code(&buf.meta, 200);
            let headers = Headers::new()?;
            apply_response_meta(&headers, &buf.meta)?;

            let has_ct = MetaGet::contains_key(&buf.meta, META_RESP_CONTENT_TYPE)
                || MetaGet::contains_key(&buf.meta, "Content-Type");
            if !has_ct {
                headers.set("Content-Type", "application/json")?;
            }

            make_response(buf.body, status, headers)
        }

        Err(TerminalNotResponse::Error(err)) => {
            let status = get_error_status_code(Some(&err), &err.meta);
            let headers = Headers::new()?;
            apply_response_meta(&headers, &err.meta)?;
            headers.set("Content-Type", "application/json")?;

            let body = serde_json::json!({
                "error": err.code,
                "message": err.message,
            })
            .to_string()
            .into_bytes();

            make_response(body, status, headers)
        }

        Err(TerminalNotResponse::Drop) => {
            let headers = Headers::new()?;
            make_response(Vec::new(), 204, headers)
        }

        Err(TerminalNotResponse::Continue(_msg)) => {
            let headers = Headers::new()?;
            headers.set("Content-Type", "application/json")?;
            make_response(
                b"{\"error\":\"continue not supported by listener\"}".to_vec(),
                500,
                headers,
            )
        }

        Err(TerminalNotResponse::Malformed) => {
            let headers = Headers::new()?;
            headers.set("Content-Type", "application/json")?;
            make_response(
                b"{\"error\":\"stream ended without terminal event\"}".to_vec(),
                500,
                headers,
            )
        }

        Err(TerminalNotResponse::Halt(buf)) => {
            // Halt is a successful short-circuit terminal — the body+meta
            // ARE the response. Same wire shape as the Ok arm.
            let status = get_status_code(&buf.meta, 200);
            let headers = Headers::new()?;
            apply_response_meta(&headers, &buf.meta)?;

            let has_ct = MetaGet::contains_key(&buf.meta, META_RESP_CONTENT_TYPE)
                || MetaGet::contains_key(&buf.meta, "Content-Type");
            if !has_ct {
                headers.set("Content-Type", "application/json")?;
            }

            make_response(buf.body, status, headers)
        }
    }
}

/// Build a streaming `web_sys::Response` from the leading meta (carrying
/// status + headers) and an `OutputStream` whose remaining events should be
/// piped into the body.
fn build_streaming_response(
    leading_meta: Vec<MetaEntry>,
    first_chunk: Vec<u8>,
    remaining: OutputStream,
) -> Result<web_sys::Response, JsValue> {
    let status = get_status_code(&leading_meta, 200);
    let headers = Headers::new()?;
    apply_response_meta(&headers, &leading_meta)?;

    let has_ct = MetaGet::contains_key(&leading_meta, META_RESP_CONTENT_TYPE)
        || MetaGet::contains_key(&leading_meta, "Content-Type");
    if !has_ct {
        // Streaming bodies without an explicit Content-Type fall back to
        // octet-stream rather than the JSON default the buffered path uses.
        headers.set("Content-Type", "application/octet-stream")?;
    }

    let stream = make_streaming_body(first_chunk, remaining);
    let raw_js = stream.into_raw();
    let init = ResponseInit::new();
    init.set_status(status);
    init.set_headers(&headers);
    web_sys::Response::new_with_opt_readable_stream_and_init(Some(&raw_js), &init)
}
