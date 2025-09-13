package middleware

import (
	"log"
	"net/http"

	"github.com/suppers-ai/solobase/internal/iam"
	"github.com/suppers-ai/solobase/utils"
)

// AdminMiddleware ensures the user has admin role
func AdminMiddleware(iamService *iam.Service) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
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
			log.Printf("AdminMiddleware: Checking roles for user ID: %s", userIDStr)
			roles, err := iamService.GetUserRoles(r.Context(), userIDStr)
			if err != nil {
				log.Printf("AdminMiddleware: Failed to get roles for user %s: %v", userIDStr, err)
				utils.JSONError(w, http.StatusInternalServerError, "Failed to check user roles")
				return
			}
			log.Printf("AdminMiddleware: User %s has roles: %v", userIDStr, roles)

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