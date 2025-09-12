package middleware

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"
	"strings"

	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
	auth "github.com/suppers-ai/auth"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/utils"
)

var jwtSecret []byte

func init() {
	// Initialize JWT secret on package load
	// Don't fail on init, let the application handle it
	_ = SetJWTSecret("")
}

// SetJWTSecret sets the JWT secret for authentication
func SetJWTSecret(secret string) error {
	if secret == "" {
		// Use environment variable as fallback
		secret = os.Getenv("JWT_SECRET")
	}
	if secret == "" {
		// Use a default for development only
		if os.Getenv("ENVIRONMENT") == "development" {
			secret = "dev-secret-key-do-not-use-in-production"
			log.Println("WARNING: Using default JWT secret for development. DO NOT use in production!")
		}
	}
	if secret == "" {
		return fmt.Errorf("JWT secret is required. Set JWT_SECRET environment variable or pass it in configuration")
	}
	jwtSecret = []byte(secret)
	return nil
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
				return jwtSecret, nil
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
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}
