//! Shared test helpers for the `suppers-ai/admin` migration tests.
//!
//! Mirrors `tests/auth/common.rs`: `MigrationTestCtx` routes
//! `call_block("wafer-run/database", ...)` to a real `DatabaseBlock`
//! wrapping an in-memory `SQLiteDatabaseService`.

use std::sync::Arc;

use wafer_run::{Block, context::Context, Message, WaferError, InputStream, OutputStream};

#[derive(Clone)]
pub struct MigrationTestCtx {
    db_block: Arc<dyn Block>,
}

impl MigrationTestCtx {
    pub fn new() -> Self {
        let svc = Arc::new(
            wafer_block_sqlite::service::SQLiteDatabaseService::open_in_memory()
                .expect("open in-memory sqlite"),
        );
        let db_block: Arc<dyn Block> = Arc::new(
            wafer_core::service_blocks::database::DatabaseBlock::new(svc),
        );
        Self { db_block }
    }
}

#[async_trait::async_trait]
impl Context for MigrationTestCtx {
    async fn call_block(&self, block_name: &str, msg: Message, input: InputStream) -> OutputStream {
        match block_name {
            "wafer-run/database" => self.db_block.handle(self, msg, input).await,
            _ => OutputStream::error(WaferError::new(
                wafer_run::ErrorCode::NotFound,
                format!("block '{block_name}' not registered in test ctx"),
            )),
        }
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
