package logger

import (
	"bytes"
	"context"
	"encoding/json"
	"io"
	"net/http"
	"strings"
	"time"

	"github.com/google/uuid"
)

// HTTPMiddleware creates middleware for HTTP request logging
func HTTPMiddleware(logger Logger, config *MiddlewareConfig) func(http.Handler) http.Handler {
	if config == nil {
		config = DefaultMiddlewareConfig()
	}

	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Skip paths
			for _, skip := range config.SkipPaths {
				if r.URL.Path == skip || strings.HasPrefix(r.URL.Path, skip) {
					next.ServeHTTP(w, r)
					return
				}
			}

			// Generate trace ID
			traceID := r.Header.Get("X-Trace-ID")
			if traceID == "" {
				traceID = uuid.New().String()
			}

			// Get user ID from context or header
			userID := r.Header.Get("X-User-ID")
			if userID == "" {
				if user := r.Context().Value("user"); user != nil {
					if u, ok := user.(map[string]interface{}); ok {
						if id, ok := u["id"].(string); ok {
							userID = id
						}
					}
				}
			}

			// Add to context
			ctx := context.WithValue(r.Context(), "trace_id", traceID)
			ctx = context.WithValue(ctx, "user_id", userID)
			r = r.WithContext(ctx)

			// Capture request body if enabled
			var requestBody string
			if config.LogRequestBody && r.Body != nil {
				bodyBytes, err := io.ReadAll(r.Body)
				if err == nil {
					requestBody = string(bodyBytes)
					if config.MaxBodySize > 0 && len(requestBody) > config.MaxBodySize {
						requestBody = requestBody[:config.MaxBodySize] + "...[truncated]"
					}
					// Restore body for handler
					r.Body = io.NopCloser(bytes.NewBuffer(bodyBytes))
				}
			}

			// Capture headers if enabled
			var headers string
			if config.LogHeaders {
				headerMap := make(map[string]string)
				for k, v := range r.Header {
					// Skip sensitive headers
					if isSensitiveHeader(k) {
						headerMap[k] = "[REDACTED]"
					} else {
						headerMap[k] = strings.Join(v, ", ")
					}
				}
				if data, err := json.Marshal(headerMap); err == nil {
					headers = string(data)
				}
			}

			// Create response writer wrapper
			rw := &responseWriter{
				ResponseWriter: w,
				statusCode:     http.StatusOK,
				body:           &bytes.Buffer{},
			}

			// Start timer
			start := time.Now()

			// Serve request
			next.ServeHTTP(rw, r)

			// Calculate execution time
			duration := time.Since(start)

			// Create request log
			reqLog := &RequestLog{
				Method:     r.Method,
				Path:       r.URL.Path,
				Query:      r.URL.RawQuery,
				StatusCode: rw.statusCode,
				ExecTimeMs: duration.Milliseconds(),
				UserIP:     getClientIP(r),
				UserAgent:  r.UserAgent(),
				CreatedAt:  start,
			}

			if traceID != "" {
				reqLog.TraceID = &traceID
			}

			if userID != "" {
				reqLog.UserID = &userID
			}

			if requestBody != "" {
				reqLog.RequestBody = &requestBody
			}

			if headers != "" {
				reqLog.Headers = &headers
			}

			// Capture response body if enabled and status indicates error
			if config.LogResponseBody && rw.statusCode >= 400 {
				responseBody := rw.body.String()
				if config.MaxBodySize > 0 && len(responseBody) > config.MaxBodySize {
					responseBody = responseBody[:config.MaxBodySize] + "...[truncated]"
				}
				reqLog.ResponseBody = &responseBody
			}

			// Add error if status code indicates error
			if rw.statusCode >= 400 {
				errMsg := http.StatusText(rw.statusCode)
				reqLog.Error = &errMsg
			}

			// Log request
			if err := logger.LogRequest(ctx, reqLog); err != nil {
				// Log error but don't fail the request
				logger.Error(ctx, "Failed to log HTTP request",
					Err(err),
					String("method", r.Method),
					String("path", r.URL.Path))
			}
		})
	}
}

// MiddlewareConfig configures the HTTP middleware
type MiddlewareConfig struct {
	SkipPaths       []string
	LogHeaders      bool
	LogRequestBody  bool
	LogResponseBody bool
	MaxBodySize     int
}

// DefaultMiddlewareConfig returns default middleware configuration
func DefaultMiddlewareConfig() *MiddlewareConfig {
	return &MiddlewareConfig{
		SkipPaths: []string{
			"/health",
			"/metrics",
			"/favicon.ico",
			"/static/",
			"/public/",
		},
		LogHeaders:      true,
		LogRequestBody:  true,
		LogResponseBody: true,
		MaxBodySize:     4096,
	}
}

// responseWriter wraps http.ResponseWriter to capture status code and body
type responseWriter struct {
	http.ResponseWriter
	statusCode int
	body       *bytes.Buffer
	written    bool
}

func (rw *responseWriter) WriteHeader(code int) {
	if !rw.written {
		rw.statusCode = code
		rw.written = true
		rw.ResponseWriter.WriteHeader(code)
	}
}

func (rw *responseWriter) Write(data []byte) (int, error) {
	if !rw.written {
		rw.WriteHeader(http.StatusOK)
	}
	// Capture body for error responses
	if rw.statusCode >= 400 && rw.body.Len() < 4096 {
		rw.body.Write(data)
	}
	return rw.ResponseWriter.Write(data)
}

// getClientIP extracts the client IP from the request
func getClientIP(r *http.Request) string {
	// Check X-Real-IP header
	if ip := r.Header.Get("X-Real-IP"); ip != "" {
		return ip
	}

	// Check X-Forwarded-For header
	if forwarded := r.Header.Get("X-Forwarded-For"); forwarded != "" {
		parts := strings.Split(forwarded, ",")
		return strings.TrimSpace(parts[0])
	}

	// Fall back to RemoteAddr
	ip := r.RemoteAddr
	if colonIndex := strings.LastIndex(ip, ":"); colonIndex != -1 {
		ip = ip[:colonIndex]
	}

	return ip
}

// isSensitiveHeader checks if a header contains sensitive information
func isSensitiveHeader(name string) bool {
	sensitive := []string{
		"Authorization",
		"Cookie",
		"Set-Cookie",
		"X-Auth-Token",
		"X-API-Key",
		"X-Session-ID",
	}

	nameLower := strings.ToLower(name)
	for _, s := range sensitive {
		if strings.ToLower(s) == nameLower {
			return true
		}
	}

	return false
}

// GinMiddleware creates middleware for Gin framework
func GinMiddleware(logger Logger, config *MiddlewareConfig) func(c interface{}) {
	if config == nil {
		config = DefaultMiddlewareConfig()
	}

	// This is a placeholder - actual Gin implementation would use gin.Context
	return func(c interface{}) {
		// Implementation would go here
		// This requires importing gin which we're avoiding for now
	}
}

// EchoMiddleware creates middleware for Echo framework
func EchoMiddleware(logger Logger, config *MiddlewareConfig) func(next interface{}) interface{} {
	if config == nil {
		config = DefaultMiddlewareConfig()
	}

	// This is a placeholder - actual Echo implementation would use echo.Context
	return func(next interface{}) interface{} {
		// Implementation would go here
		// This requires importing echo which we're avoiding for now
		return nil
	}
}

// FiberMiddleware creates middleware for Fiber framework
func FiberMiddleware(logger Logger, config *MiddlewareConfig) func(c interface{}) error {
	if config == nil {
		config = DefaultMiddlewareConfig()
	}

	// This is a placeholder - actual Fiber implementation would use fiber.Ctx
	return func(c interface{}) error {
		// Implementation would go here
		// This requires importing fiber which we're avoiding for now
		return nil
	}
}
