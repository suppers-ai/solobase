package middleware

import (
	"net/http"
	"sync"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

type visitor struct {
	lastSeen apptime.Time
	count    int
}

var (
	visitors = make(map[string]*visitor)
	mu       sync.RWMutex
)

// RateLimit middleware
func RateLimit(requestsPerMinute int) func(http.Handler) http.Handler {
	// Cleanup old visitors periodically
	go func() {
		for {
			apptime.Sleep(apptime.Minute)
			mu.Lock()
			for ip, v := range visitors {
				if apptime.Since(v.lastSeen) > apptime.Minute {
					delete(visitors, ip)
				}
			}
			mu.Unlock()
		}
	}()

	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Get client IP
			ip := r.RemoteAddr
			if xff := r.Header.Get("X-Forwarded-For"); xff != "" {
				ip = xff
			}

			mu.Lock()
			v, exists := visitors[ip]
			if !exists {
				visitors[ip] = &visitor{lastSeen: apptime.NowTime(), count: 1}
				mu.Unlock()
				next.ServeHTTP(w, r)
				return
			}

			// Reset count if more than a minute has passed
			if apptime.Since(v.lastSeen) > apptime.Minute {
				v.count = 1
				v.lastSeen = apptime.NowTime()
				mu.Unlock()
				next.ServeHTTP(w, r)
				return
			}

			// Check rate limit
			if v.count >= requestsPerMinute {
				mu.Unlock()
				http.Error(w, "Rate limit exceeded", http.StatusTooManyRequests)
				return
			}

			v.count++
			v.lastSeen = apptime.NowTime()
			mu.Unlock()

			next.ServeHTTP(w, r)
		})
	}
}
