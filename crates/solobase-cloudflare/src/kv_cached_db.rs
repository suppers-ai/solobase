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
