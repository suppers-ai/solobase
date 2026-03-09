//! HTTP ↔ Message conversion for Cloudflare Workers.
//!
//! This mirrors the logic in `wafer-core/src/blocks/http/mod.rs` but uses
//! Cloudflare Worker `Request`/`Response` types instead of axum.

use std::collections::HashMap;

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

    let mut meta = HashMap::new();

    // HTTP-specific meta
    meta.insert("http.method".to_string(), method.clone());
    meta.insert("http.path".to_string(), path.clone());
    meta.insert("http.raw_query".to_string(), query.clone());
    meta.insert("http.remote_addr".to_string(), remote_addr.clone());

    let content_type = req
        .headers()
        .get("content-type")
        .ok()
        .flatten()
        .unwrap_or_default();
    meta.insert("http.content_type".to_string(), content_type.clone());

    let host = req
        .headers()
        .get("host")
        .ok()
        .flatten()
        .unwrap_or_default();
    meta.insert("http.host".to_string(), host);

    // Normalized request meta
    let action = match method.as_str() {
        "GET" | "HEAD" => "retrieve",
        "POST" => "create",
        "PUT" | "PATCH" => "update",
        "DELETE" => "delete",
        _ => "execute",
    };
    meta.insert(META_REQ_ACTION.to_string(), action.to_string());
    meta.insert(META_REQ_RESOURCE.to_string(), path.clone());
    meta.insert(META_REQ_CLIENT_IP.to_string(), remote_addr);
    meta.insert(META_REQ_CONTENT_TYPE.to_string(), content_type);

    // Copy headers to meta
    for (name, value) in req.headers() {
        meta.insert(format!("http.header.{}", name), value);
    }

    // Parse query params
    if !query.is_empty() {
        for pair in query.split('&') {
            let mut parts = pair.splitn(2, '=');
            if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
                let decoded = urlencoding_decode(val);
                meta.insert(format!("http.query.{}", key), decoded.clone());
                meta.insert(format!("{}{}", META_REQ_QUERY_PREFIX, key), decoded);
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
                None => (Vec::new(), HashMap::new()),
            };

            let status = get_status(&resp_meta, 200);

            let content_type = resp_meta
                .get(META_RESP_CONTENT_TYPE)
                .or_else(|| resp_meta.get("Content-Type"))
                .cloned()
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
            let empty_meta = HashMap::new();
            let err_meta = result
                .error
                .as_ref()
                .map(|e| &e.meta)
                .unwrap_or(&empty_meta);

            let status = get_error_status(result.error.as_ref(), err_meta);

            let body = if let Some(ref err) = result.error {
                serde_json::json!({
                    "error": err.code,
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

            apply_meta_headers(headers, err_meta)?;
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

fn get_status(meta: &HashMap<String, String>, default: u16) -> u16 {
    meta.get(META_RESP_STATUS)
        .or_else(|| meta.get("http.status"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn get_error_status(error: Option<&WaferError>, meta: &HashMap<String, String>) -> u16 {
    let from_meta = get_status(meta, 0);
    if from_meta > 0 {
        return from_meta;
    }
    if let Some(err) = error {
        return error_code_to_http_status(&err.code);
    }
    500
}

fn error_code_to_http_status(code: &str) -> u16 {
    match code {
        "ok" => 200,
        "cancelled" => 499,
        "invalid_argument" => 400,
        "deadline_exceeded" => 504,
        "not_found" => 404,
        "already_exists" => 409,
        "permission_denied" => 403,
        "resource_exhausted" => 429,
        "failed_precondition" => 412,
        "aborted" => 409,
        "out_of_range" => 400,
        "unimplemented" => 501,
        "internal" => 500,
        "unavailable" => 503,
        "data_loss" => 500,
        "unauthenticated" => 401,
        _ => 500,
    }
}

fn apply_meta_headers(
    headers: &mut worker::Headers,
    meta: &HashMap<String, String>,
) -> Result<()> {
    for (k, v) in meta {
        if k.starts_with(META_RESP_COOKIE_PREFIX) || k.starts_with("http.resp.set-cookie.") {
            headers.append("Set-Cookie", v)?;
        } else if let Some(name) = k.strip_prefix(META_RESP_HEADER_PREFIX)
            .or_else(|| k.strip_prefix("http.resp.header."))
        {
            headers.set(name, v)?;
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
