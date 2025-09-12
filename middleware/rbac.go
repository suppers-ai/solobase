package middleware

import (
	"context"
	"net/http"

	"github.com/suppers-ai/solobase/constants"
	"github.com/suppers-ai/solobase/services"
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

			// Get user role from database using GORM
			var user struct {
				Role string
			}
			// Use simple table name
			tableName := "users"
			err := svc.DB().Table(tableName).
				Select("COALESCE(role, 'user') as role").
				Where("id = ?", userID).
				First(&user).Error

			role := user.Role

			if err != nil {
				svc.Logger().Error(ctx, "Failed to get user role")
				http.Error(w, "Internal server error", http.StatusInternalServerError)
				return
			}

			// Check if user role is deleted (banned)
			if role == RoleDeleted {
				http.Error(w, "Access denied - Account banned", http.StatusForbidden)
				return
			}

			// Check if user has required role
			hasRole := false
			for _, allowedRole := range allowedRoles {
				if role == allowedRole {
					hasRole = true
					break
				}
			}

			if !hasRole {
				http.Error(w, "Access denied - Insufficient permissions", http.StatusForbidden)
				return
			}

			// Add role to context for downstream handlers
			ctx = context.WithValue(ctx, constants.ContextKeyUserRole, role)
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
			role, ok := ctx.Value(constants.ContextKeyUserRole).(string)
			if !ok {
				// If no role in context, check it
				userID, ok := ctx.Value(constants.ContextKeyUserID).(string)
				if !ok || userID == "" {
					http.Error(w, "Unauthorized", http.StatusUnauthorized)
					return
				}

				var user struct {
					Role string
				}
				// Use simple table name
				tableName := "users"
				err := svc.DB().Table(tableName).
					Select("COALESCE(role, 'user') as role").
					Where("id = ?", userID).
					First(&user).Error
				role = user.Role

				if err != nil {
					svc.Logger().Error(ctx, "Failed to get user role")
					http.Error(w, "Internal server error", http.StatusInternalServerError)
					return
				}
			}

			// Only admins can perform write operations
			if role != RoleAdmin {
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

			// Get current user's role
			currentRole, ok := ctx.Value("user_role").(string)
			if !ok {
				http.Error(w, "Unauthorized", http.StatusUnauthorized)
				return
			}

			// Admins can manage everyone
			if currentRole == RoleAdmin {
				next.ServeHTTP(w, r)
				return
			}

			// Managers can only manage regular users
			if currentRole == RoleManager {
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
