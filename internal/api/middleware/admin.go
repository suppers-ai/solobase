package middleware

import (
	"fmt"
	"net/http"

	"github.com/suppers-ai/solobase/internal/iam"
	"github.com/suppers-ai/solobase/utils"
)

// AdminMiddleware ensures the user has admin role
func AdminMiddleware(iamService *iam.Service) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Check if IAM service is available
			if iamService == nil {
				// In WASM mode, IAM might not be available - fall back to token roles
				userRoles := r.Context().Value("user_roles")
				if userRoles == nil {
					utils.JSONError(w, http.StatusForbidden, "Admin access required")
					return
				}
				roles, ok := userRoles.([]string)
				if !ok {
					utils.JSONError(w, http.StatusForbidden, "Admin access required")
					return
				}
				// Check for admin role from token
				hasAdmin := false
				for _, role := range roles {
					if role == "admin" || role == "admin_viewer" {
						hasAdmin = true
						break
					}
				}
				if !hasAdmin {
					utils.JSONError(w, http.StatusForbidden, "Admin access required")
					return
				}
				next.ServeHTTP(w, r)
				return
			}

			// Get user ID from context (set by AuthMiddleware)
			userID := r.Context().Value("userID")
			if userID == nil {
				utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
				return
			}

			userIDStr, ok := userID.(string)
			if !ok {
				utils.JSONError(w, http.StatusUnauthorized, "Invalid user ID")
				return
			}

			// Check if user has admin role
			fmt.Printf("AdminMiddleware: Checking roles for user ID: %s\n", userIDStr)
			roles, err := iamService.GetUserRoles(r.Context(), userIDStr)
			if err != nil {
				fmt.Printf("AdminMiddleware: IAM GetUserRoles failed for user %s: %v, falling back to token roles\n", userIDStr, err)
				// Fall back to token roles when IAM fails (e.g., in WASM mode with unimplemented repo)
				userRoles := r.Context().Value("user_roles")
				if userRoles == nil {
					utils.JSONError(w, http.StatusForbidden, "Admin access required")
					return
				}
				tokenRoles, ok := userRoles.([]string)
				if !ok {
					utils.JSONError(w, http.StatusForbidden, "Admin access required")
					return
				}
				// Check for admin role from token
				hasAdmin := false
				for _, role := range tokenRoles {
					if role == "admin" || role == "admin_viewer" {
						hasAdmin = true
						break
					}
				}
				if !hasAdmin {
					utils.JSONError(w, http.StatusForbidden, "Admin access required")
					return
				}
				fmt.Printf("AdminMiddleware: User %s has admin access via token roles\n", userIDStr)
				next.ServeHTTP(w, r)
				return
			}
			fmt.Printf("AdminMiddleware: User %s has roles: %v\n", userIDStr, roles)

			// Check for admin or admin_viewer role
			hasAdminRole := false
			for _, roleName := range roles {
				if roleName == "admin" || roleName == "admin_viewer" {
					hasAdminRole = true
					break
				}
			}

			if !hasAdminRole {
				utils.JSONError(w, http.StatusForbidden, "Admin access required")
				return
			}

			// For write operations, ensure it's not admin_viewer
			if r.Method != "GET" && r.Method != "HEAD" && r.Method != "OPTIONS" {
				isFullAdmin := false
				for _, roleName := range roles {
					if roleName == "admin" {
						isFullAdmin = true
						break
					}
				}

				if !isFullAdmin {
					utils.JSONError(w, http.StatusForbidden, "Write access requires full admin role")
					return
				}
			}

			next.ServeHTTP(w, r)
		})
	}
}
