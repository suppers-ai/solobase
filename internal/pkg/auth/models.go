package auth

import (
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"time"

	"github.com/google/uuid"
	"golang.org/x/crypto/bcrypt"
	"gorm.io/gorm"
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
	ID              uuid.UUID  `gorm:"type:char(36);primary_key" json:"id"`
	Email           string     `gorm:"uniqueIndex;not null;size:255" json:"email"`
	Password        string     `gorm:"not null" json:"-"` // Never expose in JSON
	Username        string     `gorm:"size:255" json:"username,omitempty"`
	Confirmed       bool       `gorm:"default:false" json:"confirmed"`
	FirstName       string     `gorm:"size:100" json:"firstName,omitempty"`
	LastName        string     `gorm:"size:100" json:"lastName,omitempty"`
	DisplayName     string     `gorm:"size:100" json:"displayName,omitempty"`
	Phone           string     `gorm:"size:50" json:"phone,omitempty"`
	Location        string     `gorm:"size:255" json:"location,omitempty"`
	ConfirmToken    *string    `gorm:"size:255" json:"-"`
	ConfirmSelector *string    `gorm:"size:255;index" json:"-"`
	RecoverToken    *string    `gorm:"size:255" json:"-"`
	RecoverTokenExp *time.Time `json:"-"`
	RecoverSelector *string    `gorm:"size:255;index" json:"-"`
	AttemptCount    int        `gorm:"default:0" json:"-"`
	LastAttempt     *time.Time `json:"-"`
	LastLogin       *time.Time `json:"lastLogin,omitempty"`
	Metadata        string     `gorm:"type:text" json:"metadata,omitempty"`
	CreatedAt       time.Time  `json:"createdAt"`
	UpdatedAt       time.Time  `json:"updatedAt"`
	DeletedAt       *time.Time `gorm:"index" json:"deletedAt,omitempty"`

	// 2FA fields
	TOTPSecret       *string `gorm:"size:255" json:"-"`
	TOTPSecretBackup *string `gorm:"size:255" json:"-"`
	SMSPhoneNumber   *string `gorm:"size:50" json:"-"`
	RecoveryCodes    *string `gorm:"type:text" json:"-"`

	// Relationships
	Tokens []Token `gorm:"foreignKey:UserID;constraint:OnDelete:CASCADE" json:"-"`
}

// TableName specifies the table name
func (User) TableName() string {
	return "auth_users"
}

// BeforeCreate hook
func (u *User) BeforeCreate(tx *gorm.DB) error {
	if u.ID == uuid.Nil {
		u.ID = uuid.New()
	}

	// Hash password if it's plain text
	if u.Password != "" && !isHashed(u.Password) {
		hashed, err := bcrypt.GenerateFromPassword([]byte(u.Password), bcrypt.DefaultCost)
		if err != nil {
			return err
		}
		u.Password = string(hashed)
	}

	return nil
}

// BeforeUpdate hook
func (u *User) BeforeUpdate(tx *gorm.DB) error {
	// Hash password if it changed and is plain text
	if tx.Statement.Changed("Password") && u.Password != "" && !isHashed(u.Password) {
		hashed, err := bcrypt.GenerateFromPassword([]byte(u.Password), bcrypt.DefaultCost)
		if err != nil {
			return err
		}
		u.Password = string(hashed)
	}
	return nil
}

// CheckPassword verifies a password
func (u *User) CheckPassword(password string) bool {
	err := bcrypt.CompareHashAndPassword([]byte(u.Password), []byte(password))
	return err == nil
}

// SetPassword hashes and sets the password
func (u *User) SetPassword(password string) error {
	hashed, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
	if err != nil {
		return err
	}
	u.Password = string(hashed)
	return nil
}

// Authboss interface implementations
func (u *User) GetPID() string { return u.ID.String() }
func (u *User) PutPID(id string) {
	if uid, err := uuid.Parse(id); err == nil {
		u.ID = uid
	}
}
func (u *User) GetEmail() string            { return u.Email }
func (u *User) PutEmail(email string)       { u.Email = email }
func (u *User) GetUsername() string         { return u.Username }
func (u *User) PutUsername(username string) { u.Username = username }
func (u *User) GetPassword() string         { return u.Password }
func (u *User) PutPassword(password string) { u.Password = password }
func (u *User) GetConfirmed() bool          { return u.Confirmed }
func (u *User) PutConfirmed(confirmed bool) { u.Confirmed = confirmed }

// Authboss confirm interface
func (u *User) GetConfirmSelector() string {
	if u.ConfirmSelector != nil {
		return *u.ConfirmSelector
	}
	return ""
}
func (u *User) PutConfirmSelector(selector string) {
	u.ConfirmSelector = &selector
}
func (u *User) GetConfirmVerifier() string {
	if u.ConfirmToken != nil {
		return *u.ConfirmToken
	}
	return ""
}
func (u *User) PutConfirmVerifier(verifier string) {
	u.ConfirmToken = &verifier
}

// Authboss recover interface
func (u *User) GetRecoverSelector() string {
	if u.RecoverSelector != nil {
		return *u.RecoverSelector
	}
	return ""
}
func (u *User) PutRecoverSelector(selector string) {
	u.RecoverSelector = &selector
}
func (u *User) GetRecoverVerifier() string {
	if u.RecoverToken != nil {
		return *u.RecoverToken
	}
	return ""
}
func (u *User) PutRecoverVerifier(verifier string) {
	u.RecoverToken = &verifier
}
func (u *User) GetRecoverExpiry() time.Time {
	if u.RecoverTokenExp != nil {
		return *u.RecoverTokenExp
	}
	return time.Time{}
}
func (u *User) PutRecoverExpiry(expiry time.Time) {
	u.RecoverTokenExp = &expiry
}

// Authboss lock interface
func (u *User) GetAttemptCount() int      { return u.AttemptCount }
func (u *User) PutAttemptCount(count int) { u.AttemptCount = count }
func (u *User) GetLastAttempt() time.Time {
	if u.LastAttempt != nil {
		return *u.LastAttempt
	}
	return time.Time{}
}
func (u *User) PutLastAttempt(attempt time.Time) {
	u.LastAttempt = &attempt
}

// Authboss 2FA interface
func (u *User) GetTOTPSecretKey() string {
	if u.TOTPSecret != nil {
		return *u.TOTPSecret
	}
	return ""
}
func (u *User) PutTOTPSecretKey(secret string) {
	u.TOTPSecret = &secret
}
func (u *User) GetSMSPhoneNumber() string {
	if u.SMSPhoneNumber != nil {
		return *u.SMSPhoneNumber
	}
	return ""
}
func (u *User) PutSMSPhoneNumber(phone string) {
	u.SMSPhoneNumber = &phone
}
func (u *User) GetRecoveryCodes() string {
	if u.RecoveryCodes != nil {
		return *u.RecoveryCodes
	}
	return ""
}
func (u *User) PutRecoveryCodes(codes string) {
	u.RecoveryCodes = &codes
}

// Authboss arbitrary interface
func (u *User) GetArbitrary() map[string]string {
	return map[string]string{
		"created_at": u.CreatedAt.Format(time.RFC3339),
		"updated_at": u.UpdatedAt.Format(time.RFC3339),
	}
}
func (u *User) PutArbitrary(values map[string]string) {
	// No additional fields to update
}

// Token represents various auth tokens (refresh, reset, confirm, oauth)
type Token struct {
	ID        uuid.UUID  `gorm:"type:char(36);primary_key" json:"id"`
	UserID    uuid.UUID  `gorm:"type:char(36);not null;index" json:"userId"`
	TokenHash string     `gorm:"size:64;index" json:"-"`              // SHA-256 hash for secure tokens (refresh)
	Token     string     `gorm:"size:255;index" json:"-"`             // Plain token for short-lived tokens (reset/confirm)
	Type      string     `gorm:"not null;size:50;index" json:"type"`  // refresh, reset, confirm, oauth

	// For refresh token rotation detection
	FamilyID *uuid.UUID `gorm:"type:char(36);index" json:"-"`

	// For OAuth tokens
	Provider    *string    `gorm:"size:50" json:"provider,omitempty"`  // google, github, microsoft, etc.
	ProviderUID *string    `gorm:"size:255;index" json:"-"`            // Provider's user ID
	AccessToken *string    `gorm:"type:text" json:"-"`                 // OAuth access token (encrypted in production)
	OAuthExpiry *time.Time `json:"-"`                                  // OAuth token expiry

	// Lifecycle
	ExpiresAt time.Time  `gorm:"not null;index" json:"expiresAt"`
	UsedAt    *time.Time `json:"usedAt,omitempty"`
	RevokedAt *time.Time `gorm:"index" json:"revokedAt,omitempty"`
	CreatedAt time.Time  `json:"createdAt"`

	// Audit fields (for refresh tokens - security tracking)
	DeviceInfo *string `gorm:"size:255" json:"deviceInfo,omitempty"` // "Chrome on MacOS"
	IPAddress  *string `gorm:"size:45" json:"ipAddress,omitempty"`   // IPv6 max length

	// Relationships
	User User `gorm:"foreignKey:UserID" json:"-"`
}

// TableName specifies the table name
func (Token) TableName() string {
	return "auth_tokens"
}

// BeforeCreate hook
func (t *Token) BeforeCreate(tx *gorm.DB) error {
	if t.ID == uuid.Nil {
		t.ID = uuid.New()
	}
	return nil
}

// IsValid checks if the token is still valid (not expired, not revoked, not used for single-use tokens)
func (t *Token) IsValid() bool {
	now := time.Now()
	if t.ExpiresAt.Before(now) {
		return false
	}
	if t.RevokedAt != nil {
		return false
	}
	// For reset/confirm tokens, check if already used
	if (t.Type == TokenTypeReset || t.Type == TokenTypeConfirm) && t.UsedAt != nil {
		return false
	}
	return true
}

// HashToken creates a SHA-256 hash of a token string
func HashToken(token string) string {
	hash := sha256.Sum256([]byte(token))
	return hex.EncodeToString(hash[:])
}

// Helper function to check if password is already hashed
func isHashed(password string) bool {
	// BCrypt hashes start with $2a$, $2b$, or $2y$
	return len(password) >= 4 && password[0] == '$' && password[1] == '2'
}

// APIKey represents a user's API key for programmatic access
type APIKey struct {
	ID          uuid.UUID  `gorm:"type:char(36);primary_key" json:"id"`
	UserID      uuid.UUID  `gorm:"type:char(36);not null;index" json:"userId"`
	Name        string     `gorm:"size:255;not null" json:"name"`                  // User-friendly name like "Production Server"
	KeyPrefix   string     `gorm:"size:8;not null;index" json:"keyPrefix"`         // First 8 chars for identification (e.g., "sb_live_")
	KeyHash     string     `gorm:"size:64;not null;uniqueIndex" json:"-"`          // SHA-256 hash of the full key
	Scopes      string     `gorm:"type:text" json:"scopes,omitempty"`              // JSON array of scopes (for future use)
	ExpiresAt   *time.Time `json:"expiresAt,omitempty"`                            // Optional expiration
	LastUsedAt  *time.Time `json:"lastUsedAt,omitempty"`                           // Track usage
	LastUsedIP  *string    `gorm:"size:45" json:"lastUsedIp,omitempty"`            // Last IP that used this key
	RevokedAt   *time.Time `gorm:"index" json:"revokedAt,omitempty"`               // Soft revoke
	CreatedAt   time.Time  `json:"createdAt"`
	UpdatedAt   time.Time  `json:"updatedAt"`

	// Relationships
	User User `gorm:"foreignKey:UserID" json:"-"`
}

// TableName specifies the table name
func (APIKey) TableName() string {
	return "api_keys"
}

// BeforeCreate hook
func (k *APIKey) BeforeCreate(tx *gorm.DB) error {
	if k.ID == uuid.Nil {
		k.ID = uuid.New()
	}
	return nil
}

// IsValid checks if the API key is still valid
func (k *APIKey) IsValid() bool {
	if k.RevokedAt != nil {
		return false
	}
	if k.ExpiresAt != nil && k.ExpiresAt.Before(time.Now()) {
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
