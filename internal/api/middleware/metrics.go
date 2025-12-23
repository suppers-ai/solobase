package middleware

import "net/http"

// MetricsMiddleware is a pass-through middleware (metrics collected via request logging)
func MetricsMiddleware(next http.Handler) http.Handler {
	return next
}
