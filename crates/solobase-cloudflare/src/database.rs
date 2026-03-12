//! Async database service backed by Cloudflare D1 (SQLite at the edge).
//!
//! Implements the shared `DatabaseService` trait from wafer-core so D1Block
//! can reuse the shared message handler.

use std::collections::HashMap;

use wasm_bindgen::JsValue;
use worker::*;

use wafer_core::interfaces::database::service::{
    Column, DatabaseError, DatabaseService, Filter, FilterOp, ListOptions, Record, RecordList,
    SortField, Table,
};

/// Async database service wrapping Cloudflare D1.
pub struct D1DatabaseService {
    db: D1Database,
}

impl D1DatabaseService {
    pub fn new(db: D1Database) -> Self {
        Self { db }
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
            .map_err(|e| DatabaseError::Internal(e.to_string()))?;

        let row = stmt
            .first::<serde_json::Value>(None)
            .await
            .map_err(|e| DatabaseError::Internal(e.to_string()))?;
        match row {
            Some(val) => Ok(json_to_record(val)),
            None => Err(DatabaseError::NotFound),
        }
    }

    async fn list(&self, collection: &str, opts: &ListOptions) -> Result<RecordList, DatabaseError> {
        let table = sanitize_ident(collection);

        let mut where_clauses: Vec<String> = Vec::new();
        let mut params: Vec<JsValue> = Vec::new();

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

        let where_sql = if where_clauses.is_empty() {
            "1=1".to_string()
        } else {
            where_clauses.join(" AND ")
        };

        // Count query
        let count_sql = format!("SELECT COUNT(*) as cnt FROM {} WHERE {}", table, where_sql);
        let count_stmt = self
            .db
            .prepare(&count_sql)
            .bind(&params)
            .map_err(|e| DatabaseError::Internal(e.to_string()))?;
        let count_row = count_stmt
            .first::<serde_json::Value>(None)
            .await
            .map_err(|e| DatabaseError::Internal(e.to_string()))?;
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

        let stmt = self
            .db
            .prepare(&sql)
            .bind(&params)
            .map_err(|e| DatabaseError::Internal(e.to_string()))?;
        let results = stmt
            .all()
            .await
            .map_err(|e| DatabaseError::Internal(e.to_string()))?;
        let rows: Vec<serde_json::Value> = results
            .results()
            .map_err(|e| DatabaseError::Internal(e.to_string()))?;

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

        let mut columns = vec!["id".to_string()];
        let mut placeholders = vec!["?".to_string()];
        let mut params: Vec<JsValue> = vec![id.clone().into()];

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

        self.db
            .prepare(&sql)
            .bind(&params)
            .map_err(|e| DatabaseError::Internal(e.to_string()))?
            .run()
            .await
            .map_err(|e| DatabaseError::Internal(e.to_string()))?;

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
        data.insert(
            "updated_at".to_string(),
            serde_json::Value::String(now),
        );

        for (key, val) in &data {
            sets.push(format!("{} = ?", sanitize_ident(key)));
            params.push(json_value_to_js(val));
        }

        params.push(id.into());

        let sql = format!(
            "UPDATE {} SET {} WHERE id = ?",
            table,
            sets.join(", ")
        );

        self.db
            .prepare(&sql)
            .bind(&params)
            .map_err(|e| DatabaseError::Internal(e.to_string()))?
            .run()
            .await
            .map_err(|e| DatabaseError::Internal(e.to_string()))?;

        self.get(collection, id).await
    }

    async fn delete(&self, collection: &str, id: &str) -> Result<(), DatabaseError> {
        let table = sanitize_ident(collection);
        let sql = format!("DELETE FROM {} WHERE id = ?", table);

        self.db
            .prepare(&sql)
            .bind(&[id.into()])
            .map_err(|e| DatabaseError::Internal(e.to_string()))?
            .run()
            .await
            .map_err(|e| DatabaseError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn count(&self, collection: &str, filters: &[Filter]) -> Result<i64, DatabaseError> {
        let table = sanitize_ident(collection);
        let mut where_clauses: Vec<String> = Vec::new();
        let mut params: Vec<JsValue> = Vec::new();

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

        let where_sql = if where_clauses.is_empty() {
            "1=1".to_string()
        } else {
            where_clauses.join(" AND ")
        };

        let sql = format!("SELECT COUNT(*) as cnt FROM {} WHERE {}", table, where_sql);

        let row = self
            .db
            .prepare(&sql)
            .bind(&params)
            .map_err(|e| DatabaseError::Internal(e.to_string()))?
            .first::<serde_json::Value>(None)
            .await
            .map_err(|e| DatabaseError::Internal(e.to_string()))?;

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
        let mut where_parts: Vec<String> = Vec::new();
        let mut args: Vec<JsValue> = Vec::new();

        for f in filters {
            let fc = sanitize_ident(&f.field);
            match f.operator {
                FilterOp::IsNull => where_parts.push(format!("{} IS NULL", fc)),
                FilterOp::IsNotNull => where_parts.push(format!("{} IS NOT NULL", fc)),
                _ => {
                    where_parts.push(format!("{} {} ?", fc, f.operator.as_sql()));
                    args.push(json_value_to_js(&f.value));
                }
            }
        }

        let where_sql = if where_parts.is_empty() {
            "1=1".to_string()
        } else {
            where_parts.join(" AND ")
        };

        let sql = format!(
            "SELECT COALESCE(SUM({}), 0) as s FROM {} WHERE {}",
            col, table, where_sql
        );

        let row = self
            .db
            .prepare(&sql)
            .bind(&args)
            .map_err(|e| DatabaseError::Internal(e.to_string()))?
            .first::<serde_json::Value>(None)
            .await
            .map_err(|e| DatabaseError::Internal(e.to_string()))?;

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
        let stmt = self
            .db
            .prepare(query)
            .bind(&params)
            .map_err(|e| DatabaseError::Internal(e.to_string()))?;
        let results = stmt
            .all()
            .await
            .map_err(|e| DatabaseError::Internal(e.to_string()))?;
        let rows: Vec<serde_json::Value> = results
            .results()
            .map_err(|e| DatabaseError::Internal(e.to_string()))?;
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
            .map_err(|e| DatabaseError::Internal(e.to_string()))?
            .run()
            .await
            .map_err(|e| DatabaseError::Internal(e.to_string()))?;
        Ok(0)
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
