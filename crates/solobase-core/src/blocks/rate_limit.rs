use std::time::Duration;
use wafer_core::clients::config;
use wafer_run::context::Context;
use wafer_run::OutputStream;

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
        // std::time::SystemTime::now() panics on wasm32-unknown-unknown
        // (no system clock). Use js_sys::Date::now() which returns ms since epoch.
        let now = (js_sys::Date::now() / 1000.0) as i64;
        let window_secs = limit.window.as_secs() as i64;
        let window_start = now - window_secs;

        // Atomic upsert: increment count if within window, reset if expired.
        // Uses CASE WHEN for conditional reset -- too complex for the upsert builder.
        use crate::blocks::auth::RATE_LIMITS_COLLECTION as RATE_LIMITS;
        let sql = format!(
            "INSERT INTO {RATE_LIMITS} (id, key, count, window_start, created_at, updated_at) \
             VALUES (?1, ?2, 1, ?3, datetime('now'), datetime('now')) \
             ON CONFLICT(key) DO UPDATE SET \
             count = CASE WHEN window_start < ?4 THEN 1 ELSE count + 1 END, \
             window_start = CASE WHEN window_start < ?4 THEN ?3 ELSE window_start END, \
             updated_at = datetime('now')"
        );

        let id = super::helpers::sha256_hex(format!("rl:{key}:{now}").as_bytes());
        let _ = db::exec_raw(ctx, &sql, &[
            serde_json::json!(id),
            serde_json::json!(key),
            serde_json::json!(now),
            serde_json::json!(window_start),
        ]).await;

        // Read back the current count
        use wafer_sql_utils::{query, value::sea_values_to_json, Backend};
        use wafer_core::interfaces::database::service::{Filter, FilterOp, ListOptions};

        let (sql, vals) = query::build_select_columns(
            RATE_LIMITS,
            &["count"],
            &ListOptions {
                filters: vec![
                    Filter { field: "key".into(), operator: FilterOp::Equal, value: serde_json::json!(key) },
                    Filter { field: "window_start".into(), operator: FilterOp::GreaterEqual, value: serde_json::json!(window_start) },
                ],
                ..Default::default()
            },
            None,
            Backend::Sqlite,
        );
        let args = sea_values_to_json(vals);
        let rows = db::query_raw(ctx, &sql, &args).await.unwrap_or_default();

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

/// Rate limit headers to attach to a successful response.
#[derive(Debug, Clone, Copy)]
pub struct RateLimitHeaders {
    pub limit: u32,
    pub remaining: u32,
}

impl RateLimitHeaders {
    /// Apply these headers to a `ResponseBuilder`.
    pub fn apply(
        self,
        builder: crate::blocks::helpers::ResponseBuilder,
    ) -> crate::blocks::helpers::ResponseBuilder {
        builder
            .set_header("X-RateLimit-Limit", &self.limit.to_string())
            .set_header("X-RateLimit-Remaining", &self.remaining.to_string())
    }
}

/// Return a 429 Too Many Requests response with a `Retry-After` header.
pub fn rate_limited_response(retry_after: u64) -> OutputStream {
    use super::errors::ErrorCode;
    let wafer_code = super::errors::solobase_error_code_to_wafer(ErrorCode::RateLimitExceeded);
    let full_message = format!(
        "[{}] Too many requests — try again later",
        ErrorCode::RateLimitExceeded.as_str()
    );
    OutputStream::error(wafer_run::WaferError {
        code: wafer_code,
        message: full_message,
        meta: vec![wafer_run::types::MetaEntry {
            key: "resp.header.Retry-After".to_string(),
            value: retry_after.to_string(),
        }],
    })
}

/// Outcome of a rate-limit check.
pub enum RateLimitOutcome {
    /// Allowed — caller should attach these headers to the success response.
    Allowed(RateLimitHeaders),
    /// Disabled — no rate limiting applied for this category.
    Disabled,
    /// Rate-limited — caller should return this `OutputStream` immediately.
    Limited(OutputStream),
}

/// Check a per-user/identity rate limit and return an `OutputStream` if blocked,
/// or rate-limit headers to attach to the success response.
pub async fn check_rate_limit(
    limiter: &UserRateLimiter,
    ctx: &dyn wafer_run::context::Context,
    identity: &str,
    category: &str,
    default: RateLimit,
) -> RateLimitOutcome {
    let limit = match default.resolve(ctx, category).await {
        Some(l) => l,
        None => return RateLimitOutcome::Disabled,
    };
    let key = UserRateLimiter::key(identity, category);
    match limiter.check(ctx, &key, limit).await {
        Ok(remaining) => RateLimitOutcome::Allowed(RateLimitHeaders {
            limit: limit.max_requests,
            remaining,
        }),
        Err(retry_after) => RateLimitOutcome::Limited(rate_limited_response(retry_after)),
    }
}

/// Convenience wrapper: check per-user rate limit using the request's user_id.
///
/// Automatically determines read vs write category from the message action.
/// Returns `RateLimitOutcome::Disabled` for unauthenticated requests (empty user_id).
pub async fn check_user_rate_limit(
    limiter: &UserRateLimiter,
    ctx: &dyn wafer_run::context::Context,
    msg: &wafer_run::types::Message,
) -> RateLimitOutcome {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return RateLimitOutcome::Disabled;
    }
    let action = msg.action().to_string();
    let (default, category) = if action == "retrieve" {
        (RateLimit::API_READ, "api_read")
    } else {
        (RateLimit::API_WRITE, "api_write")
    };
    check_rate_limit(limiter, ctx, &user_id, category, default).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use wafer_run::context::Context;
    use wafer_run::types::Message;
    use wafer_run::{InputStream, OutputStream};

    struct TestCtx;

    #[async_trait::async_trait]
    impl Context for TestCtx {
        async fn call_block(
            &self,
            _block_name: &str,
            _msg: Message,
            _input: InputStream,
        ) -> OutputStream {
            OutputStream::respond(vec![])
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
