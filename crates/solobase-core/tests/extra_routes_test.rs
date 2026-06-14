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
    // Default to a GET (`retrieve`) action so the per-endpoint matcher can
    // resolve declared `AuthLevel`s; POST/DELETE cases override `req.action`.
    msg.set_meta("req.action", "retrieve");
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
    let stream = routing::route_to_block(&ctx, msg, input, &features, &[], &extras).await;
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
    let stream = routing::route_to_block(&ctx, msg, input, &features, &[], &extras).await;
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
    let stream = routing::route_to_block(&ctx, msg, input, &features, &[], &extras).await;
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
    let stream = routing::route_to_block(&ctx, msg, input, &features, &[], &extras).await;
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
    let stream = routing::route_to_block(&ctx, msg, input, &features, &[], &extras).await;
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
    let stream = routing::route_to_block(&ctx, msg, input, &features, &[], &extras).await;
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
    let stream = routing::route_to_block(&ctx, msg, input, &features, &[], &extras).await;
    let status = response_status(stream).await;
    assert_eq!(status, 404);

    assert!(ctx.calls().is_empty());
}

// ---------------------------------------------------------------------------
// Central per-endpoint AuthLevel enforcement (S4-U).
//
// These pin the new behavior: the router enforces the *declared* endpoint
// `AuthLevel` (from `BlockInfo::endpoints`) before dispatch, taking the
// stricter of the coarse prefix tier and the declared level. This is what
// lets blocks drop their per-handler `is_admin`/`user_id` preambles — and it
// is the fix for the #1 regression risk (legalpages admin protection that
// used to rest only on route-table ordering).
// ---------------------------------------------------------------------------

use wafer_run::{AuthLevel, BlockEndpoint, BlockInfo};

/// A `block_infos` slice declaring the legalpages admin/api endpoints as
/// `Admin` and the public terms page as `Public` — exactly as the block's
/// `info()` does.
fn legalpages_infos() -> Vec<BlockInfo> {
    vec![
        BlockInfo::new("suppers-ai/legalpages", "0.0.1", "http-handler@v1", "legal").endpoints(
            vec![
                BlockEndpoint::get("/b/legalpages/terms").auth(AuthLevel::Public),
                BlockEndpoint::get("/b/legalpages/admin").auth(AuthLevel::Admin),
                BlockEndpoint::patch("/b/legalpages/api/documents/{id}").auth(AuthLevel::Admin),
            ],
        ),
    ]
}

#[tokio::test]
async fn declared_admin_endpoint_rejects_non_admin_even_when_prefix_is_public() {
    let ctx = RecordingContext::new();
    let features = AllEnabled;
    let infos = legalpages_infos();

    // `/b/legalpages/admin` is declared Admin. An authenticated non-admin must
    // be rejected centrally BEFORE the block runs — proving the admin gate no
    // longer rests on route-table ordering.
    let msg = make_msg_with_user("/b/legalpages/admin", "user-1");
    let stream =
        routing::route_to_block(&ctx, msg, InputStream::empty(), &features, &infos, &[]).await;
    assert_eq!(response_status(stream).await, 403);
    assert!(ctx.calls().is_empty(), "must not dispatch to the block");
}

#[tokio::test]
async fn declared_admin_endpoint_allows_admin() {
    let ctx = RecordingContext::new();
    let features = AllEnabled;
    let infos = legalpages_infos();

    let msg = make_msg_with_admin("/b/legalpages/admin", "admin-1");
    let stream =
        routing::route_to_block(&ctx, msg, InputStream::empty(), &features, &infos, &[]).await;
    assert_eq!(response_status(stream).await, 200);
    assert_eq!(ctx.calls(), vec!["suppers-ai/legalpages".to_string()]);
}

#[tokio::test]
async fn declared_admin_endpoint_with_path_param_enforced() {
    let ctx = RecordingContext::new();
    let features = AllEnabled;
    let infos = legalpages_infos();

    // PATCH /b/legalpages/api/documents/{id} is Admin — a non-admin is 403'd
    // even though the `{id}` segment is dynamic.
    let mut msg = make_msg_with_user("/b/legalpages/api/documents/doc-7", "user-1");
    msg.set_meta("req.action", "update");
    let stream =
        routing::route_to_block(&ctx, msg, InputStream::empty(), &features, &infos, &[]).await;
    assert_eq!(response_status(stream).await, 403);
    assert!(ctx.calls().is_empty());
}

#[tokio::test]
async fn public_declared_endpoint_passes_without_auth() {
    let ctx = RecordingContext::new();
    let features = AllEnabled;
    let infos = legalpages_infos();

    // `/b/legalpages/terms` is declared Public — anonymous request dispatches.
    let msg = make_msg("/b/legalpages/terms");
    let stream =
        routing::route_to_block(&ctx, msg, InputStream::empty(), &features, &infos, &[]).await;
    assert_eq!(response_status(stream).await, 200);
    assert_eq!(ctx.calls(), vec!["suppers-ai/legalpages".to_string()]);
}

#[tokio::test]
async fn undeclared_path_falls_back_to_prefix_tier() {
    let ctx = RecordingContext::new();
    let features = AllEnabled;
    let infos = legalpages_infos();

    // `/b/legalpages/api/documents` (no `{id}`) is NOT in the declared
    // endpoint list above. The coarse prefix route for `/b/legalpages/api` is
    // Admin (the backstop), so a non-admin is still rejected — an undeclared
    // path can never be LESS protected than its prefix tier.
    let msg = make_msg_with_user("/b/legalpages/api/documents", "user-1");
    let stream =
        routing::route_to_block(&ctx, msg, InputStream::empty(), &features, &infos, &[]).await;
    assert_eq!(response_status(stream).await, 403);
    assert!(ctx.calls().is_empty());
}

/// `block_infos` mirroring the llm block's mixed-tier declarations: chat is
/// `Authenticated`, the provider/model-admin UI + CRUD are `Admin`. The llm
/// prefix route (`/b/llm`) is `Public`, so these prove the declared level is
/// what gates — the source of the deleted per-handler `is_admin` re-checks.
fn llm_infos() -> Vec<BlockInfo> {
    vec![
        BlockInfo::new("suppers-ai/llm", "0.0.1", "http-handler@v1", "llm").endpoints(vec![
            BlockEndpoint::post("/b/llm/api/chat").auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/llm/providers").auth(AuthLevel::Admin),
            BlockEndpoint::get("/b/llm/models").auth(AuthLevel::Admin),
            BlockEndpoint::post("/b/llm/api/providers").auth(AuthLevel::Admin),
            BlockEndpoint::delete("/b/llm/api/providers/{id}").auth(AuthLevel::Admin),
            BlockEndpoint::post("/b/llm/api/models/{backend_id}/{model_id}/load")
                .auth(AuthLevel::Admin),
        ]),
    ]
}

#[tokio::test]
async fn llm_admin_ui_rejects_non_admin() {
    let ctx = RecordingContext::new();
    let infos = llm_infos();
    for path in ["/b/llm/providers", "/b/llm/models"] {
        let msg = make_msg_with_user(path, "user-1");
        let stream =
            routing::route_to_block(&ctx, msg, InputStream::empty(), &AllEnabled, &infos, &[]).await;
        assert_eq!(response_status(stream).await, 403, "{path} must reject non-admin");
    }
    assert!(ctx.calls().is_empty());
}

#[tokio::test]
async fn llm_admin_provider_crud_rejects_non_admin() {
    let ctx = RecordingContext::new();
    let infos = llm_infos();

    // POST /b/llm/api/providers — declared Admin.
    let mut create = make_msg_with_user("/b/llm/api/providers", "user-1");
    create.set_meta("req.action", "create");
    let s1 =
        routing::route_to_block(&ctx, create, InputStream::empty(), &AllEnabled, &infos, &[]).await;
    assert_eq!(response_status(s1).await, 403);

    // DELETE /b/llm/api/providers/{id} — declared Admin, dynamic segment.
    let mut del = make_msg_with_user("/b/llm/api/providers/p-9", "user-1");
    del.set_meta("req.action", "delete");
    let s2 = routing::route_to_block(&ctx, del, InputStream::empty(), &AllEnabled, &infos, &[]).await;
    assert_eq!(response_status(s2).await, 403);

    // POST /b/llm/api/models/{backend}/{model}/load — declared Admin.
    let mut load = make_msg_with_user("/b/llm/api/models/ollama/llama3/load", "user-1");
    load.set_meta("req.action", "create");
    let s3 =
        routing::route_to_block(&ctx, load, InputStream::empty(), &AllEnabled, &infos, &[]).await;
    assert_eq!(response_status(s3).await, 403);

    assert!(ctx.calls().is_empty());
}

#[tokio::test]
async fn auth_ui_admin_settings_rejects_non_admin() {
    // `/b/auth/admin/settings` is declared `Admin` while the auth-ui prefix is
    // Public — so the declared level is the sole gate (the deleted inline
    // `is_admin` check). A non-admin must be 403'd before dispatch.
    let ctx = RecordingContext::new();
    let infos = vec![BlockInfo::new(
        "suppers-ai/auth-ui",
        "0.0.1",
        "http-handler@v1",
        "auth-ui",
    )
    .endpoints(vec![
        BlockEndpoint::get("/b/auth/admin/settings").auth(AuthLevel::Admin),
        BlockEndpoint::get("/b/auth/login").auth(AuthLevel::Public),
    ])];

    let msg = make_msg_with_user("/b/auth/admin/settings", "user-1");
    let s = routing::route_to_block(&ctx, msg, InputStream::empty(), &AllEnabled, &infos, &[]).await;
    assert_eq!(response_status(s).await, 403);
    assert!(ctx.calls().is_empty());

    // The public login page still dispatches anonymously.
    let ctx2 = RecordingContext::new();
    let login = make_msg("/b/auth/login");
    let s2 =
        routing::route_to_block(&ctx2, login, InputStream::empty(), &AllEnabled, &infos, &[]).await;
    assert_eq!(response_status(s2).await, 200);
    assert_eq!(ctx2.calls(), vec!["suppers-ai/auth-ui".to_string()]);
}

#[tokio::test]
async fn llm_chat_allows_authenticated_non_admin() {
    let ctx = RecordingContext::new();
    let infos = llm_infos();

    // POST /b/llm/api/chat — declared Authenticated; a plain user passes and
    // dispatches (proving the chat endpoint is NOT swept up by the admin gate).
    let mut chat = make_msg_with_user("/b/llm/api/chat", "user-1");
    chat.set_meta("req.action", "create");
    let stream =
        routing::route_to_block(&ctx, chat, InputStream::empty(), &AllEnabled, &infos, &[]).await;
    assert_eq!(response_status(stream).await, 200);
    assert_eq!(ctx.calls(), vec!["suppers-ai/llm".to_string()]);
}
