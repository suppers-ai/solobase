//! HTTP ↔ Message conversion for Cloudflare Workers (streaming protocol).
//!
//! Converts `worker::Request` → `(Message, InputStream)` and
//! `OutputStream` → `worker::Response`. Mirrors the native axum adapter
//! in `wafer-block-http-listener` and the browser adapter in `solobase-web`.

use solobase_core::blocks::helpers::urlencoding_decode;
use wafer_run::{
    ErrorCode, InputStream, Message, MetaEntry, OutputStream, TerminalNotResponse, WaferError,
    META_REQ_ACTION, META_REQ_CLIENT_IP, META_REQ_CONTENT_TYPE, META_REQ_QUERY_PREFIX,
    META_REQ_RESOURCE, META_RESP_CONTENT_TYPE, META_RESP_COOKIE_PREFIX, META_RESP_HEADER_PREFIX,
    META_RESP_STATUS,
};
use worker::{Request, Response, Result};

// ---------------------------------------------------------------------------
// Request conversion
// ---------------------------------------------------------------------------

/// Convert a Cloudflare Worker Request into a WAFER `(Message, InputStream)`.
///
/// Also normalizes paths by stripping the `/api` prefix.
pub async fn worker_request_to_message(req: &Request) -> Result<(Message, InputStream)> {
    let method = req.method().to_string();
    let url = req.url()?;
    let raw_path = url.path().to_string();
    let query = url.query().unwrap_or("").to_string();

    // Normalize path — strip /api prefix
    let mut path = raw_path.clone();
    if path.starts_with("/api") {
        path = path[4..].to_string();
        if path.is_empty() {
            path = "/".to_string();
        }
    }

    // Read body (with size limit). A read error here would otherwise be
    // swallowed and turned into an empty body, silently corrupting POST/PUT.
    const MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10 MB
    let mut req_clone = req.clone()?;
    let body = req_clone.bytes().await?;
    if body.len() > MAX_BODY_SIZE {
        return Err("request body too large".into());
    }

    // Extract remote address
    let remote_addr = req
        .headers()
        .get("cf-connecting-ip")
        .ok()
        .flatten()
        .or_else(|| req.headers().get("x-forwarded-for").ok().flatten())
        .unwrap_or_else(|| "unknown".to_string());

    let mut msg = Message::new(format!("{}:{}", method, path));

    // HTTP-specific meta
    msg.set_meta("http.method", method.clone());
    msg.set_meta("http.path", raw_path);
    msg.set_meta("http.raw_query", query.clone());
    msg.set_meta("http.remote_addr", remote_addr.clone());

    let content_type = req
        .headers()
        .get("content-type")
        .ok()
        .flatten()
        .unwrap_or_default();
    msg.set_meta("http.content_type", content_type.clone());

    let host = req.headers().get("host").ok().flatten().unwrap_or_default();
    msg.set_meta("http.host", host);

    // Normalized request meta (using the rewritten path)
    let action = match method.as_str() {
        "GET" | "HEAD" => "retrieve",
        "POST" => "create",
        "PUT" | "PATCH" => "update",
        "DELETE" => "delete",
        _ => "execute",
    };
    msg.set_meta(META_REQ_ACTION, action);
    msg.set_meta(META_REQ_RESOURCE, path);
    msg.set_meta(META_REQ_CLIENT_IP, remote_addr);
    msg.set_meta(META_REQ_CONTENT_TYPE, content_type);

    // Copy headers to meta
    for (name, value) in req.headers() {
        msg.set_meta(format!("http.header.{}", name), value);
    }

    // Parse query params
    if !query.is_empty() {
        for pair in query.split('&') {
            let mut parts = pair.splitn(2, '=');
            if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
                let decoded = urlencoding_decode(val);
                msg.set_meta(format!("http.query.{}", key), decoded.clone());
                msg.set_meta(format!("{}{}", META_REQ_QUERY_PREFIX, key), decoded);
            }
        }
    }

    Ok((msg, InputStream::from_bytes(body)))
}

// ---------------------------------------------------------------------------
// Response conversion
// ---------------------------------------------------------------------------

/// Convert a WAFER `OutputStream` into a Cloudflare Worker `Response`.
///
/// Buffers the full stream, mirroring the axum + web_sys adapters. Handles
/// all four `TerminalNotResponse` branches.
pub async fn output_to_response(output: OutputStream) -> Result<Response> {
    match output.collect_buffered().await {
        Ok(buf) => {
            let status = get_status(&buf.meta, 200);

            let resp = Response::from_bytes(buf.body)?;
            let mut resp = resp.with_status(status);
            let headers = resp.headers_mut();

            // Apply response headers from meta
            apply_meta_headers(headers, &buf.meta)?;

            // Default Content-Type: application/json if unset
            let has_ct = meta_contains_ct(&buf.meta);
            if !has_ct {
                headers.set("Content-Type", "application/json")?;
            }

            Ok(resp)
        }

        Err(TerminalNotResponse::Error(err)) => {
            let err_meta = err.meta.clone();
            let status = get_error_status(Some(&err), &err_meta);

            let body = serde_json::json!({
                "error": format!("{:?}", err.code),
                "message": err.message,
            })
            .to_string();

            let resp = Response::ok(body)?;
            let mut resp = resp.with_status(status);
            let headers = resp.headers_mut();
            apply_meta_headers(headers, &err_meta)?;
            headers.set("Content-Type", "application/json")?;

            Ok(resp)
        }

        Err(TerminalNotResponse::Drop) => {
            let resp = Response::empty()?;
            Ok(resp.with_status(204))
        }

        Err(TerminalNotResponse::Continue(msg)) => {
            // At the top-level HTTP boundary, Continue is treated as an empty 200
            // with the trailing message meta forwarded as headers.
            let resp = Response::empty()?;
            let mut resp = resp.with_status(200);
            let headers = resp.headers_mut();
            apply_meta_headers(headers, &msg.meta)?;
            headers.set("Content-Type", "application/json")?;
            Ok(resp)
        }

        Err(TerminalNotResponse::Malformed) => {
            let resp = Response::ok("{\"error\":\"stream ended without terminal event\"}")?;
            let mut resp = resp.with_status(500);
            resp.headers_mut().set("Content-Type", "application/json")?;
            Ok(resp)
        }

        Err(TerminalNotResponse::Halt(buf)) => {
            // Halt is a successful short-circuit terminal — body+meta ARE
            // the response. Same wire shape as the Ok arm.
            let status = get_status(&buf.meta, 200);

            let resp = Response::from_bytes(buf.body)?;
            let mut resp = resp.with_status(status);
            let headers = resp.headers_mut();

            apply_meta_headers(headers, &buf.meta)?;

            let has_ct = meta_contains_ct(&buf.meta);
            if !has_ct {
                headers.set("Content-Type", "application/json")?;
            }

            Ok(resp)
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn meta_get(meta: &[MetaEntry], key: &str) -> Option<String> {
    meta.iter().find(|e| e.key == key).map(|e| e.value.clone())
}

fn meta_contains_ct(meta: &[MetaEntry]) -> bool {
    meta.iter()
        .any(|e| e.key == META_RESP_CONTENT_TYPE || e.key == "Content-Type")
}

fn get_status(meta: &[MetaEntry], default: u16) -> u16 {
    meta_get(meta, META_RESP_STATUS)
        .or_else(|| meta_get(meta, "http.status"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn get_error_status(error: Option<&WaferError>, meta: &[MetaEntry]) -> u16 {
    let from_meta = get_status(meta, 0);
    if from_meta > 0 {
        return from_meta;
    }
    if let Some(err) = error {
        return error_code_to_http_status(&err.code);
    }
    500
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

/// Apply response headers from meta to the Worker response.
///
/// Recognizes:
/// - `resp.cookie.*` / `resp.set_cookie.*` / `http.resp.set-cookie.*` → Set-Cookie
/// - `resp.header.*` / `http.resp.header.*` → custom response header
/// - `resp.content_type` / `Content-Type` → Content-Type
fn apply_meta_headers(headers: &mut worker::Headers, meta: &[MetaEntry]) -> Result<()> {
    for entry in meta {
        let k = entry.key.as_str();
        let v = &entry.value;
        // Status is handled separately; skip.
        if k == META_RESP_STATUS || k == "http.status" {
            continue;
        }
        if k.starts_with(META_RESP_COOKIE_PREFIX)
            || k.starts_with("resp.set_cookie.")
            || k.starts_with("http.resp.set-cookie.")
        {
            headers.append("Set-Cookie", v)?;
        } else if let Some(name) = k
            .strip_prefix(META_RESP_HEADER_PREFIX)
            .or_else(|| k.strip_prefix("http.resp.header."))
        {
            headers.set(name, v)?;
        } else if k == META_RESP_CONTENT_TYPE || k == "Content-Type" {
            headers.set("Content-Type", v)?;
        }
    }
    Ok(())
}
