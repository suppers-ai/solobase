/**
 * Rate limiting for the Cloudflare Worker using KV store.
 *
 * Uses sliding-window counters stored in KV with TTL expiration.
 * KV has eventual consistency (~60s), so limits may be slightly exceeded
 * under extreme concurrency — but this is acceptable for preventing bulk
 * abuse. For stricter limits, use Cloudflare Rate Limiting rules.
 *
 * Rate limit buckets:
 *   AUTH:     30 req / 60s per IP  — login, signup, forgot-password
 *   REFRESH:  30 req / 60s per IP  — token refresh
 *   API:     300 req / 60s per IP  — general API (authenticated)
 */

import type { Env } from './types';

interface RateLimitEntry {
  /** Request count in the current window. */
  count: number;
  /** Window start timestamp (epoch seconds). */
  windowStart: number;
}

interface RateLimitConfig {
  /** Maximum requests allowed in the window. */
  maxRequests: number;
  /** Window duration in seconds. */
  windowSeconds: number;
}

const LIMITS: Record<string, RateLimitConfig> = {
  AUTH:    { maxRequests: 30,  windowSeconds: 60 },
  REFRESH: { maxRequests: 30,  windowSeconds: 60 },
  API:     { maxRequests: 300, windowSeconds: 60 },
};

/**
 * Result of a rate limit check.
 */
export interface RateLimitResult {
  /** Whether the request is allowed. */
  allowed: boolean;
  /** Remaining requests in the current window. */
  remaining: number;
  /** Maximum requests allowed. */
  limit: number;
  /** Seconds until the window resets (for Retry-After header). */
  retryAfter: number;
}

/**
 * Classify a request path into a rate limit bucket.
 * Returns null for paths that don't need rate limiting.
 */
export function classifyRequest(path: string): string | null {
  if (path === '/auth/login' || path === '/auth/signup' ||
      path === '/auth/forgot-password' || path === '/auth/reset-password' ||
      path === '/auth/verify-email') {
    return 'AUTH';
  }
  if (path === '/auth/refresh') {
    return 'REFRESH';
  }
  // General API rate limit for all other paths
  return 'API';
}

/**
 * Check and increment the rate limit counter for a given key and bucket.
 *
 * @param kv      KV namespace for storing rate limit counters
 * @param ip      Client IP address
 * @param bucket  Rate limit bucket name (AUTH, REFRESH, API)
 * @returns       Rate limit result
 */
export async function checkRateLimit(
  kv: KVNamespace,
  ip: string,
  bucket: string,
): Promise<RateLimitResult> {
  const config = LIMITS[bucket];
  if (!config) {
    return { allowed: true, remaining: 999, limit: 999, retryAfter: 0 };
  }

  const kvKey = `rl:${bucket}:${ip}`;
  const now = Math.floor(Date.now() / 1000);

  let entry: RateLimitEntry | null = null;
  try {
    const raw = await kv.get(kvKey, 'json');
    if (raw) entry = raw as RateLimitEntry;
  } catch {
    // KV read failure — allow the request (fail open)
    return { allowed: true, remaining: config.maxRequests, limit: config.maxRequests, retryAfter: 0 };
  }

  // If the window has expired, start a new one
  if (!entry || (now - entry.windowStart) >= config.windowSeconds) {
    entry = { count: 1, windowStart: now };
    // Write with TTL so KV auto-cleans expired entries
    await kv.put(kvKey, JSON.stringify(entry), { expirationTtl: config.windowSeconds * 2 }).catch(() => {});
    return {
      allowed: true,
      remaining: config.maxRequests - 1,
      limit: config.maxRequests,
      retryAfter: 0,
    };
  }

  // Within the window — increment
  entry.count += 1;

  const remaining = Math.max(0, config.maxRequests - entry.count);
  const retryAfter = config.windowSeconds - (now - entry.windowStart);

  if (entry.count > config.maxRequests) {
    // Over limit — still write the incremented count to track continued abuse
    await kv.put(kvKey, JSON.stringify(entry), { expirationTtl: config.windowSeconds * 2 }).catch(() => {});
    return {
      allowed: false,
      remaining: 0,
      limit: config.maxRequests,
      retryAfter,
    };
  }

  // Under limit — write updated count
  await kv.put(kvKey, JSON.stringify(entry), { expirationTtl: config.windowSeconds * 2 }).catch(() => {});
  return {
    allowed: true,
    remaining,
    limit: config.maxRequests,
    retryAfter: 0,
  };
}

/**
 * Build rate limit response headers.
 */
export function rateLimitHeaders(result: RateLimitResult): Record<string, string> {
  const headers: Record<string, string> = {
    'X-RateLimit-Limit': String(result.limit),
    'X-RateLimit-Remaining': String(result.remaining),
  };
  if (!result.allowed) {
    headers['Retry-After'] = String(result.retryAfter);
  }
  return headers;
}
