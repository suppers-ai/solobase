package middleware

import (
	"context"
	"net/http"

	auth "github.com/suppers-ai/auth"
)

// GetUserFromContext retrieves the authenticated user from the request context
func GetUserFromContext(ctx context.Context) (*auth.User, bool) {
	user, ok := ctx.Value("user").(*auth.User)
	return user, ok
}

// GetUserIDFromContext retrieves the user ID from the request context
func GetUserIDFromContext(ctx context.Context) (string, bool) {
	userID, ok := ctx.Value("userID").(string)
	return userID, ok
}

// GetUserRolesFromContext retrieves the user roles from the request context
func GetUserRolesFromContext(ctx context.Context) ([]string, bool) {
	// Roles are now stored in context from JWT claims
	roles, ok := ctx.Value("user_roles").([]string)
	if !ok {
		return nil, false
	}
	return roles, true
}

// IsAdminFromContext checks if the user in context has admin role
func IsAdminFromContext(ctx context.Context) bool {
	roles, ok := GetUserRolesFromContext(ctx)
	if !ok {
		return false
	}
	for _, role := range roles {
		if role == "admin" {
			return true
		}
	}
	return false
}

// GetUserFromRequest is a convenience function to get user from HTTP request
func GetUserFromRequest(r *http.Request) (*auth.User, bool) {
	return GetUserFromContext(r.Context())
}

// GetUserIDFromRequest is a convenience function to get user ID from HTTP request
func GetUserIDFromRequest(r *http.Request) (string, bool) {
	return GetUserIDFromContext(r.Context())
}

// IsAdminRequest checks if the request is from an admin user
func IsAdminRequest(r *http.Request) bool {
	return IsAdminFromContext(r.Context())
}