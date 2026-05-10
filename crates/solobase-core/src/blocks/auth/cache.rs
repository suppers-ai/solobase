//! In-process cache for `verify_org_admin` results.
//!
//! Keys are `(user_id, provider, verified_ref)`. Values are the boolean
//! "is admin" outcome plus an `Instant` at which the entry was inserted;
//! entries expire after a configurable TTL (5 minutes in production).
//!
//! Both positive and negative outcomes are cached. The current
//! `verify_org_admin` implementation is fully DB-driven (reserved-org check
//! + owner short-circuit) and never returns a transient error, so every
//! computed answer is safe to memoize.
//!
//! Thread-safety is provided by a single `Mutex<HashMap<…>>`. At launch
//! traffic the lock contention is negligible; if that changes we can swap
//! in `DashMap` behind the same surface without touching callers.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use wafer_core::interfaces::auth::service::UserId;

type Key = (String, String, String); // (user_id, provider, verified_ref)

/// 5-minute TTL in production. Exposed as a constant so the wiring site
/// (block construction) and tests agree on the default.
pub const DEFAULT_TTL: Duration = Duration::from_secs(300);

#[derive(Clone)]
pub struct OrgAdminCache {
    ttl: Duration,
    inner: Arc<Mutex<HashMap<Key, (bool, Instant)>>>,
}

impl Default for OrgAdminCache {
    fn default() -> Self {
        Self::new(DEFAULT_TTL)
    }
}

impl OrgAdminCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Returns the cached value if one is present and still within TTL.
    /// Expired entries are evicted on read so the map doesn't grow
    /// unboundedly through stale reads.
    pub fn get(&self, user: &UserId, provider: &str, verified_ref: &str) -> Option<bool> {
        let key = (user.0.clone(), provider.into(), verified_ref.into());
        let mut guard = self.inner.lock().expect("OrgAdminCache mutex poisoned");
        match guard.get(&key) {
            Some(&(value, inserted_at)) if inserted_at.elapsed() < self.ttl => Some(value),
            Some(_) => {
                guard.remove(&key);
                None
            }
            None => None,
        }
    }

    /// Insert or overwrite. Both `true` and `false` are cached so a
    /// non-admin who probes the publish endpoint doesn't hammer the
    /// upstream provider's API every request.
    pub fn insert(&self, user: &UserId, provider: &str, verified_ref: &str, value: bool) {
        let key = (user.0.clone(), provider.into(), verified_ref.into());
        let mut guard = self.inner.lock().expect("OrgAdminCache mutex poisoned");
        guard.insert(key, (value, Instant::now()));
    }

    /// Drop every entry whose first key component matches `user`. Called
    /// from the logout handler so a user who had admin privileges revoked
    /// upstream doesn't retain them for up to `ttl` after signing out.
    pub fn invalidate_user(&self, user: &UserId) {
        let mut guard = self.inner.lock().expect("OrgAdminCache mutex poisoned");
        guard.retain(|(uid, _, _), _| uid != &user.0);
    }

    /// Test-only: number of live entries. Expired entries aren't pruned
    /// until they're probed via `get`, so the returned count can include
    /// stale rows — callers use this to assert "empty after invalidate".
    #[cfg(test)]
    fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uid(s: &str) -> UserId {
        UserId(s.into())
    }

    #[test]
    fn miss_when_empty() {
        let cache = OrgAdminCache::new(Duration::from_secs(300));
        assert!(cache.get(&uid("u1"), "github", "acme").is_none());
    }

    #[test]
    fn insert_then_hit_true() {
        let cache = OrgAdminCache::new(Duration::from_secs(300));
        cache.insert(&uid("u1"), "github", "acme", true);
        assert_eq!(cache.get(&uid("u1"), "github", "acme"), Some(true));
    }

    #[test]
    fn insert_false_is_cached_too() {
        let cache = OrgAdminCache::new(Duration::from_secs(300));
        cache.insert(&uid("u1"), "github", "acme", false);
        assert_eq!(cache.get(&uid("u1"), "github", "acme"), Some(false));
    }

    #[test]
    fn expired_entry_is_a_miss_and_evicted() {
        let cache = OrgAdminCache::new(Duration::from_millis(1));
        cache.insert(&uid("u1"), "github", "acme", true);
        std::thread::sleep(Duration::from_millis(10));
        assert!(cache.get(&uid("u1"), "github", "acme").is_none());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn invalidate_user_drops_only_that_users_entries() {
        let cache = OrgAdminCache::new(Duration::from_secs(300));
        cache.insert(&uid("u1"), "github", "acme", true);
        cache.insert(&uid("u1"), "github", "widgets", true);
        cache.insert(&uid("u2"), "github", "acme", true);
        cache.invalidate_user(&uid("u1"));
        assert!(cache.get(&uid("u1"), "github", "acme").is_none());
        assert!(cache.get(&uid("u1"), "github", "widgets").is_none());
        assert_eq!(cache.get(&uid("u2"), "github", "acme"), Some(true));
    }

    #[test]
    fn different_verified_ref_is_a_different_entry() {
        let cache = OrgAdminCache::new(Duration::from_secs(300));
        cache.insert(&uid("u1"), "github", "acme", true);
        assert!(cache.get(&uid("u1"), "github", "widgets").is_none());
    }

    #[test]
    fn different_provider_is_a_different_entry() {
        let cache = OrgAdminCache::new(Duration::from_secs(300));
        cache.insert(&uid("u1"), "github", "acme", true);
        assert!(cache.get(&uid("u1"), "gitlab", "acme").is_none());
    }

    #[test]
    fn concurrent_put_and_get_does_not_panic() {
        // Smoke check for the Mutex path under light contention. 100 writers
        // + 100 readers racing for 200 ms is enough to trip any obvious
        // poisoning/double-lock bug.
        use std::sync::Arc as StdArc;
        let cache = StdArc::new(OrgAdminCache::new(Duration::from_secs(300)));
        let mut handles = Vec::new();
        for i in 0..100 {
            let c = cache.clone();
            handles.push(std::thread::spawn(move || {
                for j in 0..10 {
                    c.insert(&uid(&format!("u{i}")), "github", "acme", j % 2 == 0);
                }
            }));
        }
        for i in 0..100 {
            let c = cache.clone();
            handles.push(std::thread::spawn(move || {
                for _ in 0..10 {
                    let _ = c.get(&uid(&format!("u{i}")), "github", "acme");
                }
            }));
        }
        for h in handles {
            h.join().expect("worker thread panicked");
        }
    }
}
