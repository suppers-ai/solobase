package middleware

import (
	"context"
	"log"
	"net/http"
	"strings"

	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
	"github.com/suppers-ai/solobase/constants"
	auth "github.com/suppers-ai/solobase/internal/pkg/auth"
	commonjwt "github.com/suppers-ai/solobase/internal/common/jwt"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/internal/iam"
	"github.com/suppers-ai/solobase/utils"
	"gorm.io/gorm"
)

// Package-level variable for database access (set during initialization)
var authDB *gorm.DB
var iamService *iam.Service

// SetAuthDB sets the database for API key authentication
func SetAuthDB(db *gorm.DB) {
	authDB = db
}

// SetIAMService sets the IAM service for role lookups
func SetIAMService(svc *iam.Service) {
	iamService = svc
}

// SetJWTSecret sets the JWT secret for authentication
// Delegates to the common JWT package
func SetJWTSecret(secret string) error {
	return commonjwt.SetJWTSecret(secret)
}

func AuthMiddleware(authService *services.AuthService) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Try authentication methods in order:
			// 1. httpOnly cookie (browsers)
			// 2. Bearer token in Authorization header (JWT)
			// 3. API key in Authorization header (sb_ prefix)

			var tokenString string
			var isAPIKey bool

			// 1. Try cookie first (browsers)
			cookie, err := r.Cookie("auth_token")
			if err == nil && cookie.Value != "" {
				tokenString = cookie.Value
			} else {
				// 2/3. Check Authorization header
				authHeader := r.Header.Get("Authorization")
				if authHeader == "" {
					utils.JSONError(w, http.StatusUnauthorized, "No authorization provided")
					return
				}

				tokenString = strings.TrimPrefix(authHeader, "Bearer ")
				if tokenString == authHeader {
					utils.JSONError(w, http.StatusUnauthorized, "Invalid authorization header format")
					return
				}

				// Check if it's an API key (starts with sb_)
				if strings.HasPrefix(tokenString, "sb_") {
					isAPIKey = true
				}
			}

			var userID string
			var userEmail string
			var userRoles []string
			var user interface{}

			if isAPIKey {
				// Authenticate via API key
				if authDB == nil {
					log.Printf("API key authentication failed: database not initialized")
					utils.JSONError(w, http.StatusInternalServerError, "Server configuration error")
					return
				}

				// Hash the API key and look it up
				keyHash := auth.HashToken(tokenString)
				storage := auth.NewGormStorage(authDB)
				apiKey, err := storage.GetAPIKeyByHash(r.Context(), keyHash)
				if err != nil {
					utils.JSONError(w, http.StatusUnauthorized, "Invalid API key")
					return
				}

				// Check if API key is valid (not expired, not revoked)
				if !apiKey.IsValid() {
					utils.JSONError(w, http.StatusUnauthorized, "API key expired or revoked")
					return
				}

				// Get the user associated with this API key
				fullUser, err := authService.GetUserByID(apiKey.UserID.String())
				if err != nil {
					utils.JSONError(w, http.StatusUnauthorized, "User not found")
					return
				}

				userID = apiKey.UserID.String()
				userEmail = fullUser.Email
				user = fullUser

				// Get user's roles from IAM
				if iamService != nil {
					roles, err := iamService.GetUserRoles(r.Context(), userID)
					if err != nil {
						log.Printf("Warning: Failed to fetch roles for API key user: %v", err)
						userRoles = []string{}
					} else {
						userRoles = roles
					}
				}

				// Update last used timestamp (async, don't block the request)
				go func() {
					ip := getClientIPFromRequest(r)
					_ = storage.UpdateAPIKeyLastUsed(context.Background(), apiKey.ID, ip)
				}()

			} else {
				// Authenticate via JWT
				claims := &Claims{}
				token, err := jwt.ParseWithClaims(tokenString, claims, func(token *jwt.Token) (interface{}, error) {
					return commonjwt.GetJWTSecret(), nil
				})

				if err != nil || !token.Valid {
					utils.JSONError(w, http.StatusUnauthorized, "Invalid token")
					return
				}

				userID = claims.UserID
				userEmail = claims.Email
				userRoles = claims.Roles

				// For /auth/me endpoint, we need full user data from DB
				// For other endpoints, we can use token claims to avoid DB lookup
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
					}
				}
			}

			// Add user to context
			ctx := context.WithValue(r.Context(), "user", user)
			// Also add just the user ID for easier access
			ctx = context.WithValue(ctx, "userID", userID)
			// Add roles to context
			ctx = context.WithValue(ctx, "user_roles", userRoles)
			// Add constants keys for products extension compatibility
			ctx = context.WithValue(ctx, constants.ContextKeyUserID, userID)
			ctx = context.WithValue(ctx, constants.ContextKeyUserEmail, userEmail)
			ctx = context.WithValue(ctx, constants.ContextKeyUserRoles, userRoles)
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

// getClientIPFromRequest extracts the client IP from a request
func getClientIPFromRequest(r *http.Request) string {
	// Check X-Forwarded-For header first (for proxies)
	if xff := r.Header.Get("X-Forwarded-For"); xff != "" {
		parts := strings.Split(xff, ",")
		return strings.TrimSpace(parts[0])
	}
	// Check X-Real-IP header
	if xri := r.Header.Get("X-Real-IP"); xri != "" {
		return xri
	}
	// Fall back to RemoteAddr
	ip := r.RemoteAddr
	// Remove port if present
	if idx := strings.LastIndex(ip, ":"); idx != -1 {
		ip = ip[:idx]
	}
	return ip
}
