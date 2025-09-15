package middleware

import (
	"net/http"
	"os"
	"strconv"
	"sync"
	"time"
)

// SecurityHeadersMiddleware adds security headers to all responses
func SecurityHeadersMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Content Security Policy - prevent XSS attacks
		w.Header().Set("Content-Security-Policy", "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'")

		// Prevent clickjacking
		w.Header().Set("X-Frame-Options", "DENY")

		// Prevent MIME type sniffing
		w.Header().Set("X-Content-Type-Options", "nosniff")

		// Enable XSS protection in older browsers
		w.Header().Set("X-XSS-Protection", "1; mode=block")

		// Only send referrer for same origin
		w.Header().Set("Referrer-Policy", "same-origin")

		// Force HTTPS in production
		if os.Getenv("ENVIRONMENT") == "production" {
			w.Header().Set("Strict-Transport-Security", "max-age=31536000; includeSubDomains")
		}

		next.ServeHTTP(w, r)
	})
}

// RateLimitMiddleware provides additional rate limiting for demo mode
type RateLimitMiddleware struct {
	requests map[string]*userRateLimit
	mu       sync.RWMutex
}

type userRateLimit struct {
	count      int
	resetTime  time.Time
	maxPerMin  int
}

// NewRateLimitMiddleware creates a new rate limiter
func NewRateLimitMiddleware() *RateLimitMiddleware {
	rl := &RateLimitMiddleware{
		requests: make(map[string]*userRateLimit),
	}

	// Clean up old entries every minute
	go func() {
		ticker := time.NewTicker(1 * time.Minute)
		defer ticker.Stop()
		for range ticker.C {
			rl.cleanup()
		}
	}()

	return rl
}

// Middleware returns the rate limiting middleware function
func (rl *RateLimitMiddleware) Middleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Only apply aggressive rate limiting in readonly mode
		if os.Getenv("READONLY_MODE") != "true" {
			next.ServeHTTP(w, r)
			return
		}

		clientIP := r.RemoteAddr
		if xff := r.Header.Get("X-Forwarded-For"); xff != "" {
			clientIP = xff
		}

		// Check rate limit
		limit := 60 // 60 requests per minute default

		// More restrictive for expensive endpoints
		if r.URL.Path == "/api/admin/logs" || r.URL.Path == "/api/admin/database/query" {
			limit = 10
		}

		allowed, remaining := rl.checkLimit(clientIP, limit)

		// Set rate limit headers
		w.Header().Set("X-RateLimit-Limit", strconv.Itoa(limit))
		w.Header().Set("X-RateLimit-Remaining", strconv.Itoa(remaining))

		if !allowed {
			w.Header().Set("Content-Type", "application/json")
			w.Header().Set("Retry-After", "60")
			w.WriteHeader(http.StatusTooManyRequests)
			w.Write([]byte(`{"error":"Rate limit exceeded","message":"Please wait before making more requests"}`))
			return
		}

		next.ServeHTTP(w, r)
	})
}

func (rl *RateLimitMiddleware) checkLimit(clientIP string, maxPerMin int) (bool, int) {
	rl.mu.Lock()
	defer rl.mu.Unlock()

	now := time.Now()

	// Get or create rate limit entry
	limit, exists := rl.requests[clientIP]
	if !exists || now.After(limit.resetTime) {
		rl.requests[clientIP] = &userRateLimit{
			count:     1,
			resetTime: now.Add(1 * time.Minute),
			maxPerMin: maxPerMin,
		}
		return true, maxPerMin - 1
	}

	// Check if limit exceeded
	if limit.count >= maxPerMin {
		return false, 0
	}

	// Increment counter
	limit.count++
	remaining := maxPerMin - limit.count
	return true, remaining
}

func (rl *RateLimitMiddleware) cleanup() {
	rl.mu.Lock()
	defer rl.mu.Unlock()

	now := time.Now()
	for ip, limit := range rl.requests {
		if now.After(limit.resetTime) {
			delete(rl.requests, ip)
		}
	}
}