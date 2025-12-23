package interfaces

import (
	"context"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

// JWTSigner defines the interface for JWT token operations.
// Implementations should be TinyGo compatible.
type JWTSigner interface {
	// Sign creates a signed JWT token
	Sign(claims JWTClaims) (string, error)

	// Verify verifies and parses a JWT token
	Verify(token string) (*JWTClaims, error)

	// Refresh creates a new token with extended expiration
	Refresh(token string) (string, error)
}

// JWTClaims represents JWT token claims
type JWTClaims struct {
	UserID    string            `json:"user_id"`
	Email     string            `json:"email"`
	Roles     []string          `json:"roles"`
	ExpiresAt apptime.Time         `json:"exp"`
	IssuedAt  apptime.Time         `json:"iat"`
	Extra     map[string]string `json:"extra,omitempty"`
}

// TokenGenerator defines the interface for generating secure tokens.
type TokenGenerator interface {
	// GenerateToken generates a cryptographically secure random token
	GenerateToken(length int) (string, error)

	// GenerateUUID generates a UUID v4
	GenerateUUID() (string, error)
}

// AuthConfig contains authentication configuration
type AuthConfig struct {
	// Password hashing
	BCryptCost int // For bcrypt adapter (default 10)

	// JWT
	JWTSecret          string
	JWTExpirationHours int

	// Session
	SessionName   string
	SessionMaxAge int // in seconds
	CookieSecure  bool
	CookieDomain  string

	// CSRF
	CSRFKey string
}

// User represents an authenticated user
type User interface {
	GetID() string
	GetEmail() string
	GetRoles() []string
	IsAdmin() bool
}

// AuthService defines the high-level authentication service interface.
// This is optional - some implementations may use individual interfaces directly.
type AuthService interface {
	// User operations
	CreateUser(ctx context.Context, email, password string) (User, error)
	GetUser(ctx context.Context, id string) (User, error)
	GetUserByEmail(ctx context.Context, email string) (User, error)
	ValidateCredentials(ctx context.Context, email, password string) (User, error)
	UpdatePassword(ctx context.Context, userID, oldPassword, newPassword string) error
	DeleteUser(ctx context.Context, id string) error

	// Token operations
	CreateAccessToken(user User) (string, error)
	CreateRefreshToken(user User) (string, error)
	ValidateAccessToken(token string) (User, error)
	RefreshAccessToken(refreshToken string) (string, error)

	// Session operations (optional, may return ErrNotSupported)
	CreateSession(ctx context.Context, userID string) (string, error)
	GetSession(ctx context.Context, sessionID string) (User, error)
	DestroySession(ctx context.Context, sessionID string) error
}
