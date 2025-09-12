package api

import (
	"context"
	"net/http"
	"strings"

	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
	auth "github.com/suppers-ai/auth"
	"github.com/suppers-ai/solobase/services"
	"github.com/suppers-ai/solobase/utils"
)

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
					Role:  claims.Role,
				}
			}

			// Add user to context
			ctx := context.WithValue(r.Context(), "user", user)
			// Also add just the user ID for easier access
			ctx = context.WithValue(ctx, "userID", claims.UserID)
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}
