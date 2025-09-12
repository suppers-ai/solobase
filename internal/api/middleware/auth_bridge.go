package middleware

import (
	"context"
	"net/http"

	"github.com/volatiletech/authboss/v3"
)

// AuthBridge extracts user information from authboss and adds it to context for RBAC middleware
func AuthBridge(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Get user ID from authboss context
		// The authboss.SessionKey is stored in the request context after LoadClientStateMiddleware
		if userID := r.Context().Value(authboss.SessionKey); userID != nil {
			// Add user ID to context for RBAC middleware
			ctx := r.Context()
			ctx = context.WithValue(ctx, "user_id", userID)
			next.ServeHTTP(w, r.WithContext(ctx))
			return
		}

		// No user in context, pass through unchanged
		next.ServeHTTP(w, r)
	})
}
