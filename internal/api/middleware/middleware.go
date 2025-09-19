package middleware

import (
	"context"
	"net/http"
	"strings"

	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
	"github.com/suppers-ai/solobase/constants"
	auth "github.com/suppers-ai/solobase/internal/pkg/auth"
	commonjwt "github.com/suppers-ai/solobase/internal/common/jwt"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/utils"
)

// SetJWTSecret sets the JWT secret for authentication
// Delegates to the common JWT package
func SetJWTSecret(secret string) error {
	return commonjwt.SetJWTSecret(secret)
}

func CORSMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS, PATCH")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")
		w.Header().Set("Access-Control-Allow-Credentials", "true")

		if r.Method == "OPTIONS" {
			w.WriteHeader(http.StatusNoContent)
			return
		}

		next.ServeHTTP(w, r)
	})
}

func AuthMiddleware(authService *services.AuthService) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			authHeader := r.Header.Get("Authorization")
			if authHeader == "" {
				utils.JSONError(w, http.StatusUnauthorized, "No authorization header")
				return
			}

			tokenString := strings.TrimPrefix(authHeader, "Bearer ")
			if tokenString == authHeader {
				utils.JSONError(w, http.StatusUnauthorized, "Invalid authorization header format")
				return
			}

			claims := &Claims{}
			token, err := jwt.ParseWithClaims(tokenString, claims, func(token *jwt.Token) (interface{}, error) {
				return commonjwt.GetJWTSecret(), nil
			})

			if err != nil || !token.Valid {
				utils.JSONError(w, http.StatusUnauthorized, "Invalid token")
				return
			}

			// For /auth/me endpoint, we need full user data from DB
			// For other endpoints, we can use token claims to avoid DB lookup
			var user interface{}
			if r.URL.Path == "/api/auth/me" {
				// Get full user from database for profile endpoint
				fullUser, err := authService.GetUserByID(claims.UserID)
				if err != nil {
					utils.JSONError(w, http.StatusUnauthorized, "User not found")
					return
				}
				user = fullUser
			} else {
				// For other endpoints, create lightweight user from token claims
				// This avoids database lookup on every request
				user = &auth.User{
					ID:    uuid.MustParse(claims.UserID),
					Email: claims.Email,
					// Role removed - roles are now in JWT claims.Roles
				}
			}

			// Add user to context
			ctx := context.WithValue(r.Context(), "user", user)
			// Also add just the user ID for easier access
			ctx = context.WithValue(ctx, "userID", claims.UserID)
			// Add roles from JWT claims to context
			ctx = context.WithValue(ctx, "user_roles", claims.Roles)
			// Add constants keys for products extension compatibility
			ctx = context.WithValue(ctx, constants.ContextKeyUserID, claims.UserID)
			ctx = context.WithValue(ctx, constants.ContextKeyUserEmail, claims.Email)
			ctx = context.WithValue(ctx, constants.ContextKeyUserRoles, claims.Roles)
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}
