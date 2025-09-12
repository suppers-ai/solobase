package middleware

import (
	"context"
	"net/http"

	"github.com/suppers-ai/solobase/constants"
	"github.com/suppers-ai/solobase/internal/core/services"
)

// Role constants - use from constants package
const (
	RoleUser    = string(constants.RoleUser)
	RoleManager = string(constants.RoleManager)
	RoleAdmin   = string(constants.RoleAdmin)
	RoleDeleted = string(constants.RoleDeleted)
)

// RequireRole creates a middleware that checks if user has one of the required roles
func RequireRole(svc *services.Service, allowedRoles ...string) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			ctx := r.Context()

			// Get user from context (set by auth middleware)
			userID, ok := ctx.Value(constants.ContextKeyUserID).(string)
			if !ok || userID == "" {
				http.Error(w, "Unauthorized", http.StatusUnauthorized)
				return
			}

			// Get user roles from context (set by JWT auth middleware)
			userRoles, ok := ctx.Value(constants.ContextKeyUserRoles).([]string)
			if !ok {
				// No roles in context, assume no roles
				userRoles = []string{}
			}

			// Check if user has deleted role (banned)
			for _, role := range userRoles {
				if role == RoleDeleted {
					http.Error(w, "Access denied - Account banned", http.StatusForbidden)
					return
				}
			}

			// Check if user has required role
			hasRole := false
			for _, userRole := range userRoles {
				for _, allowedRole := range allowedRoles {
					if userRole == allowedRole {
						hasRole = true
						break
					}
				}
				if hasRole {
					break
				}
			}

			if !hasRole {
				http.Error(w, "Access denied - Insufficient permissions", http.StatusForbidden)
				return
			}

			// Add roles to context for downstream handlers if not already there
			if !ok {
				ctx = context.WithValue(ctx, constants.ContextKeyUserRoles, userRoles)
			}
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

// RequireAdmin is a convenience middleware for admin-only routes
func RequireAdmin(svc *services.Service) func(http.Handler) http.Handler {
	return RequireRole(svc, RoleAdmin)
}

// RequireManagerOrAdmin is a convenience middleware for manager and admin routes
func RequireManagerOrAdmin(svc *services.Service) func(http.Handler) http.Handler {
	return RequireRole(svc, RoleManager, RoleAdmin)
}

// CheckReadOnly prevents non-admin users from making modifications
func CheckReadOnly(svc *services.Service) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Allow GET, HEAD, OPTIONS requests for all authorized users
			if r.Method == "GET" || r.Method == "HEAD" || r.Method == "OPTIONS" {
				next.ServeHTTP(w, r)
				return
			}

			// For write operations, require admin role
			ctx := r.Context()
			userRoles, ok := ctx.Value(constants.ContextKeyUserRoles).([]string)
			if !ok {
				// No roles in context, assume no roles
				userRoles = []string{}
			}

			// Check if user has admin role
			isAdmin := false
			for _, role := range userRoles {
				if role == RoleAdmin {
					isAdmin = true
					break
				}
			}

			// Only admins can perform write operations
			if !isAdmin {
				http.Error(w, "Access denied - Read-only access", http.StatusForbidden)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// EnforceRoleHierarchy ensures users can only manage users with lower roles
func EnforceRoleHierarchy(svc *services.Service) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			ctx := r.Context()

			// Get current user's roles
			currentRoles, ok := ctx.Value(constants.ContextKeyUserRoles).([]string)
			if !ok {
				http.Error(w, "Unauthorized", http.StatusUnauthorized)
				return
			}

			// Check if user is admin
			isAdmin := false
			isManager := false
			for _, role := range currentRoles {
				if role == RoleAdmin {
					isAdmin = true
					break
				}
				if role == RoleManager {
					isManager = true
				}
			}

			// Admins can manage everyone
			if isAdmin {
				next.ServeHTTP(w, r)
				return
			}

			// Managers can only manage regular users
			if isManager {
				// TODO: Add logic to check target user's role
				// For now, allow managers to proceed but with limitations
				ctx = context.WithValue(ctx, "role_limit", RoleUser)
				next.ServeHTTP(w, r.WithContext(ctx))
				return
			}

			// Regular users cannot manage other users
			http.Error(w, "Access denied", http.StatusForbidden)
		})
	}
}
