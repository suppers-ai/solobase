package middleware

import (
	"context"
	"net/http"
	"strings"

	"github.com/dgrijalva/jwt-go"
)

// GetUserIDFromContext retrieves the user ID from the request context
func GetUserIDFromContext(ctx context.Context) (string, bool) {
	userID, ok := ctx.Value("userID").(string)
	return userID, ok
}

// ExtractUserIDFromToken extracts user ID from JWT token in Authorization header
// This is a fallback method when user ID is not in context
func ExtractUserIDFromToken(r *http.Request) string {
	authHeader := r.Header.Get("Authorization")
	if authHeader == "" {
		return ""
	}

	parts := strings.Split(authHeader, " ")
	if len(parts) != 2 || parts[0] != "Bearer" {
		return ""
	}

	// Parse without verification (just to extract claims)
	// Actual verification should be done by auth middleware
	token, _ := jwt.Parse(parts[1], nil)
	if token == nil {
		return ""
	}

	claims, ok := token.Claims.(jwt.MapClaims)
	if !ok {
		return ""
	}

	// Try different claim keys that might contain user ID
	if userID, ok := claims["user_id"].(string); ok {
		return userID
	}
	if userID, ok := claims["sub"].(string); ok {
		return userID
	}
	if userID, ok := claims["userId"].(string); ok {
		return userID
	}

	return ""
}
