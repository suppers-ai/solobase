package auth

import (
	"time"

	"github.com/google/uuid"
	"golang.org/x/crypto/bcrypt"
	"gorm.io/gorm"
)

// UserRole represents user roles
type UserRole string

const (
	RoleUser    UserRole = "user"
	RoleManager UserRole = "manager"
	RoleAdmin   UserRole = "admin"
	RoleDeleted UserRole = "deleted"
)

// User represents a user account
type User struct {
	ID              uuid.UUID  `gorm:"type:char(36);primary_key" json:"id"`
	Email           string     `gorm:"uniqueIndex;not null;size:255" json:"email"`
	Password        string     `gorm:"not null" json:"-"` // Never expose in JSON
	Username        string     `gorm:"size:255" json:"username,omitempty"`
	Confirmed       bool       `gorm:"default:false" json:"confirmed"`
	FirstName       string     `gorm:"size:100" json:"first_name,omitempty"`
	LastName        string     `gorm:"size:100" json:"last_name,omitempty"`
	DisplayName     string     `gorm:"size:100" json:"display_name,omitempty"`
	Phone           string     `gorm:"size:50" json:"phone,omitempty"`
	Location        string     `gorm:"size:255" json:"location,omitempty"`
	ConfirmToken    *string    `gorm:"size:255" json:"-"`
	ConfirmSelector *string    `gorm:"size:255;index" json:"-"`
	RecoverToken    *string    `gorm:"size:255" json:"-"`
	RecoverTokenExp *time.Time `json:"-"`
	RecoverSelector *string    `gorm:"size:255;index" json:"-"`
	AttemptCount    int        `gorm:"default:0" json:"-"`
	LastAttempt     *time.Time `json:"-"`
	LastLogin       *time.Time `json:"last_login,omitempty"`
	Metadata        string     `gorm:"type:text" json:"metadata,omitempty"`
	CreatedAt       time.Time  `json:"created_at"`
	UpdatedAt       time.Time  `json:"updated_at"`
	DeletedAt       *time.Time `gorm:"index" json:"deleted_at,omitempty"`

	// OAuth2 fields for authboss
	OAuth2UID      *string    `gorm:"size:255" json:"-"`
	OAuth2Provider *string    `gorm:"size:50" json:"-"`
	OAuth2Token    *string    `gorm:"type:text" json:"-"`
	OAuth2Refresh  *string    `gorm:"type:text" json:"-"`
	OAuth2Expiry   *time.Time `json:"-"`

	// 2FA fields
	TOTPSecret       *string `gorm:"size:255" json:"-"`
	TOTPSecretBackup *string `gorm:"size:255" json:"-"`
	SMSPhoneNumber   *string `gorm:"size:50" json:"-"`
	RecoveryCodes    *string `gorm:"type:text" json:"-"`

	// Relationships
	Sessions []Session `gorm:"foreignKey:UserID;constraint:OnDelete:CASCADE" json:"-"`
	Tokens   []Token   `gorm:"foreignKey:UserID;constraint:OnDelete:CASCADE" json:"-"`
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

// Authboss OAuth2 interface
func (u *User) IsOAuth2User() bool {
	return u.OAuth2UID != nil && u.OAuth2Provider != nil
}
func (u *User) GetOAuth2UID() string {
	if u.OAuth2UID != nil {
		return *u.OAuth2UID
	}
	return ""
}
func (u *User) PutOAuth2UID(uid string) {
	u.OAuth2UID = &uid
}
func (u *User) GetOAuth2Provider() string {
	if u.OAuth2Provider != nil {
		return *u.OAuth2Provider
	}
	return ""
}
func (u *User) PutOAuth2Provider(provider string) {
	u.OAuth2Provider = &provider
}
func (u *User) GetOAuth2AccessToken() string {
	if u.OAuth2Token != nil {
		return *u.OAuth2Token
	}
	return ""
}
func (u *User) PutOAuth2AccessToken(token string) {
	u.OAuth2Token = &token
}
func (u *User) GetOAuth2RefreshToken() string {
	if u.OAuth2Refresh != nil {
		return *u.OAuth2Refresh
	}
	return ""
}
func (u *User) PutOAuth2RefreshToken(token string) {
	u.OAuth2Refresh = &token
}
func (u *User) GetOAuth2Expiry() time.Time {
	if u.OAuth2Expiry != nil {
		return *u.OAuth2Expiry
	}
	return time.Time{}
}
func (u *User) PutOAuth2Expiry(expiry time.Time) {
	u.OAuth2Expiry = &expiry
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
	// No role field to update anymore
}

// Session represents a user session
type Session struct {
	ID        string    `gorm:"type:varchar(255);primary_key" json:"id"`
	UserID    uuid.UUID `gorm:"type:char(36);not null;index" json:"user_id"`
	Token     string    `gorm:"uniqueIndex;not null;size:255" json:"token"`
	Data      []byte    `json:"-"`
	ExpiresAt time.Time `gorm:"not null;index" json:"expires_at"`
	CreatedAt time.Time `json:"created_at"`
	UpdatedAt time.Time `json:"updated_at"`

	// Relationships
	User User `gorm:"foreignKey:UserID" json:"-"`
}

// TableName specifies the table name
func (Session) TableName() string {
	return "sessions"
}

// Token represents various auth tokens
type Token struct {
	ID        uuid.UUID  `gorm:"type:char(36);primary_key" json:"id"`
	UserID    uuid.UUID  `gorm:"type:char(36);not null;index" json:"user_id"`
	Token     string     `gorm:"uniqueIndex;not null;size:255" json:"token"`
	Type      string     `gorm:"not null;size:50;index" json:"type"` // reset, confirm, etc.
	ExpiresAt time.Time  `gorm:"not null" json:"expires_at"`
	UsedAt    *time.Time `json:"used_at,omitempty"`
	CreatedAt time.Time  `json:"created_at"`

	// Relationships
	User User `gorm:"foreignKey:UserID" json:"-"`
}

// TableName specifies the table name
func (Token) TableName() string {
	return "tokens"
}

// BeforeCreate hook
func (t *Token) BeforeCreate(tx *gorm.DB) error {
	if t.ID == uuid.Nil {
		t.ID = uuid.New()
	}
	return nil
}

// Helper function to check if password is already hashed
func isHashed(password string) bool {
	// BCrypt hashes start with $2a$, $2b$, or $2y$
	return len(password) >= 4 && password[0] == '$' && password[1] == '2'
}
