package middleware

import (
	"context"
	"net/http"
	"strings"

	"github.com/suppers-ai/logger"
	"github.com/suppers-ai/solobase/constants"
	"github.com/suppers-ai/solobase/services"
)

// SessionAuth checks if user is logged in via session
func SessionAuth(svc *services.Service) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			ctx := r.Context()

			// Get session cookie
			cookie, err := r.Cookie("session_id")
			if err != nil || cookie.Value == "" {
				svc.Logger().Debug(ctx, "No session cookie",
					logger.String("path", r.URL.Path))
				handleUnauthorized(w, r)
				return
			}

			// Verify session in database
			session, err := svc.Sessions().GetSession(ctx, cookie.Value)
			if err != nil {
				svc.Logger().Debug(ctx, "Invalid or expired session",
					logger.Err(err),
					logger.String("path", r.URL.Path))
				handleUnauthorized(w, r)
				return
			}

			// Get user info
			user, err := svc.Users().GetUserByID(ctx, session.UserID)
			if err != nil {
				svc.Logger().Debug(ctx, "Failed to get user",
					logger.Err(err),
					logger.String("user_id", session.UserID.String()))
				handleUnauthorized(w, r)
				return
			}

			// Add user info to context
			ctx = context.WithValue(ctx, constants.ContextKeyUserID, user.ID.String())
			ctx = context.WithValue(ctx, constants.ContextKeyUserEmail, user.Email)

			svc.Logger().Debug(ctx, "Session auth successful",
				logger.String("user_id", user.ID.String()),
				logger.String("email", user.Email),
				logger.String("path", r.URL.Path))

			// Continue with the request
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

// handleUnauthorized handles unauthorized requests by either redirecting to login or returning 401
func handleUnauthorized(w http.ResponseWriter, r *http.Request) {
	// Check if this is an API request
	if strings.HasPrefix(r.URL.Path, "/api/") {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	// Check Accept header to determine if this is an AJAX request
	accept := r.Header.Get("Accept")
	if strings.Contains(accept, "application/json") {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusUnauthorized)
		w.Write([]byte(`{"error":"Unauthorized"}`))
		return
	}

	// For HTML requests, redirect to login page
	http.Redirect(w, r, "/auth/login", http.StatusSeeOther)
}
