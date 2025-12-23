package auth

import (
	"crypto/rand"
	"encoding/base64"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/crypto"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
)

// Token type constants
const (
	TokenTypeRefresh = "refresh" // JWT refresh tokens
	TokenTypeReset   = "reset"   // Password reset
	TokenTypeConfirm = "confirm" // Email confirmation
	TokenTypeOAuth   = "oauth"   // OAuth provider tokens
)

// UserResponse is the API response structure for user data
// Separates database fields (User) from runtime fields (Roles, Permissions)
type UserResponse struct {
	User        *User    `json:"user"`
	Roles       []string `json:"roles,omitempty"`
	Permissions []string `json:"permissions,omitempty"`
}

// NewUserResponse creates a UserResponse from a User and optional roles
func NewUserResponse(user *User, roles []string) *UserResponse {
	if roles == nil {
		roles = []string{}
	}
	return &UserResponse{
		User:  user,
		Roles: roles,
	}
}

// User represents a user account
type User struct {
	ID              uuid.UUID        `json:"id"`
	Email           string           `json:"email"`
	Password        string           `json:"-"` // Never expose in JSON
	Username        string           `json:"username,omitempty"`
	Confirmed       bool             `json:"confirmed"`
	FirstName       string           `json:"firstName,omitempty"`
	LastName        string           `json:"lastName,omitempty"`
	DisplayName     string           `json:"displayName,omitempty"`
	Phone           string           `json:"phone,omitempty"`
	Location        string           `json:"location,omitempty"`
	ConfirmToken    *string          `json:"-"`
	ConfirmSelector *string          `json:"-"`
	RecoverToken    *string          `json:"-"`
	RecoverTokenExp apptime.NullTime `json:"-"`
	RecoverSelector *string          `json:"-"`
	AttemptCount    int              `json:"-"`
	LastAttempt     apptime.NullTime `json:"-"`
	LastLogin       apptime.NullTime `json:"lastLogin,omitempty"`
	Metadata        string           `json:"metadata,omitempty"`
	CreatedAt       apptime.Time     `json:"createdAt"`
	UpdatedAt       apptime.Time     `json:"updatedAt"`
	DeletedAt       apptime.NullTime `json:"deletedAt,omitempty"`

	// 2FA fields
	TOTPSecret       *string `json:"-"`
	TOTPSecretBackup *string `json:"-"`
	SMSPhoneNumber   *string `json:"-"`
	RecoveryCodes    *string `json:"-"`

	// Note: Relationships are handled at the service layer
}

// TableName returns the table name for auth users
func (User) TableName() string {
	return "auth_users"
}

// PrepareForCreate prepares the user for database insertion
// Prepares model for database insert
func (u *User) PrepareForCreate() error {
	if u.ID == uuid.Nil {
		u.ID = uuid.New()
	}

	// Hash password if it's plain text
	if u.Password != "" && !isHashed(u.Password) {
		hashed, err := crypto.HashPassword(u.Password)
		if err != nil {
			return err
		}
		u.Password = hashed
	}

	now := apptime.NowTime()
	u.CreatedAt = now
	u.UpdatedAt = now

	return nil
}

// PreparePasswordUpdate hashes the password if needed before update
// Prepares model for database update for password changes
func (u *User) PreparePasswordUpdate() error {
	if u.Password != "" && !isHashed(u.Password) {
		hashed, err := crypto.HashPassword(u.Password)
		if err != nil {
			return err
		}
		u.Password = hashed
	}
	u.UpdatedAt = apptime.NowTime()
	return nil
}

// CheckPassword verifies a password against the stored hash
func (u *User) CheckPassword(password string) bool {
	return crypto.ComparePassword(u.Password, password) == nil
}

// SetPassword hashes and sets the password
func (u *User) SetPassword(password string) error {
	hashed, err := crypto.HashPassword(password)
	if err != nil {
		return err
	}
	u.Password = hashed
	return nil
}

// GetID returns the user ID as a string
func (u *User) GetID() string { return u.ID.String() }

// Token represents various auth tokens (refresh, reset, confirm, oauth)
type Token struct {
	ID        uuid.UUID `json:"id"`
	UserID    uuid.UUID `json:"userId"`
	TokenHash string    `json:"-"`    // SHA-256 hash for secure tokens (refresh)
	Token     string    `json:"-"`    // Plain token for short-lived tokens (reset/confirm)
	Type      string    `json:"type"` // refresh, reset, confirm, oauth

	// For refresh token rotation detection
	FamilyID *uuid.UUID `json:"-"`

	// For OAuth tokens
	Provider    *string          `json:"provider,omitempty"` // google, github, microsoft, etc.
	ProviderUID *string          `json:"-"`                  // Provider's user ID
	AccessToken *string          `json:"-"`                  // OAuth access token (encrypted in production)
	OAuthExpiry apptime.NullTime `json:"-"`                  // OAuth token expiry

	// Lifecycle
	ExpiresAt apptime.Time     `json:"expiresAt"`
	UsedAt    apptime.NullTime `json:"usedAt,omitempty"`
	RevokedAt apptime.NullTime `json:"revokedAt,omitempty"`
	CreatedAt apptime.Time     `json:"createdAt"`

	// Audit fields (for refresh tokens - security tracking)
	DeviceInfo *string `json:"deviceInfo,omitempty"` // "Chrome on MacOS"
	IPAddress  *string `json:"ipAddress,omitempty"`  // IPv6 max length
}

// TableName returns the table name for auth tokens
func (Token) TableName() string {
	return "auth_tokens"
}

// PrepareForCreate prepares the token for database insertion
// Prepares model for database insert
func (t *Token) PrepareForCreate() {
	if t.ID == uuid.Nil {
		t.ID = uuid.New()
	}
	t.CreatedAt = apptime.NowTime()
}

// IsValid checks if the token is still valid (not expired, not revoked, not used for single-use tokens)
func (t *Token) IsValid() bool {
	now := apptime.NowTime()
	if t.ExpiresAt.Before(now) {
		return false
	}
	if t.RevokedAt.Valid {
		return false
	}
	// For reset/confirm tokens, check if already used
	if (t.Type == TokenTypeReset || t.Type == TokenTypeConfirm) && t.UsedAt.Valid {
		return false
	}
	return true
}

// Helper function to check if password is already hashed
func isHashed(password string) bool {
	// BCrypt hashes start with $2a$, $2b$, or $2y$
	return len(password) >= 4 && password[0] == '$' && password[1] == '2'
}

// APIKey represents a user's API key for programmatic access
type APIKey struct {
	ID         uuid.UUID        `json:"id"`
	UserID     uuid.UUID        `json:"userId"`
	Name       string           `json:"name"`                 // User-friendly name like "Production Server"
	KeyPrefix  string           `json:"keyPrefix"`            // First 8 chars for identification (e.g., "sb_live_")
	KeyHash    string           `json:"-"`                    // SHA-256 hash of the full key
	Scopes     string           `json:"scopes,omitempty"`     // JSON array of scopes (for future use)
	ExpiresAt  apptime.NullTime `json:"expiresAt,omitempty"`  // Optional expiration
	LastUsedAt apptime.NullTime `json:"lastUsedAt,omitempty"` // Track usage
	LastUsedIP *string          `json:"lastUsedIp,omitempty"` // Last IP that used this key
	RevokedAt  apptime.NullTime `json:"revokedAt,omitempty"`  // Soft revoke
	CreatedAt  apptime.Time     `json:"createdAt"`
	UpdatedAt  apptime.Time     `json:"updatedAt"`
}

// TableName returns the table name for API keys
func (APIKey) TableName() string {
	return "api_keys"
}

// PrepareForCreate prepares the API key for database insertion
// Prepares model for database insert
func (k *APIKey) PrepareForCreate() {
	if k.ID == uuid.Nil {
		k.ID = uuid.New()
	}
	now := apptime.NowTime()
	k.CreatedAt = now
	k.UpdatedAt = now
}

// IsValid checks if the API key is still valid
func (k *APIKey) IsValid() bool {
	if k.RevokedAt.Valid {
		return false
	}
	if k.ExpiresAt.Valid && k.ExpiresAt.Time.Before(apptime.NowTime()) {
		return false
	}
	return true
}

// GenerateAPIKey creates a new API key with prefix
// Returns the full key (only shown once) and the prefix for storage
func GenerateAPIKey(prefix string) (fullKey string, keyPrefix string, keyHash string, err error) {
	// Generate 32 random bytes
	randomBytes := make([]byte, 32)
	if _, err := rand.Read(randomBytes); err != nil {
		return "", "", "", err
	}

	// Create the full key: prefix + base64 encoded random bytes
	randomPart := base64.URLEncoding.EncodeToString(randomBytes)
	fullKey = prefix + randomPart

	// Store prefix for display (first 8 chars of full key)
	keyPrefix = fullKey[:min(8, len(fullKey))]

	// Hash the full key for storage
	keyHash = HashToken(fullKey)

	return fullKey, keyPrefix, keyHash, nil
}
