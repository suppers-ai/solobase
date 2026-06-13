//! Browser-side `DatabaseService` backed by sql.js via the JS bridge.
//!
//! The browser backend implements only the [`DbExec`] execution *primitives*
//! (synchronous `bridge::db_query_raw` / `bridge::db_exec_raw`, plus an async
//! `bridge::dbFlush()` after every write to persist sql.js to OPFS). All
//! `get/list/count/sum/create/update/delete` orchestration — filter/IN
//! expansion, sorted-key INSERT/UPDATE construction, lazy column-add,
//! table-exists guards — is inherited from the shared `wafer-core` [`DbExec`]
//! defaults, identical to `wafer-block-sqlite`, `wafer-block-postgres`, and the
//! Cloudflare D1 backend.
//!
//! Tables must already exist via the owning block's migration files (applied
//! at `lifecycle(Init)`); the shared `ensure_data_columns`/`ensure_query_columns`
//! add only missing *columns* (always `TEXT` on SQLite) on demand.

use std::collections::HashMap;

use wafer_block::db::{Filter, ListOptions};
use wafer_core::interfaces::database::{
    exec::DbExec,
    service::{Column, DatabaseError, DatabaseService, Record, RecordList, Table},
};
use wafer_sql_utils::{introspect, Backend};

use crate::bridge;

/// Browser-side DatabaseService backed by sql.js via the JS bridge.
pub struct BrowserDatabaseService;

// SAFETY: `BrowserDatabaseService` is a unit struct with no shared state.
// wasm32-unknown-unknown has no threads, so the `Send`/`Sync` bounds
// required by `Arc<dyn DatabaseService>` are satisfied trivially — no
// cross-thread aliasing or data races are possible.
unsafe impl Send for BrowserDatabaseService {}
unsafe impl Sync for BrowserDatabaseService {}

// ─── pure helpers ─────────────────────────────────────────────────────────────

/// Map a JSON value to a scalar suitable for embedding in a params array.
/// Arrays and objects are serialized as JSON strings — sql.js binds them as
/// TEXT, matching the D1 `json_value_to_js` policy.
fn coerce_param(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            serde_json::Value::String(v.to_string())
        }
        other => other.clone(),
    }
}

/// Serialize params into the JSON array string the bridge functions expect.
/// Each value is `coerce_param`'d first so arrays/objects bind as JSON text.
fn params_to_json(params: &[serde_json::Value]) -> Result<String, DatabaseError> {
    let coerced: Vec<serde_json::Value> = params.iter().map(coerce_param).collect();
    serde_json::to_string(&coerced)
        .map_err(|e| DatabaseError::Internal(format!("encode params: {e}")))
}

/// Parse the JSON array of row objects returned by `db_query_raw` into
/// `Vec<Record>`. JSON-looking TEXT columns (sql.js stores JSON as TEXT) are
/// re-parsed back into structured values.
fn parse_rows(json: &str) -> Result<Vec<Record>, DatabaseError> {
    let rows: Vec<serde_json::Value> = serde_json::from_str(json)
        .map_err(|e| DatabaseError::Internal(format!("parse rows: {e}")))?;

    let mut records = Vec::with_capacity(rows.len());
    for row in rows {
        let serde_json::Value::Object(obj) = row else {
            return Err(DatabaseError::Internal("expected row object".to_string()));
        };

        let mut data: HashMap<String, serde_json::Value> = HashMap::new();
        let mut id = String::new();

        for (k, v) in obj {
            let parsed = match &v {
                serde_json::Value::String(s)
                    if (s.starts_with('{') && s.ends_with('}'))
                        || (s.starts_with('[') && s.ends_with(']')) =>
                {
                    serde_json::from_str(s).unwrap_or(v.clone())
                }
                other => other.clone(),
            };

            if k == "id" {
                id = match &parsed {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => String::new(),
                };
            }
            data.insert(k, parsed);
        }

        records.push(Record { id, data });
    }

    Ok(records)
}

/// The first scalar value of a single-column aggregate row, regardless of its
/// alias (the shared builders alias `COUNT`/`SUM` columns).
fn first_scalar(records: Vec<Record>) -> Option<serde_json::Value> {
    records.into_iter().next().and_then(|r| {
        r.data.into_iter().next().map(|(_, v)| v)
        // `id` is stripped into `Record.id` by `parse_rows`; a pure scalar
        // query (`SELECT COUNT(*) AS cnt`) never names a column `id`, so
        // the remaining-data map carries the value.
    })
}

// ─── DbExec primitives — the only backend-specific execution code ─────────────

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl DbExec for BrowserDatabaseService {
    const BACKEND: Backend = Backend::Sqlite;

    async fn run_fetch(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> Result<Vec<Record>, DatabaseError> {
        let params_json = params_to_json(params)?;
        let json = bridge::db_query_raw(sql, &params_json)
            .map_err(|e| DatabaseError::Internal(format!("sql exec: {e:?}")))?;
        parse_rows(&json)
    }

    async fn run_fetch_one(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> Result<Record, DatabaseError> {
        let records = self.run_fetch(sql, params).await?;
        records.into_iter().next().ok_or(DatabaseError::NotFound)
    }

    async fn run_execute(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> Result<i64, DatabaseError> {
        let params_json = params_to_json(params)?;
        let result = bridge::db_exec_raw(sql, &params_json)
            .map_err(|e| DatabaseError::Internal(format!("sql exec: {e:?}")))?;
        // Persist sql.js to OPFS after every mutating statement (INSERT/UPDATE/
        // DELETE and the ALTER TABLE adds from the lazy column-add path).
        bridge::dbFlush().await;
        Ok(result.trim().parse::<i64>().unwrap_or(0))
    }

    async fn run_scalar_i64(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> Result<i64, DatabaseError> {
        let records = self.run_fetch(sql, params).await?;
        Ok(first_scalar(records)
            .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
            .unwrap_or(0))
    }

    async fn run_scalar_f64(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> Result<f64, DatabaseError> {
        let records = self.run_fetch(sql, params).await?;
        Ok(first_scalar(records)
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
            .unwrap_or(0.0))
    }

    async fn dbx_table_exists(&self, table: &str) -> Result<bool, DatabaseError> {
        let (sql, params) = introspect::build_table_exists(table, Backend::Sqlite);
        Ok(self.run_scalar_i64(&sql, &params).await? > 0)
    }
}

// ─── DatabaseService — forwards into the shared DbExec defaults ───────────────

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl DatabaseService for BrowserDatabaseService {
    async fn get(&self, collection: &str, id: &str) -> Result<Record, DatabaseError> {
        DbExec::get(self, collection, id).await
    }

    async fn list(
        &self,
        collection: &str,
        opts: &ListOptions,
    ) -> Result<RecordList, DatabaseError> {
        DbExec::list(self, collection, opts).await
    }

    async fn create(
        &self,
        collection: &str,
        data: HashMap<String, serde_json::Value>,
    ) -> Result<Record, DatabaseError> {
        DbExec::create(self, collection, data).await
    }

    async fn update(
        &self,
        collection: &str,
        id: &str,
        data: HashMap<String, serde_json::Value>,
    ) -> Result<Record, DatabaseError> {
        DbExec::update(self, collection, id, data).await
    }

    async fn delete(&self, collection: &str, id: &str) -> Result<(), DatabaseError> {
        DbExec::delete(self, collection, id).await
    }

    async fn count(&self, collection: &str, filters: &[Filter]) -> Result<i64, DatabaseError> {
        DbExec::count(self, collection, filters).await
    }

    async fn sum(
        &self,
        collection: &str,
        field: &str,
        filters: &[Filter],
    ) -> Result<f64, DatabaseError> {
        DbExec::sum(self, collection, field, filters).await
    }

    async fn query_raw(
        &self,
        query: &str,
        args: &[serde_json::Value],
    ) -> Result<Vec<Record>, DatabaseError> {
        DbExec::query_raw(self, query, args).await
    }

    async fn exec_raw(
        &self,
        query: &str,
        args: &[serde_json::Value],
    ) -> Result<i64, DatabaseError> {
        DbExec::exec_raw(self, query, args).await
    }

    async fn delete_where(
        &self,
        collection: &str,
        filters: &[Filter],
    ) -> Result<(), DatabaseError> {
        DbExec::delete_where(self, collection, filters).await
    }

    async fn delete_where_count(
        &self,
        collection: &str,
        filters: &[Filter],
    ) -> Result<i64, DatabaseError> {
        DbExec::delete_where_count(self, collection, filters).await
    }

    async fn take_where(
        &self,
        collection: &str,
        filters: &[Filter],
    ) -> Result<Vec<Record>, DatabaseError> {
        DbExec::take_where(self, collection, filters).await
    }

    async fn update_where(
        &self,
        collection: &str,
        filters: &[Filter],
        data: HashMap<String, serde_json::Value>,
    ) -> Result<(), DatabaseError> {
        DbExec::update_where(self, collection, filters, data).await
    }

    async fn increment_field_where(
        &self,
        collection: &str,
        col: &str,
        delta: i64,
        filters: &[Filter],
    ) -> Result<i64, DatabaseError> {
        DbExec::increment_field_where(self, collection, col, delta, filters).await
    }

    // --- Schema management ---

    async fn ensure_schema_table(&self, table: &Table) -> Result<(), DatabaseError> {
        // Blocks own their schema via migration files; runtime callers may still
        // ask for a one-off table. Build the DDL via the shared ddl builders and
        // run it through the execution primitive.
        let create = wafer_sql_utils::ddl::build_create_table(table, Backend::Sqlite)
            .map_err(|e| DatabaseError::Internal(format!("build create table: {e}")))?;
        self.run_execute(&create.sql, &[]).await?;

        let existing = DbExec::get_columns(self, &table.name).await?;
        for col in &table.columns {
            if !existing.contains(&col.name.to_lowercase()) {
                let alter =
                    wafer_sql_utils::ddl::build_add_column(&table.name, col, Backend::Sqlite);
                // Best-effort: a duplicate column on re-run is benign.
                let _ = self.run_execute(&alter.sql, &[]).await;
            }
        }

        for idx in &table.indexes {
            let stmt = wafer_sql_utils::ddl::build_create_index(&table.name, idx, Backend::Sqlite)
                .map_err(|e| DatabaseError::Internal(format!("build create index: {e}")))?;
            self.run_execute(&stmt.sql, &[]).await?;
        }
        for stmt in wafer_sql_utils::ddl::build_fk_indexes(table, Backend::Sqlite)
            .map_err(|e| DatabaseError::Internal(format!("build FK indexes: {e}")))?
        {
            self.run_execute(&stmt.sql, &[]).await?;
        }
        Ok(())
    }

    async fn schema_table_exists(&self, name: &str) -> Result<bool, DatabaseError> {
        DbExec::schema_table_exists(self, name).await
    }

    async fn schema_drop_table(&self, name: &str) -> Result<(), DatabaseError> {
        let stmt = wafer_sql_utils::ddl::build_drop_table(name, Backend::Sqlite);
        self.run_execute(&stmt.sql, &[]).await?;
        Ok(())
    }

    async fn schema_add_column(&self, table: &str, column: &Column) -> Result<(), DatabaseError> {
        let stmt = wafer_sql_utils::ddl::build_add_column(table, column, Backend::Sqlite);
        self.run_execute(&stmt.sql, &[]).await?;
        Ok(())
    }
}

/// Factory: returns an `Arc<dyn DatabaseService>` backed by the
/// browser's sql.js + OPFS integration. Call after `crate::db_init()`
/// has completed.
pub fn make_database_service() -> std::sync::Arc<dyn DatabaseService> {
    std::sync::Arc::new(BrowserDatabaseService)
}
