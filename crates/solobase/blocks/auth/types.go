package auth

import (
	"crypto/rand"
	"encoding/base64"

	"github.com/suppers-ai/solobase/core/apptime"
	coreauth "github.com/suppers-ai/solobase/core/auth"
	"github.com/suppers-ai/solobase/core/uuid"
	cryptosvc "github.com/suppers-ai/waffle-go/services/crypto"
	"github.com/suppers-ai/waffle-go/services/database"
)

// API Key prefix for all generated keys.
const APIKeyPrefix = "sb_"

// LoginRequest is the request body for login.
type LoginRequest struct {
	Email    string `json:"email"`
	Password string `json:"password"`
}

// SignupRequest is the request body for signup.
type SignupRequest struct {
	Email    string                 `json:"email"`
	Password string                 `json:"password"`
	Metadata map[string]interface{} `json:"metadata,omitempty"`
}

// ChangePasswordRequest is the request body for changing password.
type ChangePasswordRequest struct {
	CurrentPassword string `json:"currentPassword"`
	NewPassword     string `json:"newPassword"`
}

// CreateAPIKeyRequest is the request body for creating an API key.
type CreateAPIKeyRequest struct {
	Name      string        `json:"name"`
	ExpiresAt *apptime.Time `json:"expiresAt,omitempty"`
}

// CreateAPIKeyResponse is the response for creating an API key.
type CreateAPIKeyResponse struct {
	ID        uuid.UUID     `json:"id"`
	Name      string        `json:"name"`
	Key       string        `json:"key"`
	KeyPrefix string        `json:"keyPrefix"`
	ExpiresAt *apptime.Time `json:"expiresAt,omitempty"`
	CreatedAt apptime.Time  `json:"createdAt"`
}

// APIKeyResponse is the response for listing API keys (without the full key).
type APIKeyResponse struct {
	ID         uuid.UUID     `json:"id"`
	Name       string        `json:"name"`
	KeyPrefix  string        `json:"keyPrefix"`
	ExpiresAt  *apptime.Time `json:"expiresAt,omitempty"`
	LastUsedAt *apptime.Time `json:"lastUsedAt,omitempty"`
	LastUsedIP *string       `json:"lastUsedIp,omitempty"`
	CreatedAt  apptime.Time  `json:"createdAt"`
}

// generateRefreshToken creates a secure random refresh token.
func generateRefreshToken() (string, error) {
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return base64.URLEncoding.EncodeToString(b), nil
}

// GenerateAccessToken creates a short-lived JWT access token.
// Delegates to core/auth.GenerateAccessToken.
func GenerateAccessToken(crypto cryptosvc.Service, userID, email string, db database.Service) (string, error) {
	return coreauth.GenerateAccessToken(crypto, userID, email, db)
}
