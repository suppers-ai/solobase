package middleware

import (
	"net/http"
	"os"
	"strings"

	"github.com/suppers-ai/solobase/internal/config"
)

// Default CORS settings
var (
	defaultAllowedOrigins = []string{"*"}
	defaultAllowedMethods = []string{"GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH"}
	defaultAllowedHeaders = []string{"Content-Type", "Authorization"}
)

// getEnvSlice gets a comma-separated env var as a slice
func getEnvSlice(key string, defaultValue []string) []string {
	if value := os.Getenv(key); value != "" {
		return strings.Split(value, ",")
	}
	return defaultValue
}

// CORS middleware - if cfg is nil, reads from environment variables
func CORS(cfg *config.Config) func(http.Handler) http.Handler {
	// Get CORS settings from config or environment
	var allowedOrigins, allowedMethods, allowedHeaders []string

	if cfg != nil {
		allowedOrigins = cfg.CORSAllowedOrigins
		allowedMethods = cfg.CORSAllowedMethods
		allowedHeaders = cfg.CORSAllowedHeaders
	} else {
		allowedOrigins = getEnvSlice("CORS_ALLOWED_ORIGINS", defaultAllowedOrigins)
		allowedMethods = getEnvSlice("CORS_ALLOWED_METHODS", defaultAllowedMethods)
		allowedHeaders = getEnvSlice("CORS_ALLOWED_HEADERS", defaultAllowedHeaders)
	}

	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			origin := r.Header.Get("Origin")

			// Check if origin is allowed
			allowed := false
			allowAll := false
			for _, allowedOrigin := range allowedOrigins {
				if allowedOrigin == "*" {
					allowAll = true
					allowed = true
					break
				}
				if allowedOrigin == origin {
					allowed = true
					break
				}
			}

			// When credentials are used, we must echo the specific origin, not "*"
			// This is required by CORS spec when Access-Control-Allow-Credentials is true
			if origin != "" {
				if allowed {
					w.Header().Set("Access-Control-Allow-Origin", origin)
				}
				// If not allowed, don't set header - browser will block the request
			} else if allowAll {
				// No origin header (non-browser request like curl) - allow with *
				w.Header().Set("Access-Control-Allow-Origin", "*")
			}

			w.Header().Set("Access-Control-Allow-Methods", strings.Join(allowedMethods, ", "))
			w.Header().Set("Access-Control-Allow-Headers", strings.Join(allowedHeaders, ", "))
			w.Header().Set("Access-Control-Allow-Credentials", "true")
			w.Header().Set("Access-Control-Max-Age", "3600")

			// Handle preflight requests
			if r.Method == "OPTIONS" {
				w.WriteHeader(http.StatusOK)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}
