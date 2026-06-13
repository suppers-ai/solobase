//! Tests for configurable routing via `ExtraRoute` + `RouteAccess`.
//!
//! Downstream projects register their own routes on `SolobaseBuilder` via
//! `add_route()`. Built-in `ROUTES` take priority; extra routes only match
//! when no built-in prefix matches. Auth gating for extras is declared via
//! `RouteAccess` (Public / Authenticated / Admin).
//!
//! These tests drive `routing::route_to_block` directly with mock trait
//! impls for `Context` / `FeatureConfig`, exercising the four scenarios in
//! the task spec:
//!
//! 1. Built-in route wins when prefix collides.
//! 2. Public extra dispatches without auth.
//! 3. Authenticated extra rejects empty user_id.
//! 4. Authenticated extra dispatches with user_id set.
//! 5. Admin extra rejects non-admin.
//! 6. Unmatched path falls through to 404.

use std::sync::{Arc, Mutex};

use solobase_core::{
    features::FeatureConfig,
    routing::{self, ExtraRoute, RouteAccess},
};
use wafer_block::http_codec;
use wafer_run::{
    context::Context, streams::output::TerminalNotResponse, InputStream, Message, OutputStream,
    META_AUTH_USER_ID, META_REQ_RESOURCE, META_RESP_STATUS,
};

// ---------------------------------------------------------------------------
// Mock Context — records which block was called and returns a minimal OK response.
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct RecordingContext {
    calls: Arc<Mutex<Vec<String>>>,
}

impl RecordingContext {
    fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl Context for RecordingContext {
    async fn call_block(
        &self,
        block_name: &str,
        _msg: Message,
        _input: InputStream,
    ) -> OutputStream {
        self.calls.lock().unwrap().push(block_name.to_string());
        // Return a simple 200 response so the caller sees a "dispatch happened" signal.
        OutputStream::respond_with_meta(
            b"ok".to_vec(),
            vec![wafer_run::MetaEntry {
                key: META_RESP_STATUS.into(),
                value: "200".into(),
            }],
        )
    }

    fn is_cancelled(&self) -> bool {
        false
    }

    fn config_get(&self, _key: &str) -> Option<&str> {
        None
    }

    fn clone_arc(&self) -> Arc<dyn Context> {
        Arc::new(self.clone())
    }
}

// ---------------------------------------------------------------------------
// Feature config — always enabled.
// ---------------------------------------------------------------------------

struct AllEnabled;

impl FeatureConfig for AllEnabled {
    fn is_block_enabled(&self, _name: &str) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_msg(path: &str) -> Message {
    let mut msg = Message::new("http.request");
    msg.set_meta(META_REQ_RESOURCE, path);
    msg
}

fn make_msg_with_user(path: &str, user_id: &str) -> Message {
    let mut msg = make_msg(path);
    msg.set_meta(META_AUTH_USER_ID, user_id);
    msg
}

fn make_msg_with_admin(path: &str, user_id: &str) -> Message {
    let mut msg = make_msg_with_user(path, user_id);
    msg.set_meta("auth.user_roles", "admin");
    msg
}

/// Collect the stream and return the "HTTP-ish" status code via the
/// canonical `wafer_block::http_codec` status resolution (explicit
/// `resp.status` override, else the `ErrorCode`-derived status for Error
/// terminals — NotFound → 404, PermissionDenied → 403, Unauthenticated → 401).
async fn response_status(stream: OutputStream) -> i64 {
    match stream.collect_buffered().await {
        Ok(buf) => i64::from(http_codec::resolve_status(&buf.meta, 200)),
        Err(TerminalNotResponse::Error(err)) => i64::from(http_codec::resolve_error_status(&err)),
        Err(other) => panic!("unexpected terminal: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn built_in_route_wins_over_extra_with_same_prefix() {
    let ctx = RecordingContext::new();
    let features = AllEnabled;

    // Extra route tries to steal /b/auth/ — must lose to the built-in.
    let extras = vec![ExtraRoute {
        prefix: "/b/auth/".into(),
        access: RouteAccess::Public,
        block_name: "gizza-ai/stolen-auth".into(),
    }];

    let msg = make_msg("/b/auth/login");
    let input = InputStream::empty();
    let stream = routing::route_to_block(&ctx, msg, input, &features, &extras).await;
    let _ = stream.collect_buffered().await;

    let calls = ctx.calls();
    assert_eq!(calls.len(), 1, "exactly one dispatch should have happened");
    assert_eq!(
        calls[0], "suppers-ai/auth-ui",
        "built-in auth-ui route must win over extra with same prefix"
    );
}

#[tokio::test]
async fn public_extra_route_dispatches_without_auth() {
    let ctx = RecordingContext::new();
    let features = AllEnabled;

    let extras = vec![ExtraRoute {
        prefix: "/b/chat/".into(),
        access: RouteAccess::Public,
        block_name: "gizza-ai/chat".into(),
    }];

    // No user_id set on the message — Public access should allow it through.
    let msg = make_msg("/b/chat/hello");
    let input = InputStream::empty();
    let stream = routing::route_to_block(&ctx, msg, input, &features, &extras).await;
    let status = response_status(stream).await;
    assert_eq!(status, 200);

    let calls = ctx.calls();
    assert_eq!(calls, vec!["gizza-ai/chat".to_string()]);
}

#[tokio::test]
async fn authenticated_extra_route_forbids_empty_user_id() {
    let ctx = RecordingContext::new();
    let features = AllEnabled;

    let extras = vec![ExtraRoute {
        prefix: "/b/chat/".into(),
        access: RouteAccess::Authenticated,
        block_name: "gizza-ai/chat".into(),
    }];

    let msg = make_msg("/b/chat/hello"); // no user_id
    let input = InputStream::empty();
    let stream = routing::route_to_block(&ctx, msg, input, &features, &extras).await;
    let status = response_status(stream).await;
    assert_eq!(status, 403, "Authenticated + empty user_id should be 403");

    assert!(
        ctx.calls().is_empty(),
        "dispatch must NOT happen when forbidden"
    );
}

#[tokio::test]
async fn authenticated_extra_route_allows_user() {
    let ctx = RecordingContext::new();
    let features = AllEnabled;

    let extras = vec![ExtraRoute {
        prefix: "/b/chat/".into(),
        access: RouteAccess::Authenticated,
        block_name: "gizza-ai/chat".into(),
    }];

    let msg = make_msg_with_user("/b/chat/hello", "user-123");
    let input = InputStream::empty();
    let stream = routing::route_to_block(&ctx, msg, input, &features, &extras).await;
    let status = response_status(stream).await;
    assert_eq!(status, 200);

    assert_eq!(ctx.calls(), vec!["gizza-ai/chat".to_string()]);
}

#[tokio::test]
async fn admin_extra_route_forbids_non_admin() {
    let ctx = RecordingContext::new();
    let features = AllEnabled;

    let extras = vec![ExtraRoute {
        prefix: "/b/gizza-admin/".into(),
        access: RouteAccess::Admin,
        block_name: "gizza-ai/admin".into(),
    }];

    // User is authenticated but lacks the admin role.
    let msg = make_msg_with_user("/b/gizza-admin/dash", "user-123");
    let input = InputStream::empty();
    let stream = routing::route_to_block(&ctx, msg, input, &features, &extras).await;
    let status = response_status(stream).await;
    assert_eq!(status, 403, "Admin access + non-admin user should be 403");

    assert!(ctx.calls().is_empty());
}

#[tokio::test]
async fn admin_extra_route_allows_admin() {
    let ctx = RecordingContext::new();
    let features = AllEnabled;

    let extras = vec![ExtraRoute {
        prefix: "/b/gizza-admin/".into(),
        access: RouteAccess::Admin,
        block_name: "gizza-ai/admin".into(),
    }];

    let msg = make_msg_with_admin("/b/gizza-admin/dash", "admin-1");
    let input = InputStream::empty();
    let stream = routing::route_to_block(&ctx, msg, input, &features, &extras).await;
    let status = response_status(stream).await;
    assert_eq!(status, 200);

    assert_eq!(ctx.calls(), vec!["gizza-ai/admin".to_string()]);
}

#[tokio::test]
async fn unmatched_path_falls_through_to_not_found() {
    let ctx = RecordingContext::new();
    let features = AllEnabled;

    let extras = vec![ExtraRoute {
        prefix: "/b/chat/".into(),
        access: RouteAccess::Public,
        block_name: "gizza-ai/chat".into(),
    }];

    let msg = make_msg("/some/other/path");
    let input = InputStream::empty();
    let stream = routing::route_to_block(&ctx, msg, input, &features, &extras).await;
    let status = response_status(stream).await;
    assert_eq!(status, 404);

    assert!(ctx.calls().is_empty());
}
