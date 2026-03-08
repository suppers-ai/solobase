//! Async database service backed by Cloudflare D1 (SQLite at the edge).
//!
//! This provides the same CRUD semantics as the wafer-core `DatabaseService`
//! trait but as async methods, since Cloudflare Workers don't support blocking I/O.
//!
//! Each tenant gets schema isolation via a `tenant_id` column on every table,
//! or separate D1 databases per tenant (configurable).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use worker::*;

/// Record returned from D1 queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub data: HashMap<String, serde_json::Value>,
}

/// Paginated list of records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordList {
    pub records: Vec<Record>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}

/// Filter for list queries.
#[derive(Debug, Clone)]
pub struct Filter {
    pub field: String,
    pub operator: FilterOp,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum FilterOp {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterEqual,
    LessThan,
    LessEqual,
    Like,
    In,
    IsNull,
    IsNotNull,
}

impl FilterOp {
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::NotEqual => "!=",
            Self::GreaterThan => ">",
            Self::GreaterEqual => ">=",
            Self::LessThan => "<",
            Self::LessEqual => "<=",
            Self::Like => "LIKE",
            Self::In => "IN",
            Self::IsNull => "IS NULL",
            Self::IsNotNull => "IS NOT NULL",
        }
    }
}

/// Sort directive.
#[derive(Debug, Clone)]
pub struct SortField {
    pub field: String,
    pub desc: bool,
}

/// List query options.
#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    pub filters: Vec<Filter>,
    pub sort: Vec<SortField>,
    pub limit: i64,
    pub offset: i64,
}

/// Database error.
#[derive(Debug)]
pub enum DatabaseError {
    NotFound,
    Internal(String),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "record not found"),
            Self::Internal(msg) => write!(f, "database error: {msg}"),
        }
    }
}

/// Async database service wrapping Cloudflare D1.
///
/// Provides tenant-scoped data access. All queries are automatically filtered
/// by the tenant_id to ensure data isolation.
pub struct D1DatabaseService {
    db: D1Database,
    tenant_id: String,
}

impl D1DatabaseService {
    pub fn new(db: D1Database, tenant_id: String) -> Self {
        Self { db, tenant_id }
    }

    /// Get a single record by ID.
    pub async fn get(&self, collection: &str, id: &str) -> Result<Record> {
        let table = sanitize_ident(collection);
        let stmt = self
            .db
            .prepare(&format!(
                "SELECT * FROM {} WHERE id = ? AND tenant_id = ?",
                table
            ))
            .bind(&[id.into(), self.tenant_id.clone().into()])?;

        let row = stmt.first::<serde_json::Value>(None).await?;
        match row {
            Some(val) => Ok(json_to_record(val)),
            None => Err(Error::RustError("record not found".into())),
        }
    }

    /// List records with filtering, sorting, and pagination.
    pub async fn list(
        &self,
        collection: &str,
        opts: &ListOptions,
    ) -> Result<RecordList> {
        let table = sanitize_ident(collection);

        let mut where_clauses = vec!["tenant_id = ?".to_string()];
        let mut params: Vec<JsValue> = vec![self.tenant_id.clone().into()];

        for f in &opts.filters {
            let col = sanitize_ident(&f.field);
            match f.operator {
                FilterOp::IsNull => where_clauses.push(format!("{} IS NULL", col)),
                FilterOp::IsNotNull => where_clauses.push(format!("{} IS NOT NULL", col)),
                _ => {
                    where_clauses.push(format!("{} {} ?", col, f.operator.as_sql()));
                    params.push(json_value_to_js(&f.value));
                }
            }
        }

        let where_sql = where_clauses.join(" AND ");

        // Count query
        let count_sql = format!("SELECT COUNT(*) as cnt FROM {} WHERE {}", table, where_sql);
        let count_stmt = self.db.prepare(&count_sql).bind(&params)?;
        let count_row = count_stmt.first::<serde_json::Value>(None).await?;
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

        let stmt = self.db.prepare(&sql).bind(&params)?;
        let results = stmt.all().await?;
        let rows: Vec<serde_json::Value> = results.results()?;

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

    /// Create a new record.
    pub async fn create(
        &self,
        collection: &str,
        data: HashMap<String, serde_json::Value>,
    ) -> Result<Record> {
        let table = sanitize_ident(collection);
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let mut columns = vec!["id".to_string(), "tenant_id".to_string()];
        let mut placeholders = vec!["?".to_string(), "?".to_string()];
        let mut params: Vec<JsValue> = vec![id.clone().into(), self.tenant_id.clone().into()];

        // Add created_at/updated_at if not provided
        let mut data = data;
        data.entry("created_at".to_string())
            .or_insert_with(|| serde_json::Value::String(now.clone()));
        data.entry("updated_at".to_string())
            .or_insert_with(|| serde_json::Value::String(now));

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

        self.db.prepare(&sql).bind(&params)?.run().await?;

        let mut record_data = data;
        record_data.insert(
            "tenant_id".to_string(),
            serde_json::Value::String(self.tenant_id.clone()),
        );
        Ok(Record {
            id,
            data: record_data,
        })
    }

    /// Update an existing record.
    pub async fn update(
        &self,
        collection: &str,
        id: &str,
        data: HashMap<String, serde_json::Value>,
    ) -> Result<Record> {
        let table = sanitize_ident(collection);
        let now = chrono::Utc::now().to_rfc3339();

        let mut sets = Vec::new();
        let mut params: Vec<JsValue> = Vec::new();

        let mut data = data;
        data.insert(
            "updated_at".to_string(),
            serde_json::Value::String(now),
        );

        for (key, val) in &data {
            sets.push(format!("{} = ?", sanitize_ident(key)));
            params.push(json_value_to_js(val));
        }

        params.push(id.into());
        params.push(self.tenant_id.clone().into());

        let sql = format!(
            "UPDATE {} SET {} WHERE id = ? AND tenant_id = ?",
            table,
            sets.join(", ")
        );

        self.db.prepare(&sql).bind(&params)?.run().await?;

        // Return updated record
        self.get(collection, id).await
    }

    /// Delete a record.
    pub async fn delete(&self, collection: &str, id: &str) -> Result<()> {
        let table = sanitize_ident(collection);
        let sql = format!(
            "DELETE FROM {} WHERE id = ? AND tenant_id = ?",
            table
        );

        self.db
            .prepare(&sql)
            .bind(&[id.into(), self.tenant_id.clone().into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Count records matching filters.
    pub async fn count(&self, collection: &str, filters: &[Filter]) -> Result<i64> {
        let table = sanitize_ident(collection);
        let mut where_clauses = vec!["tenant_id = ?".to_string()];
        let mut params: Vec<JsValue> = vec![self.tenant_id.clone().into()];

        for f in filters {
            let col = sanitize_ident(&f.field);
            match f.operator {
                FilterOp::IsNull => where_clauses.push(format!("{} IS NULL", col)),
                FilterOp::IsNotNull => where_clauses.push(format!("{} IS NOT NULL", col)),
                _ => {
                    where_clauses.push(format!("{} {} ?", col, f.operator.as_sql()));
                    params.push(json_value_to_js(&f.value));
                }
            }
        }

        let sql = format!(
            "SELECT COUNT(*) as cnt FROM {} WHERE {}",
            table,
            where_clauses.join(" AND ")
        );

        let row = self
            .db
            .prepare(&sql)
            .bind(&params)?
            .first::<serde_json::Value>(None)
            .await?;

        Ok(row
            .and_then(|v| v.get("cnt").and_then(|c| c.as_i64()))
            .unwrap_or(0))
    }

    /// Execute a raw SQL query.
    pub async fn query_raw(
        &self,
        query: &str,
        args: &[serde_json::Value],
    ) -> Result<Vec<Record>> {
        let params: Vec<JsValue> = args.iter().map(json_value_to_js).collect();
        let stmt = self.db.prepare(query).bind(&params)?;
        let results = stmt.all().await?;
        let rows: Vec<serde_json::Value> = results.results()?;
        Ok(rows.into_iter().map(json_to_record).collect())
    }

    /// Execute a raw SQL statement (INSERT/UPDATE/DELETE).
    pub async fn exec_raw(&self, query: &str, args: &[serde_json::Value]) -> Result<()> {
        let params: Vec<JsValue> = args.iter().map(json_value_to_js).collect();
        self.db.prepare(query).bind(&params)?.run().await?;
        Ok(())
    }

    /// Ensure a table exists with the given columns.
    pub async fn ensure_table(&self, name: &str, columns_sql: &str) -> Result<()> {
        let table = sanitize_ident(name);
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, {})",
            table, columns_sql
        );
        self.db.prepare(&sql).bind(&[])?.run().await?;

        // Ensure tenant_id index
        let idx_sql = format!(
            "CREATE INDEX IF NOT EXISTS idx_{}_tenant ON {} (tenant_id)",
            name, table
        );
        self.db.prepare(&idx_sql).bind(&[])?.run().await?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Sanitize an identifier for safe SQL interpolation (table/column names).
fn sanitize_ident(name: &str) -> String {
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

/// Convert a D1 result row (as JSON) into a Record.
fn json_to_record(val: serde_json::Value) -> Record {
    if let serde_json::Value::Object(mut map) = val {
        let id = map
            .remove("id")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();

        // Remove tenant_id from data (internal field)
        map.remove("tenant_id");

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
