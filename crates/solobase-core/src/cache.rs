//! TTL-based per-instance cache.
//!
//! Used by solobase-cloudflare to memoize D1 reads across requests within
//! a Worker isolate's lifetime. Generic so the unit tests live here on the
//! native target — the cloudflare crate has no `cargo test` surface.

use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::sync::Arc;

pub struct TtlCache<T> {
    state: Mutex<Option<(Arc<T>, Instant)>>,
    ttl: Duration,
}

impl<T> TtlCache<T> {
    pub const fn new(ttl: Duration) -> Self {
        Self {
            state: Mutex::new(None),
            ttl,
        }
    }

    /// Returns the cached value if fresh, otherwise runs `fetcher` and caches the result.
    pub async fn get_or_load<F, Fut>(&self, fetcher: F) -> Arc<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        if let Some((value, loaded_at)) = &*self.state.lock().expect("TtlCache poisoned") {
            if loaded_at.elapsed() < self.ttl {
                return Arc::clone(value);
            }
        }
        let fresh = fetcher().await;
        let arc = Arc::new(fresh);
        let mut guard = self.state.lock().expect("TtlCache poisoned");
        *guard = Some((Arc::clone(&arc), Instant::now()));
        arc
    }

    /// Force the next call to refetch. Used by admin-write paths.
    pub fn invalidate(&self) {
        *self.state.lock().expect("TtlCache poisoned") = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn first_call_runs_fetcher() {
        let cache: TtlCache<u32> = TtlCache::new(Duration::from_secs(60));
        let calls = AtomicUsize::new(0);
        let v = cache.get_or_load(|| async {
            calls.fetch_add(1, Ordering::SeqCst);
            42
        }).await;
        assert_eq!(*v, 42);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
