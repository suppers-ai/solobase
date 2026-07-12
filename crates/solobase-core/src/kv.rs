//! Pluggable KV backend trait + config-version-stamp retry helper for the
//! Cloudflare config-var hot path.
//!
//! Lives in `solobase-core` (not `solobase-cloudflare`) so the retry logic is
//! host-testable: `solobase-cloudflare` is wasm32-only and excluded from
//! `cargo test --workspace`. Follows the `cache_key` extraction precedent —
//! pure logic that a wasm-only crate would otherwise leave untested moves
//! here.
//!
//! The production [`KvBackend`] impl (`WorkerKvBackend`, over
//! `worker::kv::KvStore`) and the `KvCachedD1DatabaseService` that consume this
//! trait stay in `solobase-cloudflare`, which owns the `worker` dependency.

use wafer_block::{MaybeSend, MaybeSync};

/// Pluggable KV backend. Production (`solobase-cloudflare`) uses
/// `worker::kv::KvStore` via `WorkerKvBackend`; host tests use the in-module
/// `MockKvBackend` (see the `#[cfg(test)] mod tests` below).
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

    /// Writes `value` under `key` with no expiration. Reserved for the
    /// config-version stamp — row-cache entries always carry a TTL.
    async fn put(&self, key: &str, value: &str) -> Result<(), String>;

    /// Deletes `key`. Deleting a non-existent key is not an error.
    async fn delete(&self, key: &str) -> Result<(), String>;
}

/// PUT `stamp` under [`crate::cache_key::CONFIG_VERSION_KEY`] persistently (no
/// TTL — a quiet day must not expire the stamp and trigger a fleet-wide
/// restamp), retrying once on KV transport error.
///
/// On a double failure the returned error names both attempts
/// (`"first attempt: …; retry: …"`) so log correlation can distinguish a
/// one-off transient blip (which the retry papers over silently) from a
/// sustained KV outage.
pub async fn put_version_stamp_with_retry(kv: &dyn KvBackend, stamp: &str) -> Result<(), String> {
    match kv.put(crate::cache_key::CONFIG_VERSION_KEY, stamp).await {
        Ok(()) => Ok(()),
        Err(first) => kv
            .put(crate::cache_key::CONFIG_VERSION_KEY, stamp)
            .await
            .map_err(|second| format!("first attempt: {first}; retry: {second}")),
    }
}

