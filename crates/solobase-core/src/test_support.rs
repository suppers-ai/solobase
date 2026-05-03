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
        let database_block: Arc<dyn Block> =
            Arc::new(wafer_core::service_blocks::database::DatabaseBlock::new(svc));

        Self {
            database_block,
            config: Arc::new(Mutex::new(HashMap::new())),
            blocks: Arc::new(Mutex::new(HashMap::new())),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use wafer_core::clients::database as db;

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
}
