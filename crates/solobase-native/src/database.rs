//! Database platform-service factories for native targets.
//!
//! - `make_sqlite_database_service(path)` — wraps `wafer-block-sqlite`
//!   `SQLiteDatabaseService`.
//! - `make_postgres_database_service(url)` — feature-gated on `postgres`.

use std::sync::Arc;

use wafer_core::interfaces::database::service::DatabaseService;

/// Open a SQLite database at `path` and wrap it in `Arc<dyn DatabaseService>`.
///
/// Panics if the file cannot be opened or created — same failure mode as
/// today's inline call. Consumers who want fallible construction can call
/// `wafer_block_sqlite::service::SQLiteDatabaseService::open(path)` directly.
pub fn make_sqlite_database_service(path: &str) -> Arc<dyn DatabaseService> {
    let svc = wafer_block_sqlite::service::SQLiteDatabaseService::open(path)
        .unwrap_or_else(|e| panic!("failed to open SQLite database at {path}: {e}"));
    Arc::new(svc)
}

/// Open a PostgreSQL connection via `url` and wrap it in
/// `Arc<dyn DatabaseService>`. Feature-gated.
///
/// Async because `PostgresDatabaseService::connect` is async.
#[cfg(feature = "postgres")]
pub async fn make_postgres_database_service(url: &str) -> Arc<dyn DatabaseService> {
    let svc = wafer_block_postgres::service::PostgresDatabaseService::connect(url)
        .await
        .unwrap_or_else(|e| panic!("failed to connect to Postgres at {url}: {e}"));
    Arc::new(svc)
}
