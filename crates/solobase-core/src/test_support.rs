//! Test infrastructure for solobase-core integration tests.
//!
//! [`TestContext`] wires a real in-memory SQLite database (via the production
//! `DatabaseBlock` + `SQLiteDatabaseService::open_in_memory()`) into a minimal
//! [`Context`] implementation so unit and integration tests can exercise the
//! full block client stack without running a server process.
//!
//! Additional capabilities (message helpers, auth state, extra block dispatch)
//! are added in subsequent tasks.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use wafer_run::{
    block::Block,
    context::Context,
    streams::output::{BufferedResponse, TerminalNotResponse},
    types::{ErrorCode, Message, WaferError},
    InputStream, OutputStream,
};

/// Minimal test context backed by a real in-memory SQLite database.
///
/// Routes `"wafer-run/database"` calls to the production `DatabaseBlock`.
/// Other named blocks can be registered via `blocks` (unused until Task 6).
pub struct TestContext {
    database_block: Arc<dyn Block>,
    /// Placeholder for config values — populated by Task 5 (`with_auth`).
    pub config: Arc<Mutex<HashMap<String, String>>>,
    /// Placeholder for dynamically registered blocks — populated by Task 6.
    pub blocks: Arc<Mutex<HashMap<String, Arc<dyn Block>>>>,
}

impl TestContext {
    /// Construct a `TestContext` with a fresh in-memory SQLite database.
    pub async fn new() -> Self {
        let svc = Arc::new(
            wafer_block_sqlite::service::SQLiteDatabaseService::open_in_memory()
                .expect("open in-memory sqlite"),
        );
        let database_block: Arc<dyn Block> = Arc::new(
            wafer_core::service_blocks::database::DatabaseBlock::new(svc),
        );

        Self {
            database_block,
            config: Arc::new(Mutex::new(HashMap::new())),
            blocks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Build a `TestContext` with the auth-block migrations applied.
    ///
    /// Convenience constructor for tests that need the
    /// `suppers_ai__auth__{users,orgs,sessions,provider_links,...}` schema
    /// in place — most repo and handler tests do.
    pub async fn with_auth() -> Self {
        let ctx = Self::new().await;
        crate::blocks::auth::migrations::apply(&ctx)
            .await
            .expect("apply auth migrations in test fixture");
        ctx
    }

    /// Register a block under `name`. Calls to `ctx.call_block(name, ...)`
    /// will route to this block's `handle()`.
    ///
    /// Used to wire up cross-block call tests — e.g. the dashboard handler
    /// in the auth block calls `"suppers-ai/userportal"` for the buttons
    /// list; tests register a real or fake `UserPortalBlock` so the call
    /// resolves.
    pub fn register_block(&mut self, name: &str, block: Arc<dyn Block>) {
        self.blocks
            .lock()
            .expect("blocks mutex poisoned")
            .insert(name.to_string(), block);
    }
}

#[async_trait::async_trait]
impl Context for TestContext {
    async fn call_block(&self, name: &str, msg: Message, input: InputStream) -> OutputStream {
        match name {
            "wafer-run/database" => self.database_block.handle(self, msg, input).await,
            other => {
                // Check the dynamically registered blocks map before giving up.
                let block = {
                    let guard = self.blocks.lock().expect("blocks mutex poisoned");
                    guard.get(other).cloned()
                };
                match block {
                    Some(b) => b.handle(self, msg, input).await,
                    None => OutputStream::error(WaferError::new(
                        ErrorCode::NOT_FOUND,
                        format!("block '{other}' not registered in TestContext"),
                    )),
                }
            }
        }
    }

    fn is_cancelled(&self) -> bool {
        false
    }

    fn config_get(&self, _key: &str) -> Option<&str> {
        // TODO(test_support): returning None here is a deliberate stub.
        // Borrowing a &str out of the Mutex<HashMap> would require holding the
        // guard for the lifetime of the return value, which the `Context` trait
        // signature (&self → Option<&str>) does not allow. Task 5 will either
        // leak a value into a thread-local or switch to a pre-computed snapshot
        // when `with_auth()` is called.
        None
    }
}

/// Build an anonymous request `Message`. No `auth.user_id` meta set.
pub fn anon_msg(action: &str, path: &str) -> Message {
    let mut m = Message::new("http.request");
    m.set_meta("req.action", action);
    m.set_meta("req.resource", path);
    m
}

/// Build an authenticated request `Message` for `user_id`. No admin role.
pub fn auth_msg(action: &str, path: &str, user_id: &str) -> Message {
    let mut m = anon_msg(action, path);
    m.set_meta("auth.user_id", user_id);
    m
}

/// Build an admin request `Message` (user_id `"admin_1"`, role `admin`).
pub fn admin_msg(action: &str, path: &str) -> Message {
    let mut m = auth_msg(action, path, "admin_1");
    m.set_meta("auth.user_roles", "admin");
    m
}

/// Drain an `OutputStream` to a `BufferedResponse`. Panics if the stream
/// terminates with anything other than `Complete`.
///
/// Tests should not see errors from handlers under test unless they're
/// explicitly asserting on error paths — use `output_is_error` for that.
pub async fn collect_or_panic(out: OutputStream) -> BufferedResponse {
    match out.collect_buffered().await {
        Ok(buf) => buf,
        Err(TerminalNotResponse::Error(e)) => {
            panic!("handler returned error: {} ({:?})", e.message, e.code)
        }
        Err(TerminalNotResponse::Drop) => panic!("handler dropped the request"),
        Err(TerminalNotResponse::Continue(_)) => panic!("handler returned Continue"),
        Err(TerminalNotResponse::Malformed) => panic!("handler returned malformed stream"),
    }
}

/// Read the HTTP status from an `OutputStream`. Defaults to 200 if the
/// handler didn't set a `resp.status` meta entry.
pub async fn output_status(out: OutputStream) -> u16 {
    let buf = collect_or_panic(out).await;
    buf.meta
        .iter()
        .find(|m| m.key == "resp.status")
        .and_then(|m| m.value.parse::<u16>().ok())
        .unwrap_or(200)
}

/// Read a named response header (e.g. `"Location"` for redirects).
/// The lookup is case-sensitive — pass the exact name handlers used in
/// `set_header(name, _)`.
pub async fn output_header(out: OutputStream, name: &str) -> Option<String> {
    let key = format!("resp.header.{name}");
    let buf = collect_or_panic(out).await;
    buf.meta
        .iter()
        .find(|m| m.key == key)
        .map(|m| m.value.clone())
}

/// Read the body as a UTF-8 string. Panics if the body is not valid UTF-8.
pub async fn output_html(out: OutputStream) -> String {
    let buf = collect_or_panic(out).await;
    String::from_utf8(buf.body).expect("body was not valid UTF-8")
}

/// Read the body as raw bytes.
pub async fn output_body(out: OutputStream) -> Vec<u8> {
    collect_or_panic(out).await.body
}

/// Read the body as JSON. Returns `Value::Null` if the body fails to parse.
pub async fn output_json(out: OutputStream) -> serde_json::Value {
    let buf = collect_or_panic(out).await;
    serde_json::from_slice(&buf.body).unwrap_or(serde_json::Value::Null)
}

/// True if the OutputStream terminated with an error matching `code`.
/// The code string should match the ErrorCode debug format (e.g., "NotFound", "Internal").
pub async fn output_is_error(out: OutputStream, code: &str) -> bool {
    matches!(
        out.collect_buffered().await,
        Err(TerminalNotResponse::Error(e)) if format!("{:?}", e.code) == code
    )
}

#[cfg(test)]
mod tests {
    use wafer_core::clients::database as db;

    use super::*;

    #[tokio::test]
    async fn database_create_and_get_round_trip() {
        let ctx = TestContext::new().await;

        db::exec_raw(
            &ctx,
            "CREATE TABLE round_trip (id TEXT PRIMARY KEY, name TEXT)",
            &[],
        )
        .await
        .expect("create table");

        db::exec_raw(
            &ctx,
            "INSERT INTO round_trip (id, name) VALUES (?, ?)",
            &[serde_json::json!("r1"), serde_json::json!("alpha")],
        )
        .await
        .expect("insert row");

        let rows = db::query_raw(
            &ctx,
            "SELECT id, name FROM round_trip WHERE id = ?",
            &[serde_json::json!("r1")],
        )
        .await
        .expect("select");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "r1");
        assert_eq!(
            rows[0].data.get("name").and_then(|v| v.as_str()),
            Some("alpha")
        );
    }

    #[test]
    fn anon_msg_sets_action_and_path_with_no_user_id() {
        let m = anon_msg("retrieve", "/b/auth/login");
        assert_eq!(m.action(), "retrieve");
        assert_eq!(m.path(), "/b/auth/login");
        assert_eq!(m.user_id(), "");
    }

    #[test]
    fn auth_msg_sets_user_id() {
        let m = auth_msg("retrieve", "/b/auth/dashboard", "user-a");
        assert_eq!(m.action(), "retrieve");
        assert_eq!(m.path(), "/b/auth/dashboard");
        assert_eq!(m.user_id(), "user-a");
    }

    #[test]
    fn admin_msg_marks_admin_role() {
        use crate::blocks::helpers::is_admin;
        let m = admin_msg("retrieve", "/b/admin/users");
        assert_eq!(m.user_id(), "admin_1");
        assert!(is_admin(&m));
    }

    #[tokio::test]
    async fn output_status_reads_status_meta() {
        use crate::blocks::helpers::ResponseBuilder;
        let out = ResponseBuilder::new()
            .status(302)
            .body(Vec::new(), "text/plain");
        assert_eq!(output_status(out).await, 302);
    }

    #[tokio::test]
    async fn output_status_defaults_to_200_when_unset() {
        use crate::blocks::helpers::ResponseBuilder;
        let out = ResponseBuilder::new().body(Vec::new(), "text/plain");
        assert_eq!(output_status(out).await, 200);
    }

    #[tokio::test]
    async fn output_header_reads_named_header() {
        use crate::blocks::helpers::ResponseBuilder;
        let out = ResponseBuilder::new()
            .status(302)
            .set_header("Location", "/dashboard")
            .body(Vec::new(), "text/plain");
        assert_eq!(
            output_header(out, "Location").await.as_deref(),
            Some("/dashboard")
        );
    }

    #[tokio::test]
    async fn output_html_reads_body_as_utf8() {
        use crate::blocks::helpers::ResponseBuilder;
        let out = ResponseBuilder::new()
            .status(200)
            .body(b"<h1>hi</h1>".to_vec(), "text/html");
        assert_eq!(output_html(out).await, "<h1>hi</h1>");
    }

    #[tokio::test]
    async fn output_json_parses_body() {
        use crate::blocks::helpers::ResponseBuilder;
        let out = ResponseBuilder::new()
            .status(200)
            .body(br#"{"ok":true}"#.to_vec(), "application/json");
        assert_eq!(output_json(out).await, serde_json::json!({"ok": true}));
    }

    #[tokio::test]
    async fn with_auth_applies_orgs_and_users_tables() {
        let ctx = TestContext::with_auth().await;
        // Verify auth tables exist by inserting a user, then an org, then selecting.
        db::exec_raw(
            &ctx,
            "INSERT INTO suppers_ai__auth__users (id, email, display_name, role, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            &[
                serde_json::json!("user-a"),
                serde_json::json!("alice@example.com"),
                serde_json::json!("Alice"),
                serde_json::json!("user"),
                serde_json::json!("2026-01-01T00:00:00Z"),
                serde_json::json!("2026-01-01T00:00:00Z"),
            ],
        )
        .await
        .expect("insert user");

        db::exec_raw(
            &ctx,
            "INSERT INTO suppers_ai__auth__orgs (id, name, owner_user_id, is_reserved, created_at) \
             VALUES (?, ?, ?, 0, ?)",
            &[
                serde_json::json!("org-1"),
                serde_json::json!("acme"),
                serde_json::json!("user-a"),
                serde_json::json!("2026-01-01T00:00:00Z"),
            ],
        )
        .await
        .expect("insert org");

        let rows = db::query_raw(
            &ctx,
            "SELECT name FROM suppers_ai__auth__orgs WHERE id = ?",
            &[serde_json::json!("org-1")],
        )
        .await
        .expect("select org");
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].data.get("name").and_then(|v| v.as_str()),
            Some("acme")
        );
    }

    #[tokio::test]
    async fn registered_block_is_dispatched_through_call_block() {
        use async_trait::async_trait;
        use wafer_run::{block::Block as RunBlock, BlockCategory, BlockInfo, LifecycleEvent};

        struct EchoBlock;

        #[async_trait]
        impl RunBlock for EchoBlock {
            fn info(&self) -> BlockInfo {
                BlockInfo::new("test/echo", "0.0.1", "echo@v1", "echoes the request path")
                    .category(BlockCategory::Service)
            }

            async fn handle(
                &self,
                _ctx: &dyn Context,
                msg: Message,
                _input: InputStream,
            ) -> OutputStream {
                crate::blocks::helpers::ResponseBuilder::new()
                    .status(200)
                    .body(msg.path().as_bytes().to_vec(), "text/plain")
            }

            async fn lifecycle(
                &self,
                _ctx: &dyn Context,
                _e: LifecycleEvent,
            ) -> Result<(), WaferError> {
                Ok(())
            }
        }

        let mut ctx = TestContext::new().await;
        ctx.register_block("test/echo", Arc::new(EchoBlock));

        let msg = anon_msg("retrieve", "/echo-me");
        let resp = ctx.call_block("test/echo", msg, InputStream::empty()).await;
        let body = output_html(resp).await;
        assert_eq!(body, "/echo-me");
    }
}
