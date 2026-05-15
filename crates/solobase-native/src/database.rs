//! Database platform-service factories for native targets.
//!
//! - `make_sqlite_database_service(path)` — wraps `wafer-block-sqlite`
//!   `SQLiteDatabaseService`.
//! - `make_postgres_database_service(url)` — feature-gated on `postgres`.

use std::sync::Arc;

use anyhow::{Context, Result};
use wafer_core::interfaces::database::service::DatabaseService;

/// Open a SQLite database at `path` and wrap it in `Arc<dyn DatabaseService>`.
///
/// # Errors
///
/// Returns an error if the underlying file cannot be opened/created or if
/// the SQLite handle fails to initialise.
pub fn make_sqlite_database_service(path: &str) -> Result<Arc<dyn DatabaseService>> {
    let svc = wafer_block_sqlite::service::SQLiteDatabaseService::open(path)
        .with_context(|| format!("open SQLite database at {path}"))?;
    Ok(Arc::new(svc))
}

/// Open a PostgreSQL connection via `url` and wrap it in
/// `Arc<dyn DatabaseService>`. Feature-gated.
///
/// Async because `PostgresDatabaseService::connect` is async.
///
/// # Errors
///
/// Returns an error if the connection cannot be established.
#[cfg(feature = "postgres")]
pub async fn make_postgres_database_service(url: &str) -> Result<Arc<dyn DatabaseService>> {
    let svc = wafer_block_postgres::service::PostgresDatabaseService::connect(url)
        .await
        .with_context(|| format!("connect to Postgres at {url}"))?;
    Ok(Arc::new(svc))
}
