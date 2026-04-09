use std::time::Duration;
use wafer_core::clients::config;
use wafer_run::context::Context;

#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(target_arch = "wasm32")]
use wafer_core::clients::database as db;

/// Per-user rate limiter using fixed-window counters.
///
/// Keyed by a composite string (typically `user_id:category`).
/// Separate from wafer-core's per-IP rate limiter which runs as middleware.
/// On native, uses in-memory counters (Mutex<HashMap>).
/// On wasm32 (Cloudflare Workers), uses D1-backed counters via the
/// `suppers_ai__auth__rate_limits` collection.
pub struct UserRateLimiter {
    #[cfg(not(target_arch = "wasm32"))]
    buckets: Mutex<HashMap<String, RateBucket>>,
}

#[cfg(not(target_arch = "wasm32"))]
struct RateBucket {
    count: u32,
    window_start: Instant,
}

/// Rate limit configuration: max requests allowed within a time window.
///
/// Configurable via env vars using the format `RATE_LIMIT_{NAME}=requests/seconds`.
/// For example: `RATE_LIMIT_AUTH=20/60` means 20 requests per 60 seconds.
/// Set `RATE_LIMIT_{NAME}=0` to disable rate limiting for that category.
#[derive(Debug, Clone, Copy)]
pub struct RateLimit {
    pub max_requests: u32,
    pub window: Duration,
}

impl RateLimit {
    /// Login and signup: 30 requests per 60 seconds per IP.
    pub const AUTH: Self = Self {
        max_requests: 30,
        window: Duration::from_secs(60),
    };
    /// Token refresh: 30 requests per 60 seconds per IP.
    pub const REFRESH: Self = Self {
        max_requests: 30,
        window: Duration::from_secs(60),
    };
    /// API reads: 300 requests per 60 seconds per user.
    pub const API_READ: Self = Self {
        max_requests: 300,
        window: Duration::from_secs(60),
    };
    /// API writes (create/update/delete): 120 requests per 60 seconds per user.
    pub const API_WRITE: Self = Self {
        max_requests: 120,
        window: Duration::from_secs(60),
    };
    /// File uploads: 60 requests per 60 seconds per user.
    pub const UPLOAD: Self = Self {
        max_requests: 60,
        window: Duration::from_secs(60),
    };

    /// Read config override for this rate limit category.
    ///
    /// Looks up `RATE_LIMIT_{name}` in config. Format: `requests/seconds` (e.g. `50/60`).
    /// Set to `0` to disable rate limiting for this category.
    /// Returns `None` if disabled, otherwise the resolved limit.
    pub async fn resolve(self, ctx: &dyn Context, name: &str) -> Option<Self> {
        let key = format!("SOLOBASE_SHARED__RATE_LIMIT_{}", name.to_uppercase());
        let default = format!("{}/{}", self.max_requests, self.window.as_secs());
        let value = config::get_default(ctx, &key, &default).await;

        // "0" disables this category
        if value.trim() == "0" {
            return None;
        }

        if let Some((req_str, sec_str)) = value.split_once('/') {
            let max = req_str.trim().parse::<u32>().unwrap_or(self.max_requests);
            if max == 0 {
                return None;
            }
            let secs = sec_str
                .trim()
                .parse::<u64>()
                .unwrap_or(self.window.as_secs());
            Some(Self {
                max_requests: max,
                window: Duration::from_secs(secs),
            })
        } else {
            // Just a number = override max requests, keep default window
            let max = value.trim().parse::<u32>().unwrap_or(self.max_requests);
            if max == 0 {
                return None;
            }
            Some(Self {
                max_requests: max,
                window: self.window,
            })
        }
    }
}

impl Default for UserRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl UserRateLimiter {
    pub fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Check rate limit for a given key. Returns `Ok(remaining)` if allowed,
    /// or `Err(retry_after_secs)` if the limit is exceeded.
    ///
    /// On native, uses in-memory counters (Mutex<HashMap>).
    /// On wasm32 (Cloudflare Workers), uses D1-backed counters.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn check(&self, _ctx: &dyn Context, key: &str, limit: RateLimit) -> Result<u32, u64> {
        let mut buckets = self.buckets.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();

        // Evict expired entries when map gets large
        if buckets.len() > 5_000 {
            buckets.retain(|_, b| now.duration_since(b.window_start) <= limit.window);
        }
        // Hard cap
        if buckets.len() > 50_000 {
            buckets.clear();
        }

        let bucket = buckets.entry(key.to_string()).or_insert(RateBucket {
            count: 0,
            window_start: now,
        });

        // Reset window if expired
        if now.duration_since(bucket.window_start) > limit.window {
            bucket.count = 0;
            bucket.window_start = now;
        }

        bucket.count += 1;

        if bucket.count > limit.max_requests {
            let remaining = limit
                .window
                .checked_sub(now.duration_since(bucket.window_start))
                .unwrap_or(Duration::ZERO);
            Err(remaining.as_secs().max(1))
        } else {
            Ok(limit.max_requests - bucket.count)
        }
    }

    /// On wasm32 (Cloudflare Workers), uses D1-backed fixed-window counters.
    ///
    /// Uses an atomic INSERT ... ON CONFLICT DO UPDATE to increment the counter
    /// within the current window, or reset if the window has expired.
    #[cfg(target_arch = "wasm32")]
    pub async fn check(&self, ctx: &dyn Context, key: &str, limit: RateLimit) -> Result<u32, u64> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let window_secs = limit.window.as_secs() as i64;
        let window_start = now - window_secs;

        // Atomic upsert: increment count if within window, reset if expired
        let sql = "INSERT INTO suppers_ai__auth__rate_limits (id, key, count, window_start, created_at, updated_at) \
                   VALUES (?1, ?2, 1, ?3, datetime('now'), datetime('now')) \
                   ON CONFLICT(key) DO UPDATE SET \
                   count = CASE WHEN window_start < ?4 THEN 1 ELSE count + 1 END, \
                   window_start = CASE WHEN window_start < ?4 THEN ?3 ELSE window_start END, \
                   updated_at = datetime('now')";

        let id = super::helpers::sha256_hex(format!("rl:{key}:{now}").as_bytes());
        let _ = db::exec_raw(ctx, sql, &[
            serde_json::json!(id),
            serde_json::json!(key),
            serde_json::json!(now),
            serde_json::json!(window_start),
        ]).await;

        // Read back the current count
        let rows = db::query_raw(
            ctx,
            "SELECT count FROM suppers_ai__auth__rate_limits WHERE key = ?1 AND window_start >= ?2",
            &[serde_json::json!(key), serde_json::json!(window_start)],
        ).await.unwrap_or_default();

        let count = rows.first()
            .and_then(|r| r.data.get("count"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as u32;

        if count > limit.max_requests {
            Err(window_secs as u64)
        } else {
            Ok(limit.max_requests - count)
        }
    }

    /// Build a composite key from user identity and category.
    /// For unauthenticated endpoints (login/signup), use IP as the identity.
    pub fn key(identity: &str, category: &str) -> String {
        format!("{}:{}", identity, category)
    }
}

/// Set rate limit response headers on the message.
pub fn set_rate_limit_headers(msg: &mut wafer_run::types::Message, limit: u32, remaining: u32) {
    msg.set_meta("resp.header.X-RateLimit-Limit", limit.to_string());
    msg.set_meta("resp.header.X-RateLimit-Remaining", remaining.to_string());
}

/// Return a 429 Too Many Requests response.
pub fn rate_limited_response(
    msg: &mut wafer_run::types::Message,
    retry_after: u64,
) -> wafer_run::types::Result_ {
    use super::errors::{error_response, ErrorCode};
    msg.set_meta("resp.header.Retry-After", retry_after.to_string());
    msg.set_meta("resp.header.X-RateLimit-Remaining", "0");
    error_response(
        msg,
        ErrorCode::RateLimitExceeded,
        "Too many requests — try again later",
    )
}

/// Check a per-user/identity rate limit and apply headers or return a 429 response.
pub async fn check_rate_limit(
    limiter: &UserRateLimiter,
    ctx: &dyn wafer_run::context::Context,
    msg: &mut wafer_run::types::Message,
    identity: &str,
    category: &str,
    default: RateLimit,
) -> Option<wafer_run::types::Result_> {
    let limit = match default.resolve(ctx, category).await {
        Some(l) => l,
        None => return None,
    };
    let key = UserRateLimiter::key(identity, category);
    match limiter.check(ctx, &key, limit).await {
        Ok(remaining) => {
            set_rate_limit_headers(msg, limit.max_requests, remaining);
            None
        }
        Err(retry_after) => Some(rate_limited_response(msg, retry_after)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wafer_run::types::{Action, Message, Result_};

    struct TestCtx;

    #[async_trait::async_trait]
    impl Context for TestCtx {
        async fn call_block(&self, _block_name: &str, _msg: &mut Message) -> Result_ {
            Result_ {
                action: Action::Continue,
                response: None,
                error: None,
                message: None,
            }
        }
        fn is_cancelled(&self) -> bool {
            false
        }
        fn config_get(&self, _key: &str) -> Option<&str> {
            None
        }
    }

    #[tokio::test]
    async fn test_rate_limit_allows_within_window() {
        let ctx = TestCtx;
        let limiter = UserRateLimiter::new();
        let limit = RateLimit {
            max_requests: 5,
            window: Duration::from_secs(60),
        };

        // First 5 requests should succeed
        for i in (0..5).rev() {
            let result = limiter.check(&ctx, "user1:test", limit).await;
            assert_eq!(result, Ok(i));
        }
    }

    #[tokio::test]
    async fn test_rate_limit_blocks_excess() {
        let ctx = TestCtx;
        let limiter = UserRateLimiter::new();
        let limit = RateLimit {
            max_requests: 3,
            window: Duration::from_secs(60),
        };

        // Use up the limit
        assert!(limiter.check(&ctx, "user1:test", limit).await.is_ok());
        assert!(limiter.check(&ctx, "user1:test", limit).await.is_ok());
        assert!(limiter.check(&ctx, "user1:test", limit).await.is_ok());

        // 4th request should be denied
        let result = limiter.check(&ctx, "user1:test", limit).await;
        assert!(result.is_err());
        let retry_after = result.unwrap_err();
        assert!(retry_after >= 1);
    }

    #[tokio::test]
    async fn test_rate_limit_separate_keys() {
        let ctx = TestCtx;
        let limiter = UserRateLimiter::new();
        let limit = RateLimit {
            max_requests: 2,
            window: Duration::from_secs(60),
        };

        // Different keys have independent limits
        assert!(limiter.check(&ctx, "user1:auth", limit).await.is_ok());
        assert!(limiter.check(&ctx, "user1:auth", limit).await.is_ok());
        assert!(limiter.check(&ctx, "user1:auth", limit).await.is_err());

        // user2 should still be allowed
        assert!(limiter.check(&ctx, "user2:auth", limit).await.is_ok());
    }

    #[test]
    fn test_rate_limit_key_format() {
        assert_eq!(UserRateLimiter::key("user123", "auth"), "user123:auth");
        assert_eq!(
            UserRateLimiter::key("192.168.1.1", "login"),
            "192.168.1.1:login"
        );
    }

    #[tokio::test]
    async fn test_rate_limit_window_reset() {
        let ctx = TestCtx;
        let limiter = UserRateLimiter::new();
        let limit = RateLimit {
            max_requests: 2,
            window: Duration::from_millis(1),
        };

        // Use up the limit
        assert!(limiter.check(&ctx, "user:test", limit).await.is_ok());
        assert!(limiter.check(&ctx, "user:test", limit).await.is_ok());
        assert!(limiter.check(&ctx, "user:test", limit).await.is_err());

        // Wait for window to expire
        tokio::time::sleep(Duration::from_millis(5)).await;

        // Should be allowed again
        assert!(limiter.check(&ctx, "user:test", limit).await.is_ok());
    }

    #[test]
    fn test_rate_limit_constants() {
        assert_eq!(RateLimit::AUTH.max_requests, 30);
        assert_eq!(RateLimit::AUTH.window, Duration::from_secs(60));
        assert_eq!(RateLimit::REFRESH.max_requests, 30);
        assert_eq!(RateLimit::API_READ.max_requests, 300);
        assert_eq!(RateLimit::API_WRITE.max_requests, 120);
        assert_eq!(RateLimit::UPLOAD.max_requests, 60);
    }

    #[tokio::test]
    async fn test_default_impl() {
        let ctx = TestCtx;
        let limiter = UserRateLimiter::default();
        let limit = RateLimit {
            max_requests: 1,
            window: Duration::from_secs(60),
        };
        assert!(limiter.check(&ctx, "key", limit).await.is_ok());
    }
}
