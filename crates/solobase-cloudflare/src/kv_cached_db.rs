//! `KvCachedD1DatabaseService` — wraps a `DatabaseService` with a
//! Cloudflare KV cache for the per-block config-var hot path.
//!
//! See `docs/superpowers/specs/2026-05-22-kv-cached-d1-config-source-design.md`.
//!
//! Pure cache-key derivation lives in `solobase_core::cache_key` so it can
//! be unit-tested on host (this crate is excluded from `cargo test --workspace`).

use wafer_block::{MaybeSend, MaybeSync};

/// Pluggable KV backend. Production uses `worker::kv::KvStore` via
/// `WorkerKvBackend`; tests use `MockKvBackend` (see `tests/support`).
///
/// All errors are returned as `String` and never propagate as a hard
/// failure to callers — `KvCachedD1DatabaseService` treats every KV
/// error as a cache miss and falls through to the underlying database.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait KvBackend: MaybeSend + MaybeSync {
    /// Returns `Ok(Some(value))` on cache hit, `Ok(None)` on miss,
    /// `Err(msg)` on KV transport error.
    async fn get(&self, key: &str) -> Result<Option<String>, String>;

    /// Writes `value` under `key` with the given TTL (seconds).
    async fn put_with_ttl(&self, key: &str, value: &str, ttl_secs: u64) -> Result<(), String>;

    /// Deletes `key`. Deleting a non-existent key is not an error.
    async fn delete(&self, key: &str) -> Result<(), String>;
}

/// Production `KvBackend` backed by `worker::kv::KvStore`.
pub struct WorkerKvBackend(pub worker::kv::KvStore);

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl KvBackend for WorkerKvBackend {
    async fn get(&self, key: &str) -> Result<Option<String>, String> {
        self.0.get(key).text().await.map_err(|e| e.to_string())
    }

    async fn put_with_ttl(&self, key: &str, value: &str, ttl_secs: u64) -> Result<(), String> {
        self.0
            .put(key, value)
            .map_err(|e| e.to_string())?
            .expiration_ttl(ttl_secs)
            .execute()
            .await
            .map_err(|e| e.to_string())
    }

    async fn delete(&self, key: &str) -> Result<(), String> {
        self.0.delete(key).await.map_err(|e| e.to_string())
    }
}

// ---------------------------------------------------------------------------
// KvCachedD1DatabaseService
// ---------------------------------------------------------------------------

use std::{collections::HashMap, sync::Arc};

use solobase_core::cache_key;
use wafer_block::db::{Filter, ListOptions};
use wafer_core::interfaces::database::service::{
    Column, DatabaseError, DatabaseService, Record, RecordList, Table,
};

/// KV TTL applied to every cache PUT (24 h).
const CACHE_TTL_SECS: u64 = 86_400;

/// Wraps a [`DatabaseService`] with a write-through-invalidated KV cache
/// for the `variables` and `block_settings` per-block read paths.
pub struct KvCachedD1DatabaseService {
    inner: Arc<dyn DatabaseService>,
    kv: Arc<dyn KvBackend>,
}

impl KvCachedD1DatabaseService {
    /// Wrap `inner` with a KV-backed cache using `kv`.
    pub fn new(inner: Arc<dyn DatabaseService>, kv: Arc<dyn KvBackend>) -> Self {
        Self { inner, kv }
    }

    /// Delete every cache key in `keys`, logging (but never failing on) KV
    /// errors. The underlying mutation has already committed by the time this
    /// runs; a failed invalidation just leaves the stale entry to expire on
    /// its 24 h TTL. `op` names the originating write for log correlation.
    async fn invalidate_all(&self, collection: &str, keys: &[String], op: &str) {
        for key in keys {
            if let Err(e) = self.kv.delete(key).await {
                tracing::warn!(
                    table = %collection,
                    key = %key,
                    error = %e,
                    op,
                    "kv invalidate failed; relying on TTL"
                );
            } else {
                tracing::debug!(table = %collection, key = %key, op, "cache_invalidate");
            }
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl DatabaseService for KvCachedD1DatabaseService {
    async fn get(&self, collection: &str, id: &str) -> Result<Record, DatabaseError> {
        self.inner.get(collection, id).await
    }

    async fn count(&self, collection: &str, filters: &[Filter]) -> Result<i64, DatabaseError> {
        self.inner.count(collection, filters).await
    }

    async fn sum(
        &self,
        collection: &str,
        field: &str,
        filters: &[Filter],
    ) -> Result<f64, DatabaseError> {
        self.inner.sum(collection, field, filters).await
    }

    async fn query_raw(
        &self,
        query: &str,
        args: &[serde_json::Value],
    ) -> Result<Vec<Record>, DatabaseError> {
        self.inner.query_raw(query, args).await
    }

    async fn exec_raw(
        &self,
        query: &str,
        args: &[serde_json::Value],
    ) -> Result<i64, DatabaseError> {
        self.inner.exec_raw(query, args).await
    }

    async fn increment_field_where(
        &self,
        collection: &str,
        col: &str,
        delta: i64,
        filters: &[Filter],
    ) -> Result<i64, DatabaseError> {
        // MUST override — trait default returns Err(Internal).
        self.inner
            .increment_field_where(collection, col, delta, filters)
            .await
    }

    async fn ensure_schema_table(&self, table: &Table) -> Result<(), DatabaseError> {
        self.inner.ensure_schema_table(table).await
    }

    async fn schema_table_exists(&self, name: &str) -> Result<bool, DatabaseError> {
        self.inner.schema_table_exists(name).await
    }

    async fn schema_drop_table(&self, name: &str) -> Result<(), DatabaseError> {
        self.inner.schema_drop_table(name).await
    }

    async fn schema_add_column(&self, table: &str, column: &Column) -> Result<(), DatabaseError> {
        self.inner.schema_add_column(table, column).await
    }

    // Bulk-write ops on cached tables hard-error to avoid silent stale-cache footguns.
    async fn delete_where(
        &self,
        collection: &str,
        filters: &[Filter],
    ) -> Result<(), DatabaseError> {
        if cache_key::classify_table(collection).is_some() {
            return Err(DatabaseError::Internal(format!(
                "bulk delete_where not supported on cached table `{collection}` \
                 (would require KV mass-invalidation)"
            )));
        }
        self.inner.delete_where(collection, filters).await
    }

    async fn update_where(
        &self,
        collection: &str,
        filters: &[Filter],
        data: HashMap<String, serde_json::Value>,
    ) -> Result<(), DatabaseError> {
        if cache_key::classify_table(collection).is_some() {
            return Err(DatabaseError::Internal(format!(
                "bulk update_where not supported on cached table `{collection}` \
                 (would require KV mass-invalidation)"
            )));
        }
        self.inner.update_where(collection, filters, data).await
    }

    async fn list(
        &self,
        collection: &str,
        opts: &ListOptions,
    ) -> Result<RecordList, DatabaseError> {
        let cache_key_opt =
            cache_key::classify_table(collection).and_then(|t| cache_key::read_key(t, opts));

        let Some(key) = cache_key_opt else {
            return self.inner.list(collection, opts).await;
        };

        // Try cache hit.
        match self.kv.get(&key).await {
            Ok(Some(body)) => match serde_json::from_str::<Vec<Record>>(&body) {
                Ok(records) => {
                    let page_size = opts.limit;
                    let total_count = records.len() as i64;
                    tracing::debug!(table = %collection, key = %key, "cache_hit");
                    return Ok(RecordList {
                        records,
                        total_count,
                        page: 1,
                        page_size,
                    });
                }
                Err(e) => {
                    tracing::warn!(
                        table = %collection,
                        key = %key,
                        error = %e,
                        "cache value deserialize failed; falling through"
                    );
                }
            },
            Ok(None) => {
                tracing::debug!(table = %collection, key = %key, "cache_miss");
            }
            Err(e) => {
                tracing::warn!(
                    table = %collection,
                    key = %key,
                    error = %e,
                    "kv get failed; falling through"
                );
            }
        }

        // Fall through to D1 and populate cache.
        let result = self.inner.list(collection, opts).await?;
        let payload = match serde_json::to_string(&result.records) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "cache payload serialize failed; skipping put");
                return Ok(result);
            }
        };
        if let Err(e) = self.kv.put_with_ttl(&key, &payload, CACHE_TTL_SECS).await {
            tracing::warn!(
                table = %collection,
                key = %key,
                error = %e,
                "kv put failed; cache stays cold"
            );
        }
        Ok(result)
    }

    async fn create(
        &self,
        collection: &str,
        data: HashMap<String, serde_json::Value>,
    ) -> Result<Record, DatabaseError> {
        let cached = cache_key::classify_table(collection);
        let invalidate = cached
            .map(|t| cache_key::invalidate_keys(t, &data))
            .unwrap_or_default();

        let record = self.inner.create(collection, data).await?;

        if invalidate.is_empty() && cached.is_some() {
            tracing::warn!(
                table = %collection,
                "cached table write with no extractable cache-key column; relying on TTL"
            );
        }
        self.invalidate_all(collection, &invalidate, "create").await;

        Ok(record)
    }

    async fn update(
        &self,
        collection: &str,
        id: &str,
        data: HashMap<String, serde_json::Value>,
    ) -> Result<Record, DatabaseError> {
        let cached = cache_key::classify_table(collection);
        let invalidate = if let Some(t) = cached {
            match self.inner.get(collection, id).await {
                Ok(row) => cache_key::invalidate_keys(t, &row.data),
                Err(e) => {
                    tracing::warn!(
                        table = %collection,
                        id = %id,
                        error = %e,
                        "pre-read for cache-key failed; skipping invalidation"
                    );
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        let record = self.inner.update(collection, id, data).await?;

        self.invalidate_all(collection, &invalidate, "update").await;

        Ok(record)
    }

    async fn delete(&self, collection: &str, id: &str) -> Result<(), DatabaseError> {
        let cached = cache_key::classify_table(collection);
        let invalidate = if let Some(t) = cached {
            match self.inner.get(collection, id).await {
                Ok(row) => cache_key::invalidate_keys(t, &row.data),
                Err(e) => {
                    tracing::warn!(
                        table = %collection,
                        id = %id,
                        error = %e,
                        "pre-read for cache-key failed; skipping invalidation"
                    );
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        self.inner.delete(collection, id).await?;

        self.invalidate_all(collection, &invalidate, "delete").await;

        Ok(())
    }
}
