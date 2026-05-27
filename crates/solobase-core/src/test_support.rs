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
    types::{ErrorCode, Message, ResourceGrant, WaferError},
    InputStream, OutputStream,
};

/// Minimal test context backed by a real in-memory SQLite database.
///
/// Routes `"wafer-run/database"` calls to the production `DatabaseBlock`.
/// Other named blocks can be registered via `blocks` (unused until Task 6).
///
/// `Clone` is shallow — every interior field is already `Arc`/`Mutex`-shared
/// (or trivially copyable), so a clone produces another handle pointing at
/// the same database, blocks map, and config. Used by [`Context::clone_arc`]
/// so service objects (e.g. `AuthServiceImpl`) can stash an owning context
/// handle in a `OnceLock` past the lifetime of a `&TestContext` borrow.
#[derive(Clone)]
pub struct TestContext {
    database_block: Arc<dyn Block>,
    /// Config snapshot used by `config_get`. Immutable after construction so
    /// `config_get` can return `Option<&str>` without holding a lock.
    /// Populated via [`set_config`].
    config: Arc<HashMap<String, String>>,
    /// Placeholder for dynamically registered blocks — populated by Task 6.
    pub blocks: Arc<Mutex<HashMap<String, Arc<dyn Block>>>>,
    /// WRAP-enforcement caller identity. `None` = WRAP checks skipped (the
    /// default — keeps existing tests untouched). Set via [`with_wrap`].
    caller_id: Option<String>,
    /// Grants visible to the WRAP check. Empty unless [`with_wrap`] populates.
    wrap_grants: Vec<ResourceGrant>,
    /// Admin block id for the WRAP check (`""` = no admin override).
    wrap_admin_block: String,
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
            config: Arc::new(HashMap::new()),
            blocks: Arc::new(Mutex::new(HashMap::new())),
            caller_id: None,
            wrap_grants: Vec::new(),
            wrap_admin_block: String::new(),
        }
    }

    /// Insert a single config entry into the snapshot.
    ///
    /// Makes a fresh `Arc<HashMap>` clone on each call — fine for tests,
    /// where the number of entries is small and mutations happen at setup time.
    pub fn set_config(&mut self, key: &str, value: &str) {
        let mut map = (*self.config).clone();
        map.insert(key.to_string(), value.to_string());
        self.config = Arc::new(map);
    }

    /// Opt the test into WRAP enforcement on `call_block`.
    ///
    /// Until called, `call_block` ignores `wrap.resource` meta — this matches
    /// pre-existing test behaviour. After calling, the same WRAP rules the
    /// production runtime applies (own-resource, admin override, grant match)
    /// gate every `call_block` invocation that carries `wrap.resource` meta.
    /// Typed clients (`wafer_core::clients::database::*`, etc.) set this
    /// meta automatically, so this is what makes a test exercise grants.
    ///
    /// `caller_id` is the block id the test is acting as — typically the
    /// block whose handler is under test. `grants` is the list visible to
    /// the WRAP check; tests that want to exercise a real block's grants
    /// should source them from `<Block>::default().info().grants` rather
    /// than re-listing grant literals.
    pub fn with_wrap(
        mut self,
        caller_id: &str,
        grants: Vec<ResourceGrant>,
        admin_block: &str,
    ) -> Self {
        self.caller_id = Some(caller_id.to_string());
        self.wrap_grants = grants;
        self.wrap_admin_block = admin_block.to_string();
        self
    }

    /// Build a `TestContext` with admin + auth block migrations applied.
    ///
    /// Convenience constructor for tests that need the
    /// `suppers_ai__auth__{users,orgs,sessions,provider_links,...}` schema
    /// in place — most repo and handler tests do.
    ///
    /// Admin migrations run first so that the
    /// `suppers_ai__admin__block_settings` tracking table exists before
    /// auth's `apply_if_blessed` upserts its `current_hash` row. In
    /// production this ordering is guaranteed by `register_all_static_blocks`
    /// (admin is registered first); here we enforce it explicitly.
    pub async fn with_auth() -> Self {
        let ctx = Self::new().await;
        crate::blocks::admin::migrations::apply(&ctx)
            .await
            .expect("apply admin migrations in test fixture");
        crate::blocks::auth::migrations::apply(&ctx)
            .await
            .expect("apply auth migrations in test fixture");
        ctx
    }

    /// Build a `TestContext` with admin migrations applied (only).
    ///
    /// Use this for tests that exercise a block's own `init()` / migration
    /// application directly — the prerequisite is that
    /// `suppers_ai__admin__block_settings` exists so `apply_if_blessed` can
    /// upsert its tracking row.
    pub async fn with_admin() -> Self {
        let ctx = Self::new().await;
        crate::blocks::admin::migrations::apply(&ctx)
            .await
            .expect("apply admin migrations in test fixture");
        ctx
    }

    /// Build a `TestContext` with admin + auth + files migrations applied.
    pub async fn with_files() -> Self {
        let ctx = Self::with_auth().await;
        crate::blocks::files::migrations::apply(&ctx)
            .await
            .expect("apply files migrations in test fixture");
        ctx
    }

    /// Build a `TestContext` with admin + auth + userportal migrations applied.
    pub async fn with_userportal() -> Self {
        let ctx = Self::with_auth().await;
        crate::blocks::userportal::migrations::apply(&ctx)
            .await
            .expect("apply userportal migrations in test fixture");
        ctx
    }

    /// Build a `TestContext` with admin + auth + vector migrations applied.
    pub async fn with_vector() -> Self {
        let ctx = Self::with_auth().await;
        crate::blocks::vector::migrations::apply(&ctx)
            .await
            .expect("apply vector migrations in test fixture");
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
        // WRAP enforcement (only when the test opted in via `with_wrap`).
        // Mirrors `RuntimeContext::call_block` in wafer-run/crates/wafer-run/
        // src/context.rs:138-163 — same `check_access` callsite shape so
        // tests see identical permission behaviour to production.
        if let Some(ref caller) = self.caller_id {
            let resource = msg.get_meta(wafer_block::meta::META_WRAP_RESOURCE);
            if !resource.is_empty() {
                let is_write = msg.get_meta(wafer_block::meta::META_WRAP_ACCESS) == "write";
                let rt_str = msg.get_meta(wafer_block::meta::META_WRAP_RESOURCE_TYPE);
                let rt = if rt_str.is_empty() {
                    None
                } else {
                    wafer_run::types::ResourceType::parse(rt_str)
                };
                if let Err(e) = wafer_block::wrap::check_access(
                    Some(caller.as_str()),
                    resource,
                    is_write,
                    rt.as_ref(),
                    &self.wrap_grants,
                    &self.wrap_admin_block,
                ) {
                    return OutputStream::error(e);
                }
            }
        }

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

    fn config_get(&self, key: &str) -> Option<&str> {
        self.config.get(key).map(String::as_str)
    }

    fn clone_arc(&self) -> Arc<dyn Context> {
        // Cheap — all interior state is `Arc`/`Mutex`-shared.
        Arc::new(self.clone())
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
/// terminates with anything other than `Complete` or `Halt`.
///
/// `Halt` is a legitimate success-shaped terminal (used e.g. by CORS
/// preflight to short-circuit with a 204 + headers), so tests treat it
/// the same as `Complete` — the body+meta are returned for assertion.
///
/// Tests should not see errors from handlers under test unless they're
/// explicitly asserting on error paths — use `output_is_error` for that.
pub async fn collect_or_panic(out: OutputStream) -> BufferedResponse {
    match out.collect_buffered().await {
        Ok(buf) => buf,
        Err(TerminalNotResponse::Halt(buf)) => buf,
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
    use wafer_block::db::ListOptions;
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
        let m = auth_msg("retrieve", "/b/userportal/", "user-a");
        assert_eq!(m.action(), "retrieve");
        assert_eq!(m.path(), "/b/userportal/");
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

    #[tokio::test]
    async fn with_wrap_denies_unowned_resource_without_grant() {
        // Caller "block-x" tries to read auth-owned table; no grants → denied.
        let ctx = TestContext::with_auth().await.with_wrap(
            "test/block-x",
            Vec::new(),
            "suppers-ai/admin",
        );

        let result = db::list(&ctx, "suppers_ai__auth__users", &ListOptions::default()).await;

        let err = result.expect_err("WRAP must deny call without grant");
        assert!(
            err.to_string().contains("WRAP"),
            "error must mention WRAP, got: {err}"
        );
    }

    #[tokio::test]
    async fn with_wrap_allows_call_when_grant_matches() {
        let grants = vec![ResourceGrant::read(
            "test/block-x",
            "suppers_ai__auth__users",
        )];
        let ctx =
            TestContext::with_auth()
                .await
                .with_wrap("test/block-x", grants, "suppers-ai/admin");

        // Empty users table — listing must succeed (zero rows is success).
        let res = db::list(&ctx, "suppers_ai__auth__users", &ListOptions::default())
            .await
            .expect("WRAP must allow listing with matching grant");
        assert_eq!(res.records.len(), 0);
    }

    #[tokio::test]
    async fn without_with_wrap_grants_are_unchecked() {
        // Default TestContext (no `with_wrap`) keeps WRAP-bypassing legacy
        // behaviour so existing tests aren't disturbed.
        let ctx = TestContext::with_auth().await;
        let res = db::list(&ctx, "suppers_ai__auth__users", &ListOptions::default())
            .await
            .expect("call must succeed without with_wrap");
        assert_eq!(res.records.len(), 0);
    }
}
