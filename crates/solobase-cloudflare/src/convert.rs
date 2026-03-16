//! HTTP ↔ Message conversion for Cloudflare Workers.
//!
//! This mirrors the logic in `wafer-core/src/blocks/http/mod.rs` but uses
//! Cloudflare Worker `Request`/`Response` types instead of axum.

use wafer_run::meta::*;
use wafer_run::types::*;
use worker::{Request, Response, Result};

/// Convert a Cloudflare Worker Request into a WAFER Message.
pub async fn worker_request_to_message(req: &Request) -> Result<Message> {
    let method = req.method().to_string();
    let url = req.url()?;
    let path = url.path().to_string();
    let query = url.query().unwrap_or("").to_string();

    // Read body
    let mut req_clone = req.clone()?;
    let body = req_clone
        .bytes()
        .await
        .unwrap_or_default();

    // Extract remote address
    let remote_addr = req
        .headers()
        .get("cf-connecting-ip")
        .ok()
        .flatten()
        .or_else(|| req.headers().get("x-forwarded-for").ok().flatten())
        .unwrap_or_else(|| "unknown".to_string());

    let mut meta = Vec::new();

    // HTTP-specific meta
    meta.push(MetaEntry { key: "http.method".into(), value: method.clone() });
    meta.push(MetaEntry { key: "http.path".into(), value: path.clone() });
    meta.push(MetaEntry { key: "http.raw_query".into(), value: query.clone() });
    meta.push(MetaEntry { key: "http.remote_addr".into(), value: remote_addr.clone() });

    let content_type = req
        .headers()
        .get("content-type")
        .ok()
        .flatten()
        .unwrap_or_default();
    meta.push(MetaEntry { key: "http.content_type".into(), value: content_type.clone() });

    let host = req
        .headers()
        .get("host")
        .ok()
        .flatten()
        .unwrap_or_default();
    meta.push(MetaEntry { key: "http.host".into(), value: host });

    // Normalized request meta
    let action = match method.as_str() {
        "GET" | "HEAD" => "retrieve",
        "POST" => "create",
        "PUT" | "PATCH" => "update",
        "DELETE" => "delete",
        _ => "execute",
    };
    meta.push(MetaEntry { key: META_REQ_ACTION.into(), value: action.into() });
    meta.push(MetaEntry { key: META_REQ_RESOURCE.into(), value: path.clone() });
    meta.push(MetaEntry { key: META_REQ_CLIENT_IP.into(), value: remote_addr });
    meta.push(MetaEntry { key: META_REQ_CONTENT_TYPE.into(), value: content_type });

    // Copy headers to meta
    for (name, value) in req.headers() {
        meta.push(MetaEntry { key: format!("http.header.{}", name), value });
    }

    // Parse query params
    if !query.is_empty() {
        for pair in query.split('&') {
            let mut parts = pair.splitn(2, '=');
            if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
                let decoded = urlencoding_decode(val);
                meta.push(MetaEntry { key: format!("http.query.{}", key), value: decoded.clone() });
                meta.push(MetaEntry { key: format!("{}{}", META_REQ_QUERY_PREFIX, key), value: decoded });
            }
        }
    }

    Ok(Message {
        kind: format!("{}:{}", method, path),
        data: body,
        meta,
    })
}

/// Convert a WAFER Result_ into a Cloudflare Worker Response.
pub fn wafer_result_to_worker_response(result: Result_) -> Result<Response> {
    match result.action {
        Action::Respond => {
            let (body, resp_meta) = match result.response {
                Some(r) => (r.data, r.meta),
                None => (Vec::new(), Vec::new()),
            };

            let status = get_status(&resp_meta, 200);

            let content_type = meta_get(&resp_meta, META_RESP_CONTENT_TYPE)
                .or_else(|| meta_get(&resp_meta, "Content-Type"))
                .unwrap_or_else(|| "application/json".to_string());

            let resp = Response::from_bytes(body)?;
            let mut resp = resp.with_status(status);
            let headers = resp.headers_mut();
            headers.set("Content-Type", &content_type)?;

            // Apply response headers from meta
            apply_meta_headers(headers, &resp_meta)?;
            if let Some(ref msg) = result.message {
                apply_meta_headers(headers, &msg.meta)?;
            }

            Ok(resp)
        }

        Action::Error => {
            let err_meta = result
                .error
                .as_ref()
                .map(|e| &e.meta)
                .unwrap_or(&Vec::new())
                .clone();

            let status = get_error_status(result.error.as_ref(), &err_meta);

            let body = if let Some(ref err) = result.error {
                serde_json::json!({
                    "error": format!("{:?}", err.code),
                    "message": err.message,
                })
                .to_string()
            } else {
                "{}".to_string()
            };

            let resp = Response::ok(body)?;
            let mut resp = resp.with_status(status);
            let headers = resp.headers_mut();
            headers.set("Content-Type", "application/json")?;

            apply_meta_headers(headers, &err_meta)?;
            if let Some(ref msg) = result.message {
                apply_meta_headers(headers, &msg.meta)?;
            }

            Ok(resp)
        }

        Action::Drop => {
            let resp = Response::empty()?;
            let mut resp = resp.with_status(204);
            if let Some(ref msg) = result.message {
                let headers = resp.headers_mut();
                apply_meta_headers(headers, &msg.meta)?;
            }
            Ok(resp)
        }

        Action::Continue => {
            let body = result.message.as_ref().map(|m| m.data.clone()).unwrap_or_default();
            let mut resp = Response::from_bytes(body)?;
            let headers = resp.headers_mut();
            headers.set("Content-Type", "application/json")?;
            if let Some(ref msg) = result.message {
                apply_meta_headers(headers, &msg.meta)?;
            }
            Ok(resp)
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn meta_get(meta: &[MetaEntry], key: &str) -> Option<String> {
    meta.iter()
        .find(|e| e.key == key)
        .map(|e| e.value.clone())
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

fn apply_meta_headers(
    headers: &mut worker::Headers,
    meta: &[MetaEntry],
) -> Result<()> {
    for entry in meta {
        if entry.key.starts_with(META_RESP_COOKIE_PREFIX) || entry.key.starts_with("http.resp.set-cookie.") {
            headers.append("Set-Cookie", &entry.value)?;
        } else if let Some(name) = entry.key.strip_prefix(META_RESP_HEADER_PREFIX)
            .or_else(|| entry.key.strip_prefix("http.resp.header."))
        {
            headers.set(name, &entry.value)?;
        }
    }
    Ok(())
}

fn urlencoding_decode(s: &str) -> String {
    let mut bytes = Vec::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'+' {
            bytes.push(b' ');
        } else if b == b'%' {
            let h1 = chars.next().and_then(|c| (c as char).to_digit(16));
            let h2 = chars.next().and_then(|c| (c as char).to_digit(16));
            if let (Some(h1), Some(h2)) = (h1, h2) {
                bytes.push((h1 * 16 + h2) as u8);
            }
        } else {
            bytes.push(b);
        }
    }
    String::from_utf8(bytes).unwrap_or_else(|_| s.to_string())
}
