//! HTTP request handlers for the Plan A2 auth endpoints.
//!
//! Each submodule implements one or more of the `/auth/*` routes declared in
//! the auth-block-design spec. The routes are mounted on
//! [`super::block::SolobaseAuthBlock::handle`]; handlers are pure in the
//! sense that they take `&dyn Context` + `&Message` (+ body / path params)
//! and return an [`wafer_run::OutputStream`].
//!
//! The `HttpReply` helper type is a thin builder that collapses to an
//! [`OutputStream`] so handlers can express HTTP-status + headers + body
//! without reaching for the full `ResponseBuilder` machinery every time.

pub mod login;

use wafer_run::{
    meta::{META_RESP_COOKIE_PREFIX, META_RESP_HEADER_PREFIX, META_RESP_STATUS},
    types::MetaEntry,
    OutputStream,
};

/// Normalised HTTP reply used by Plan A2 handlers.
///
/// `headers` carry both plain response headers and `Set-Cookie` values; the
/// name is matched case-insensitively so callers can write
/// `("Set-Cookie", ...)` and the conversion picks the right wire shape.
#[derive(Debug)]
pub struct HttpReply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpReply {
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    pub fn json_body(mut self, value: &serde_json::Value) -> Self {
        self.body = serde_json::to_vec(value).unwrap_or_default();
        // Only set Content-Type if the caller hasn't already set one.
        if !self
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Content-Type"))
        {
            self.headers
                .push(("Content-Type".into(), "application/json".into()));
        }
        self
    }

    pub fn raw_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }
}

/// Convert an [`HttpReply`] into the `OutputStream` wire form consumed by the
/// wafer runtime's HTTP adapter.
impl From<HttpReply> for OutputStream {
    fn from(reply: HttpReply) -> Self {
        let mut meta: Vec<MetaEntry> = Vec::with_capacity(reply.headers.len() + 1);
        meta.push(MetaEntry {
            key: META_RESP_STATUS.to_string(),
            value: reply.status.to_string(),
        });
        let mut cookie_idx: usize = 0;
        for (k, v) in reply.headers {
            if k.eq_ignore_ascii_case("Set-Cookie") {
                meta.push(MetaEntry {
                    key: format!("{}{}", META_RESP_COOKIE_PREFIX, cookie_idx),
                    value: v,
                });
                cookie_idx += 1;
            } else {
                meta.push(MetaEntry {
                    key: format!("{}{}", META_RESP_HEADER_PREFIX, k),
                    value: v,
                });
            }
        }
        OutputStream::respond_with_meta(reply.body, meta)
    }
}
