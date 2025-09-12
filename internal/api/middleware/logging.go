package middleware

import (
	"net/http"
	"time"

	"github.com/suppers-ai/logger"
)

// LoggingMiddleware logs all HTTP requests
func LoggingMiddleware(log logger.Logger) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			start := time.Now()

			// Create a response writer wrapper to capture status code
			wrapped := &responseWriter{
				ResponseWriter: w,
				statusCode:     http.StatusOK,
			}

			// Process request
			next.ServeHTTP(wrapped, r)

			// Calculate duration
			duration := time.Since(start)

			// Log the request based on status code
			if wrapped.statusCode >= 500 {
				log.Error(r.Context(), "HTTP Request",
					logger.String("method", r.Method),
					logger.String("path", r.URL.Path),
					logger.Int("status", wrapped.statusCode),
					logger.Duration("duration", duration),
					logger.String("user_ip", r.RemoteAddr),
					logger.String("user_agent", r.UserAgent()),
				)
			} else if wrapped.statusCode >= 400 {
				log.Warn(r.Context(), "HTTP Request",
					logger.String("method", r.Method),
					logger.String("path", r.URL.Path),
					logger.Int("status", wrapped.statusCode),
					logger.Duration("duration", duration),
					logger.String("user_ip", r.RemoteAddr),
					logger.String("user_agent", r.UserAgent()),
				)
			} else {
				log.Info(r.Context(), "HTTP Request",
					logger.String("method", r.Method),
					logger.String("path", r.URL.Path),
					logger.Int("status", wrapped.statusCode),
					logger.Duration("duration", duration),
					logger.String("user_ip", r.RemoteAddr),
					logger.String("user_agent", r.UserAgent()),
				)
			}
		})
	}
}

type responseWriter struct {
	http.ResponseWriter
	statusCode int
	written    bool
}

func (rw *responseWriter) WriteHeader(code int) {
	if !rw.written {
		rw.statusCode = code
		rw.ResponseWriter.WriteHeader(code)
		rw.written = true
	}
}

func (rw *responseWriter) Write(b []byte) (int, error) {
	if !rw.written {
		rw.WriteHeader(http.StatusOK)
	}
	return rw.ResponseWriter.Write(b)
}
