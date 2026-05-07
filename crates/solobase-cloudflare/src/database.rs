//! Async database service backed by Cloudflare D1 (SQLite at the edge).
//!
//! Implements the shared `DatabaseService` trait from wafer-core so D1Block
//! can reuse the shared message handler.
//!
//! ## Lazy schema
//!
//! Mirrors `wafer-block-sqlite`'s lazy-schema semantics: reads against a
//! missing table return empty/NotFound (instead of erroring), and the
//! first `create()` against a collection runs `CREATE TABLE IF NOT EXISTS`
//! to materialize the table from the data keys (TEXT columns, `id` as
//! PRIMARY KEY). This keeps init code paths like `seed_reserved_orgs`
//! working on a fresh D1 without external migrations — same behavior as
//! a fresh native sqlite file.

use std::collections::{HashMap, HashSet};

use wafer_core::interfaces::database::service::{
    Column, DatabaseError, DatabaseService, Filter, FilterOp, ListOptions, Record, RecordList,
    Table,
};
use wasm_bindgen::JsValue;
use worker::*;

/// Async database service wrapping Cloudflare D1.
pub struct D1DatabaseService {
    db: D1Database,
}

impl D1DatabaseService {
    pub fn new(db: D1Database) -> Self {
        Self { db }
    }

    /// For each key in `data` that isn't already a column on `table`,
    /// run `ALTER TABLE ... ADD COLUMN ... TEXT`. Best-effort: errors
    /// (e.g. "duplicate column name" from a concurrent insert that
    /// already added the column) are silently dropped — the caller's
    /// subsequent INSERT will surface any real schema problem.
    ///
    /// `table` and the data keys are assumed to have already been
    /// run through `sanitize_ident`.
    async fn add_missing_columns(&self, table: &str, data: &HashMap<String, serde_json::Value>) {
        let existing = match self.list_table_columns(table).await {
            Ok(cols) => cols,
            // If we can't even read the schema, skip and let INSERT fail
            // with a real message rather than masking a deeper problem.
            Err(_) => return,
        };
        for key in data.keys() {
            let safe_key = sanitize_ident(key);
            if existing.contains(&safe_key.to_lowercase()) {
                continue;
            }
            let _ = self
                .db
                .prepare(&format!(
                    "ALTER TABLE {} ADD COLUMN {} TEXT",
                    table, safe_key
                ))
                .run()
                .await;
        }
    }

    /// Names of columns currently on `table`, lowercased for
    /// case-insensitive comparison. Returns an empty set on missing
    /// table — callers fall back to the same behaviour as if the table
    /// had no extra columns to skip.
    async fn list_table_columns(&self, table: &str) -> Result<HashSet<String>, DatabaseError> {
        let stmt = self.db.prepare(&format!("PRAGMA table_info({})", table));
        let result = match stmt.all().await {
            Ok(r) => r,
            Err(e) if is_no_such_table(&e.to_string()) => return Ok(HashSet::new()),
            Err(e) => return Err(db_err(e)),
        };
        let rows: Vec<serde_json::Value> = result.results().map_err(db_err)?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                r.get("name")
                    .and_then(|v| v.as_str())
                    .map(str::to_lowercase)
            })
            .collect())
    }
}

// Safety: wasm32-unknown-unknown is single-threaded.
unsafe impl Send for D1DatabaseService {}
unsafe impl Sync for D1DatabaseService {}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl DatabaseService for D1DatabaseService {
    async fn get(&self, collection: &str, id: &str) -> Result<Record, DatabaseError> {
        let table = sanitize_ident(collection);
        let stmt = self
            .db
            .prepare(&format!("SELECT * FROM {} WHERE id = ?", table))
            .bind(&[id.into()])
            .map_err(db_err)?;

        let row = match stmt.first::<serde_json::Value>(None).await {
            Ok(row) => row,
            Err(e) if is_no_such_table(&e.to_string()) => return Err(DatabaseError::NotFound),
            Err(e) => return Err(db_err(e)),
        };
        match row {
            Some(val) => Ok(json_to_record(val)),
            None => Err(DatabaseError::NotFound),
        }
    }

    async fn list(
        &self,
        collection: &str,
        opts: &ListOptions,
    ) -> Result<RecordList, DatabaseError> {
        let table = sanitize_ident(collection);
        let (where_sql, params) = build_where_clause(&opts.filters);
        let limit_for_empty = if opts.limit > 0 { opts.limit } else { 100 };

        // Count query
        let count_sql = format!("SELECT COUNT(*) as cnt FROM {} WHERE {}", table, where_sql);
        let count_stmt = self.db.prepare(&count_sql).bind(&params).map_err(db_err)?;
        let count_row = match count_stmt.first::<serde_json::Value>(None).await {
            Ok(row) => row,
            Err(e) if is_no_such_table(&e.to_string()) => {
                return Ok(RecordList {
                    records: Vec::new(),
                    total_count: 0,
                    page: 1,
                    page_size: limit_for_empty,
                });
            }
            Err(e) => return Err(db_err(e)),
        };
        let total_count = count_row
            .and_then(|v| v.get("cnt").and_then(|c| c.as_i64()))
            .unwrap_or(0);

        // Data query
        let mut sql = format!("SELECT * FROM {} WHERE {}", table, where_sql);

        if !opts.sort.is_empty() {
            let order: Vec<String> = opts
                .sort
                .iter()
                .map(|s| {
                    let col = sanitize_ident(&s.field);
                    if s.desc {
                        format!("{} DESC", col)
                    } else {
                        format!("{} ASC", col)
                    }
                })
                .collect();
            sql.push_str(&format!(" ORDER BY {}", order.join(", ")));
        }

        let limit = if opts.limit > 0 { opts.limit } else { 100 };
        sql.push_str(&format!(" LIMIT {} OFFSET {}", limit, opts.offset));

        let stmt = self.db.prepare(&sql).bind(&params).map_err(db_err)?;
        let results = stmt.all().await.map_err(db_err)?;
        let rows: Vec<serde_json::Value> = results.results().map_err(db_err)?;

        let page = if limit > 0 {
            (opts.offset / limit) + 1
        } else {
            1
        };

        Ok(RecordList {
            records: rows.into_iter().map(json_to_record).collect(),
            total_count,
            page,
            page_size: limit,
        })
    }

    async fn create(
        &self,
        collection: &str,
        data: HashMap<String, serde_json::Value>,
    ) -> Result<Record, DatabaseError> {
        let table = sanitize_ident(collection);
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let mut data = data;
        data.entry("created_at".to_string())
            .or_insert_with(|| serde_json::Value::String(now.clone()));
        data.entry("updated_at".to_string())
            .or_insert_with(|| serde_json::Value::String(now));

        // Lazy schema: materialize the table on first insert. Mirrors the
        // native sqlite service. The `id` column gets PRIMARY KEY; every
        // other key from `data` becomes a TEXT column.
        let ddl = ensure_table_sql(&table, data.keys().map(|k| k.as_str()));
        self.db
            .prepare(&ddl)
            .run()
            .await
            .map_err(|e| db_err(format!("ensure_table {}: {e}", table)))?;

        // Schema evolution: when the table already existed (e.g. seeded
        // with a smaller column set by an earlier insert from a different
        // code path), `IF NOT EXISTS` is a no-op — the new keys we're
        // about to insert wouldn't have columns. Mirror native sqlite's
        // `ensure_table` and ALTER TABLE ADD COLUMN for every data key
        // not already on the table. Errors are intentionally swallowed:
        // a concurrent insert may have added the same column already
        // (D1 surfaces "duplicate column name" in that case), and we'd
        // rather let the subsequent INSERT either succeed or fail with
        // a clearer message than abort here on a benign race.
        self.add_missing_columns(&table, &data).await;

        let mut columns = vec!["id".to_string()];
        let mut placeholders = vec!["?".to_string()];
        let mut params: Vec<JsValue> = vec![id.clone().into()];

        for (key, val) in &data {
            columns.push(sanitize_ident(key));
            placeholders.push("?".to_string());
            params.push(json_value_to_js(val));
        }

        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table,
            columns.join(", "),
            placeholders.join(", ")
        );

        self.db
            .prepare(&sql)
            .bind(&params)
            .map_err(db_err)?
            .run()
            .await
            .map_err(db_err)?;

        Ok(Record { id, data })
    }

    async fn update(
        &self,
        collection: &str,
        id: &str,
        data: HashMap<String, serde_json::Value>,
    ) -> Result<Record, DatabaseError> {
        let table = sanitize_ident(collection);
        let now = chrono::Utc::now().to_rfc3339();

        let mut sets = Vec::new();
        let mut params: Vec<JsValue> = Vec::new();

        let mut data = data;
        data.insert("updated_at".to_string(), serde_json::Value::String(now));

        for (key, val) in &data {
            sets.push(format!("{} = ?", sanitize_ident(key)));
            params.push(json_value_to_js(val));
        }

        params.push(id.into());

        let sql = format!("UPDATE {} SET {} WHERE id = ?", table, sets.join(", "));

        self.db
            .prepare(&sql)
            .bind(&params)
            .map_err(db_err)?
            .run()
            .await
            .map_err(db_err)?;

        self.get(collection, id).await
    }

    async fn delete(&self, collection: &str, id: &str) -> Result<(), DatabaseError> {
        let table = sanitize_ident(collection);
        let sql = format!("DELETE FROM {} WHERE id = ?", table);

        self.db
            .prepare(&sql)
            .bind(&[id.into()])
            .map_err(db_err)?
            .run()
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn count(&self, collection: &str, filters: &[Filter]) -> Result<i64, DatabaseError> {
        let table = sanitize_ident(collection);
        let (where_sql, params) = build_where_clause(filters);

        let sql = format!("SELECT COUNT(*) as cnt FROM {} WHERE {}", table, where_sql);

        let row = match self
            .db
            .prepare(&sql)
            .bind(&params)
            .map_err(db_err)?
            .first::<serde_json::Value>(None)
            .await
        {
            Ok(row) => row,
            Err(e) if is_no_such_table(&e.to_string()) => return Ok(0),
            Err(e) => return Err(db_err(e)),
        };

        Ok(row
            .and_then(|v| v.get("cnt").and_then(|c| c.as_i64()))
            .unwrap_or(0))
    }

    async fn sum(
        &self,
        collection: &str,
        field: &str,
        filters: &[Filter],
    ) -> Result<f64, DatabaseError> {
        let col = sanitize_ident(field);
        let table = sanitize_ident(collection);
        let (where_sql, params) = build_where_clause(filters);

        let sql = format!(
            "SELECT COALESCE(SUM({}), 0) as s FROM {} WHERE {}",
            col, table, where_sql
        );

        let row = match self
            .db
            .prepare(&sql)
            .bind(&params)
            .map_err(db_err)?
            .first::<serde_json::Value>(None)
            .await
        {
            Ok(row) => row,
            Err(e) if is_no_such_table(&e.to_string()) => return Ok(0.0),
            Err(e) => return Err(db_err(e)),
        };

        Ok(row
            .and_then(|v| v.get("s").and_then(|s| s.as_f64()))
            .unwrap_or(0.0))
    }

    async fn query_raw(
        &self,
        query: &str,
        args: &[serde_json::Value],
    ) -> Result<Vec<Record>, DatabaseError> {
        let params: Vec<JsValue> = args.iter().map(json_value_to_js).collect();
        let stmt = self.db.prepare(query).bind(&params).map_err(db_err)?;
        let results = stmt.all().await.map_err(db_err)?;
        let rows: Vec<serde_json::Value> = results.results().map_err(db_err)?;
        Ok(rows.into_iter().map(json_to_record).collect())
    }

    async fn exec_raw(
        &self,
        query: &str,
        args: &[serde_json::Value],
    ) -> Result<i64, DatabaseError> {
        let params: Vec<JsValue> = args.iter().map(json_value_to_js).collect();
        self.db
            .prepare(query)
            .bind(&params)
            .map_err(db_err)?
            .run()
            .await
            .map_err(db_err)?;
        Ok(0)
    }

    async fn delete_where(
        &self,
        collection: &str,
        filters: &[Filter],
    ) -> Result<(), DatabaseError> {
        let table = sanitize_ident(collection);
        let (where_sql, params) = build_where_clause(filters);

        let sql = format!("DELETE FROM {} WHERE {}", table, where_sql);
        self.db
            .prepare(&sql)
            .bind(&params)
            .map_err(db_err)?
            .run()
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn update_where(
        &self,
        collection: &str,
        filters: &[Filter],
        data: HashMap<String, serde_json::Value>,
    ) -> Result<(), DatabaseError> {
        let table = sanitize_ident(collection);

        let mut data = data;
        let now = chrono::Utc::now().to_rfc3339();
        if !data.contains_key("updated_at") {
            data.insert("updated_at".to_string(), serde_json::Value::String(now));
        }

        let mut sets = Vec::new();
        let mut params: Vec<JsValue> = Vec::new();

        for (key, val) in &data {
            sets.push(format!("{} = ?", sanitize_ident(key)));
            params.push(json_value_to_js(val));
        }

        let (where_sql, mut where_params) = build_where_clause(filters);
        params.append(&mut where_params);

        let sql = format!(
            "UPDATE {} SET {} WHERE {}",
            table,
            sets.join(", "),
            where_sql
        );

        self.db
            .prepare(&sql)
            .bind(&params)
            .map_err(db_err)?
            .run()
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn ensure_schema_table(&self, table: &Table) -> Result<(), DatabaseError> {
        // D1 schema is managed externally via Wrangler migrations
        let _ = table;
        Ok(())
    }

    async fn schema_table_exists(&self, _name: &str) -> Result<bool, DatabaseError> {
        Ok(true) // Assume tables exist (managed by Wrangler)
    }

    async fn schema_drop_table(&self, _name: &str) -> Result<(), DatabaseError> {
        Ok(()) // No-op on D1
    }

    async fn schema_add_column(&self, _table: &str, _column: &Column) -> Result<(), DatabaseError> {
        Ok(()) // No-op on D1
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Sanitize an identifier for safe SQL interpolation (table/column names).
pub(crate) fn sanitize_ident(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
}

/// Convert a serde_json::Value to a JsValue for D1 binding.
fn json_value_to_js(val: &serde_json::Value) -> JsValue {
    match val {
        serde_json::Value::Null => JsValue::NULL,
        serde_json::Value::Bool(b) => JsValue::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                JsValue::from(i as f64)
            } else if let Some(f) = n.as_f64() {
                JsValue::from(f)
            } else {
                JsValue::from(n.to_string())
            }
        }
        serde_json::Value::String(s) => JsValue::from(s.as_str()),
        _ => JsValue::from(val.to_string()),
    }
}

/// Build a WHERE clause from filters. Returns the SQL string and bound params.
fn build_where_clause(filters: &[Filter]) -> (String, Vec<JsValue>) {
    let mut where_clauses: Vec<String> = Vec::new();
    let mut params: Vec<JsValue> = Vec::new();

    for f in filters {
        let col = sanitize_ident(&f.field);
        match f.operator {
            FilterOp::IsNull => where_clauses.push(format!("{} IS NULL", col)),
            FilterOp::IsNotNull => where_clauses.push(format!("{} IS NOT NULL", col)),
            FilterOp::In => {
                if let Some(arr) = f.value.as_array() {
                    let placeholders: Vec<&str> = arr.iter().map(|_| "?").collect();
                    where_clauses.push(format!("{} IN ({})", col, placeholders.join(", ")));
                    for val in arr {
                        params.push(json_value_to_js(val));
                    }
                } else {
                    where_clauses.push(format!("{} IN (?)", col));
                    params.push(json_value_to_js(&f.value));
                }
            }
            _ => {
                where_clauses.push(format!("{} {} ?", col, f.operator.as_sql()));
                params.push(json_value_to_js(&f.value));
            }
        }
    }

    let where_sql = if where_clauses.is_empty() {
        "1=1".to_string()
    } else {
        where_clauses.join(" AND ")
    };

    (where_sql, params)
}

/// Convert any Display error into a DatabaseError::Internal.
fn db_err(e: impl std::fmt::Display) -> DatabaseError {
    DatabaseError::Internal(e.to_string())
}

/// Build the `CREATE TABLE IF NOT EXISTS` DDL for lazy schema
/// materialization. Mirrors `wafer-block-sqlite::ensure_table`: TEXT for
/// every column; `id` is PRIMARY KEY (added if not in the data keys).
///
/// Pure function so it can be unit-tested without a D1 instance.
pub(crate) fn ensure_table_sql<'a>(
    table: &str,
    data_keys: impl IntoIterator<Item = &'a str>,
) -> String {
    let safe_table = sanitize_ident(table);
    let mut col_defs: Vec<String> = Vec::new();
    let mut has_id = false;
    for key in data_keys {
        let safe_key = sanitize_ident(key);
        if key == "id" {
            col_defs.insert(0, "id TEXT PRIMARY KEY".to_string());
            has_id = true;
        } else {
            col_defs.push(format!("{safe_key} TEXT"));
        }
    }
    if !has_id {
        col_defs.insert(0, "id TEXT PRIMARY KEY".to_string());
    }
    format!(
        "CREATE TABLE IF NOT EXISTS {} ({})",
        safe_table,
        col_defs.join(", ")
    )
}

/// Whether a D1 error message indicates the target table doesn't exist.
/// D1 surfaces SQLite's `no such table: X` verbatim through the JsValue
/// error; we string-match because the `worker::Error` type doesn't expose
/// SQLite's structured error code.
///
/// Pure function so it can be unit-tested.
pub(crate) fn is_no_such_table(msg: &str) -> bool {
    msg.contains("no such table")
}

/// Convert a D1 result row (as JSON) into a Record.
fn json_to_record(val: serde_json::Value) -> Record {
    if let serde_json::Value::Object(mut map) = val {
        let id = map
            .remove("id")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();

        Record {
            id,
            data: map.into_iter().collect(),
        }
    } else {
        Record {
            id: String::new(),
            data: HashMap::new(),
        }
    }
}

// Note: unit tests for `ensure_table_sql` and `is_no_such_table` are
// omitted because `solobase-cloudflare` only compiles on
// `wasm32-unknown-unknown` (the R2/D1 services hold `!Send` JsFutures).
// `cargo test -p solobase-cloudflare` errors before reaching the test
// module. End-to-end validation comes from a real CF deploy against an
// empty D1: registry-block init must seed the reserved orgs without
// erroring on "no such table".
