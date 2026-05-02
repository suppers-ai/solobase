//! `BrowserVectorService` — sql.js-backed `VectorService`.
//!
//! Vectors are stored as `BLOB` columns in the shared OPFS sql.js database
//! (no separate file). Scoring is in-process Rust using SIMD on wasm32. FTS5
//! powers keyword search when the index has `keyword_search: true`.

use std::collections::HashMap;
use std::sync::Mutex;

use wafer_core::interfaces::vector::service::{
    DistanceMetric, MetadataFilter, Result as VResult, SearchMode, VectorEntry, VectorError,
    VectorIndexConfig, VectorMatch, VectorService,
};

use crate::bridge;
use crate::vector::sql;

fn js_err(e: wasm_bindgen::JsValue) -> String {
    e.as_string().unwrap_or_else(|| format!("{e:?}"))
}

/// Per-index config cached in memory after `create_index`. Persisted via the
/// `wafer_core::interfaces::vector` block's own registry table; this cache
/// is hydrated on first use by reading that registry.
#[derive(Clone)]
struct IndexState {
    dimensions: u32,
    metric: DistanceMetric,
    keyword_search: bool,
}

pub struct BrowserVectorService {
    indexes: Mutex<HashMap<String, IndexState>>,
}

// Safety: wasm32-unknown-unknown is single-threaded — no data races possible.
unsafe impl Send for BrowserVectorService {}
unsafe impl Sync for BrowserVectorService {}

impl Default for BrowserVectorService {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserVectorService {
    pub fn new() -> Self {
        Self {
            indexes: Mutex::new(HashMap::new()),
        }
    }

    fn lookup(&self, name: &str) -> Option<IndexState> {
        self.indexes.lock().ok()?.get(name).cloned()
    }
}

#[async_trait::async_trait(?Send)]
impl VectorService for BrowserVectorService {
    async fn create_index(&self, config: VectorIndexConfig) -> VResult<()> {
        let stmts = sql::build_create_index_sql(&config.name, config.keyword_search);
        for s in stmts {
            bridge::db_exec_raw(&s, "[]").map_err(|e| VectorError::Internal(js_err(e)))?;
        }
        bridge::dbFlush().await;
        self.indexes.lock().unwrap().insert(
            config.name.clone(),
            IndexState {
                dimensions: config.dimensions,
                metric: config.metric,
                keyword_search: config.keyword_search,
            },
        );
        Ok(())
    }

    async fn delete_index(&self, name: &str) -> VResult<()> {
        let state = self
            .lookup(name)
            .ok_or_else(|| VectorError::IndexNotFound(name.into()))?;
        let stmts = sql::build_delete_index_sql(name, state.keyword_search);
        for s in stmts {
            bridge::db_exec_raw(&s, "[]").map_err(|e| VectorError::Internal(js_err(e)))?;
        }
        bridge::dbFlush().await;
        self.indexes.lock().unwrap().remove(name);
        Ok(())
    }

    async fn upsert(&self, index: &str, entries: Vec<VectorEntry>) -> VResult<()> {
        let state = self
            .lookup(index)
            .ok_or_else(|| VectorError::IndexNotFound(index.into()))?;

        use base64ct::{Base64, Encoding};
        let prepared: Result<Vec<sql::SqlUpsertEntry>, VectorError> = entries
            .iter()
            .map(|e| {
                if e.vector.len() as u32 != state.dimensions {
                    return Err(VectorError::DimensionMismatch {
                        expected: state.dimensions,
                        got: e.vector.len() as u32,
                    });
                }
                if state.keyword_search && e.text.is_none() {
                    return Err(VectorError::TextRequired);
                }
                let blob = sql::pack_vector_blob(&e.vector);
                Ok(sql::SqlUpsertEntry {
                    id: e.id.clone(),
                    vector_blob_b64: Base64::encode_string(&blob),
                    metadata_json: e
                        .metadata
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| "{}".into()),
                    text: e.text.clone(),
                })
            })
            .collect();
        let prepared = prepared?;

        for stmt in sql::build_upsert_sql_stmts(index, state.keyword_search, &prepared) {
            bridge::db_exec_raw(&stmt.sql, &stmt.params_json)
                .map_err(|e| VectorError::Internal(js_err(e)))?;
        }
        bridge::dbFlush().await;
        Ok(())
    }

    async fn query(
        &self,
        _index: &str,
        _vector: Vec<f32>,
        _top_k: usize,
        _filter: Option<MetadataFilter>,
        _mode: SearchMode,
        _keyword_query: Option<String>,
    ) -> VResult<Vec<VectorMatch>> {
        Err(VectorError::Internal("query not yet implemented".into()))
    }

    async fn delete(&self, index: &str, ids: Vec<String>) -> VResult<()> {
        let state = self
            .lookup(index)
            .ok_or_else(|| VectorError::IndexNotFound(index.into()))?;
        if ids.is_empty() {
            return Ok(());
        }
        let (stmts, params) = sql::build_delete_ids_sql(index, &ids, state.keyword_search);
        let params_json = serde_json::to_string(&params).unwrap_or_else(|_| "[]".into());
        for s in stmts {
            bridge::db_exec_raw(&s, &params_json)
                .map_err(|e| VectorError::Internal(js_err(e)))?;
        }
        bridge::dbFlush().await;
        Ok(())
    }

    async fn count(&self, index: &str) -> VResult<u64> {
        if self.lookup(index).is_none() {
            return Err(VectorError::IndexNotFound(index.into()));
        }
        let row_json = bridge::db_query_raw(&sql::build_count_sql(index), "[]")
            .map_err(|e| VectorError::Internal(js_err(e)))?;
        // sql.js returns rows as `[{ "n": <number> }]`.
        let rows: Vec<serde_json::Value> = serde_json::from_str(&row_json)
            .map_err(|e| VectorError::Internal(format!("parse count: {e}")))?;
        let n = rows
            .first()
            .and_then(|r| r.get("n"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        Ok(n)
    }
}

