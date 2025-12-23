package middleware

import (
	"encoding/json"
	"net/http"
	"strings"

	"github.com/suppers-ai/solobase/internal/env"
)

// ReadOnlyMiddleware blocks all write operations when READONLY_MODE is enabled
func ReadOnlyMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Check if read-only mode is enabled
		if env.GetEnv("READONLY_MODE") != "true" {
			next.ServeHTTP(w, r)
			return
		}

		// Allow read-only methods
		if r.Method == http.MethodGet || r.Method == http.MethodOptions || r.Method == http.MethodHead {
			next.ServeHTTP(w, r)
			return
		}

		// Allow specific endpoints that should work even in read-only mode
		// (e.g., login endpoints need POST but don't modify data)
		allowedPaths := []string{
			"/auth/login",
			"/auth/logout",
			"/auth/refresh",
		}

		// Check if the path contains any allowed endpoint
		for _, path := range allowedPaths {
			if strings.Contains(r.URL.Path, path) {
				next.ServeHTTP(w, r)
				return
			}
		}

		// Block all other write operations
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusForbidden)

		response := map[string]interface{}{
			"error":   "This demo is read-only",
			"message": "Write operations are disabled in demo mode. You can explore all features but cannot modify data.",
			"code":    "READONLY_MODE",
		}

		json.NewEncoder(w).Encode(response)
	})
}
