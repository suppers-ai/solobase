package middleware

import (
	"context"
	"net/http"
	"strings"
	"time"

	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
	auth "github.com/suppers-ai/auth"
	"github.com/suppers-ai/solobase/constants"
	"github.com/suppers-ai/solobase/services"
	"github.com/suppers-ai/solobase/utils"
	"github.com/volatiletech/authboss/v3"
)

type Claims struct {
	UserID string `json:"user_id"`
	Email  string `json:"email"`
	Role   string `json:"role"`
	jwt.RegisteredClaims
}

// JWTAuth middleware for API authentication with optional auth service for full user data
func JWTAuth(secret string, authService *services.AuthService) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Extract token from Authorization header
			authHeader := r.Header.Get("Authorization")
			if authHeader == "" {
				respondWithError(w, http.StatusUnauthorized, constants.ErrMissingAuthHeader)
				return
			}

			// Check Bearer prefix
			tokenString := strings.TrimPrefix(authHeader, "Bearer ")
			if tokenString == authHeader {
				respondWithError(w, http.StatusUnauthorized, constants.ErrInvalidAuthFormat)
				return
			}

			// Parse and validate token
			token, err := jwt.ParseWithClaims(tokenString, &Claims{}, func(token *jwt.Token) (interface{}, error) {
				return []byte(secret), nil
			})

			if err != nil || !token.Valid {
				respondWithError(w, http.StatusUnauthorized, constants.ErrInvalidToken)
				return
			}

			// Extract claims
			if claims, ok := token.Claims.(*Claims); ok {
				// For /auth/me endpoint, we need full user data from DB if authService is provided
				// For other endpoints, we can use token claims to avoid DB lookup
				var user interface{}
				if authService != nil && r.URL.Path == "/api/auth/me" {
					// Get full user from database for profile endpoint
					fullUser, err := authService.GetUserByID(claims.UserID)
					if err != nil {
						respondWithError(w, http.StatusUnauthorized, "User not found")
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

				// Add user info to context (both old and new style for compatibility)
				ctx := context.WithValue(r.Context(), constants.ContextKeyUserID, claims.UserID)
				ctx = context.WithValue(ctx, constants.ContextKeyUserEmail, claims.Email)
				ctx = context.WithValue(ctx, constants.ContextKeyUserRole, claims.Role)

				// Also add user object and userID for api/middleware.go compatibility
				ctx = context.WithValue(ctx, "user", user)
				ctx = context.WithValue(ctx, "userID", claims.UserID)

				next.ServeHTTP(w, r.WithContext(ctx))
			} else {
				respondWithError(w, http.StatusUnauthorized, "Invalid token claims")
			}
		})
	}
}

// RequireAdmin middleware is now in rbac.go with service parameter

// GenerateToken generates a JWT token for a user
func GenerateToken(secret string, userID, email, role string) (string, error) {
	claims := &Claims{
		UserID: userID,
		Email:  email,
		Role:   role,
		RegisteredClaims: jwt.RegisteredClaims{
			ExpiresAt: jwt.NewNumericDate(time.Now().Add(24 * time.Hour)),
			IssuedAt:  jwt.NewNumericDate(time.Now()),
		},
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	return token.SignedString([]byte(secret))
}

func respondWithError(w http.ResponseWriter, code int, message string) {
	utils.JSONError(w, code, message)
}

// AuthBossBridge extracts the user from authboss context and sets it for RBAC middleware
func AuthBossBridge(authService interface {
	CurrentUser(*http.Request) (authboss.User, error)
}) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Get authboss user from context
			user, err := authService.CurrentUser(r)
			if err != nil || user == nil {
				// No user in context, likely not authenticated
				next.ServeHTTP(w, r)
				return
			}

			// Extract user ID and add it to context for RBAC middleware
			userID := user.GetPID()
			ctx := context.WithValue(r.Context(), "user_id", userID)

			// Continue with updated context
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}
