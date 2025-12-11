package auth

import (
	"context"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/volatiletech/authboss/v3"
	"gorm.io/gorm"
)

// GormStorage implements authboss storage interfaces using GORM
type GormStorage struct {
	db *gorm.DB
}

// NewGormStorage creates a new GORM-based storage
func NewGormStorage(db *gorm.DB) *GormStorage {
	return &GormStorage{db: db}
}

// AutoMigrate runs GORM auto-migration for auth models
func (s *GormStorage) AutoMigrate() error {
	return s.db.AutoMigrate(&User{}, &Token{}, &APIKey{})
}

// ServerStorer interface - Load user by key (ID or email)
func (s *GormStorage) Load(ctx context.Context, key string) (authboss.User, error) {
	var user User

	// Check if key is email or ID
	query := s.db.WithContext(ctx)
	if strings.Contains(key, "@") {
		// It's an email
		query = query.Where("email = ?", key)
	} else {
		// Try as UUID first, fallback to email
		query = query.Where("id = ? OR email = ?", key, key)
	}

	if err := query.First(&user).Error; err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, authboss.ErrUserNotFound
		}
		return nil, err
	}

	return &user, nil
}

// Save updates an existing user
func (s *GormStorage) Save(ctx context.Context, user authboss.User) error {
	u := user.(*User)
	u.UpdatedAt = time.Now()

	// Update all fields except ID and CreatedAt
	return s.db.WithContext(ctx).Model(u).Select("*").Omit("id", "created_at").Updates(u).Error
}

// New creates a new user (for authboss use)
func (s *GormStorage) New(ctx context.Context) authboss.User {
	return &User{}
}

// Create inserts a new user
func (s *GormStorage) Create(ctx context.Context, user authboss.User) error {
	u := user.(*User)
	if u.ID == uuid.Nil {
		u.ID = uuid.New()
	}
	return s.db.WithContext(ctx).Create(u).Error
}

// ConfirmableStorer interface
func (s *GormStorage) LoadByConfirmSelector(ctx context.Context, selector string) (authboss.ConfirmableUser, error) {
	var user User
	err := s.db.WithContext(ctx).Where("confirm_selector = ?", selector).First(&user).Error
	if err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, authboss.ErrUserNotFound
		}
		return nil, err
	}
	return &user, nil
}

// RecoverableStorer interface
func (s *GormStorage) LoadByRecoverSelector(ctx context.Context, selector string) (authboss.RecoverableUser, error) {
	var user User
	err := s.db.WithContext(ctx).Where("recover_selector = ?", selector).First(&user).Error
	if err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, authboss.ErrUserNotFound
		}
		return nil, err
	}
	return &user, nil
}

// RememberingServerStorer interface
func (s *GormStorage) AddRememberToken(ctx context.Context, pid, token string) error {
	// For simplicity, store remember tokens in the Token table
	t := &Token{
		ID:        uuid.New(),
		UserID:    uuid.MustParse(pid),
		Token:     token,
		Type:      "remember",
		ExpiresAt: time.Now().Add(30 * 24 * time.Hour), // 30 days
		CreatedAt: time.Now(),
	}
	return s.db.WithContext(ctx).Create(t).Error
}

func (s *GormStorage) DelRememberTokens(ctx context.Context, pid string) error {
	return s.db.WithContext(ctx).Where("user_id = ? AND type = ?", pid, "remember").Delete(&Token{}).Error
}

func (s *GormStorage) UseRememberToken(ctx context.Context, pid, token string) error {
	// Mark token as used
	now := time.Now()
	return s.db.WithContext(ctx).Model(&Token{}).
		Where("user_id = ? AND token = ? AND type = ? AND expires_at > ?", pid, token, "remember", now).
		Update("used_at", now).Error
}

// FindUserByOAuthToken finds a user by their OAuth token (provider + provider UID)
func (s *GormStorage) FindUserByOAuthToken(ctx context.Context, provider, providerUID string) (*User, error) {
	var token Token
	err := s.db.WithContext(ctx).
		Where("type = ? AND provider = ? AND provider_uid = ? AND revoked_at IS NULL", TokenTypeOAuth, provider, providerUID).
		Order("created_at DESC").
		First(&token).Error
	if err != nil {
		return nil, err
	}

	var user User
	if err := s.db.WithContext(ctx).Where("id = ?", token.UserID).First(&user).Error; err != nil {
		return nil, err
	}
	return &user, nil
}

// CreateOrUpdateOAuthToken creates or updates an OAuth token for a user
func (s *GormStorage) CreateOrUpdateOAuthToken(ctx context.Context, userID uuid.UUID, provider, providerUID, accessToken string, expiry *time.Time) error {
	// Revoke any existing OAuth tokens for this provider/user combo
	now := time.Now()
	s.db.WithContext(ctx).Model(&Token{}).
		Where("user_id = ? AND type = ? AND provider = ? AND revoked_at IS NULL", userID, TokenTypeOAuth, provider).
		Update("revoked_at", now)

	// Create new OAuth token
	token := &Token{
		ID:          uuid.New(),
		UserID:      userID,
		Type:        TokenTypeOAuth,
		Provider:    &provider,
		ProviderUID: &providerUID,
		AccessToken: &accessToken,
		ExpiresAt:   now.Add(365 * 24 * time.Hour), // OAuth tokens don't expire in our system
		CreatedAt:   now,
	}
	if expiry != nil {
		token.OAuthExpiry = expiry
	}

	return s.db.WithContext(ctx).Create(token).Error
}

// Additional helper methods

// GetUserByID retrieves a user by ID
func (s *GormStorage) GetUserByID(ctx context.Context, id uuid.UUID) (*User, error) {
	var user User
	err := s.db.WithContext(ctx).Where("id = ?", id).First(&user).Error
	if err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, fmt.Errorf("user not found")
		}
		return nil, err
	}
	return &user, nil
}

// GetUserByEmail retrieves a user by email
func (s *GormStorage) GetUserByEmail(ctx context.Context, email string) (*User, error) {
	var user User
	err := s.db.WithContext(ctx).Where("email = ?", email).First(&user).Error
	if err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, fmt.Errorf("user not found")
		}
		return nil, err
	}
	return &user, nil
}

// ListUsers retrieves all users with optional filters
func (s *GormStorage) ListUsers(ctx context.Context, offset, limit int) ([]User, int64, error) {
	var users []User
	var total int64

	// Get total count
	if err := s.db.WithContext(ctx).Model(&User{}).Count(&total).Error; err != nil {
		return nil, 0, err
	}

	// Get paginated results
	err := s.db.WithContext(ctx).
		Offset(offset).
		Limit(limit).
		Order("created_at DESC").
		Find(&users).Error

	return users, total, err
}

// UpdateUser updates a user
func (s *GormStorage) UpdateUser(ctx context.Context, user *User) error {
	user.UpdatedAt = time.Now()
	return s.db.WithContext(ctx).Save(user).Error
}

// DeleteUser soft deletes a user
func (s *GormStorage) DeleteUser(ctx context.Context, id uuid.UUID) error {
	return s.db.WithContext(ctx).Delete(&User{}, id).Error
}


// CreateToken creates a new token
func (s *GormStorage) CreateToken(ctx context.Context, token *Token) error {
	return s.db.WithContext(ctx).Create(token).Error
}

// GetToken retrieves a token
func (s *GormStorage) GetToken(ctx context.Context, token string) (*Token, error) {
	var t Token
	err := s.db.WithContext(ctx).Where("token = ?", token).First(&t).Error
	if err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, fmt.Errorf("token not found")
		}
		return nil, err
	}
	return &t, nil
}

// UseToken marks a token as used
func (s *GormStorage) UseToken(ctx context.Context, token string) error {
	now := time.Now()
	return s.db.WithContext(ctx).Model(&Token{}).Where("token = ?", token).Update("used_at", now).Error
}

// DeleteExpiredTokens removes expired tokens
func (s *GormStorage) DeleteExpiredTokens(ctx context.Context) error {
	return s.db.WithContext(ctx).Where("expires_at < ?", time.Now()).Delete(&Token{}).Error
}

// API Key management methods

// CreateAPIKey creates a new API key for a user
func (s *GormStorage) CreateAPIKey(ctx context.Context, apiKey *APIKey) error {
	return s.db.WithContext(ctx).Create(apiKey).Error
}

// GetAPIKeyByHash retrieves an API key by its hash
func (s *GormStorage) GetAPIKeyByHash(ctx context.Context, keyHash string) (*APIKey, error) {
	var apiKey APIKey
	err := s.db.WithContext(ctx).
		Where("key_hash = ? AND revoked_at IS NULL", keyHash).
		First(&apiKey).Error
	if err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, fmt.Errorf("API key not found")
		}
		return nil, err
	}
	return &apiKey, nil
}

// GetAPIKeysByUserID retrieves all API keys for a user
func (s *GormStorage) GetAPIKeysByUserID(ctx context.Context, userID uuid.UUID) ([]APIKey, error) {
	var apiKeys []APIKey
	err := s.db.WithContext(ctx).
		Where("user_id = ? AND revoked_at IS NULL", userID).
		Order("created_at DESC").
		Find(&apiKeys).Error
	return apiKeys, err
}

// GetAPIKeyByID retrieves an API key by its ID
func (s *GormStorage) GetAPIKeyByID(ctx context.Context, id uuid.UUID) (*APIKey, error) {
	var apiKey APIKey
	err := s.db.WithContext(ctx).Where("id = ?", id).First(&apiKey).Error
	if err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, fmt.Errorf("API key not found")
		}
		return nil, err
	}
	return &apiKey, nil
}

// RevokeAPIKey revokes an API key
func (s *GormStorage) RevokeAPIKey(ctx context.Context, id uuid.UUID, userID uuid.UUID) error {
	now := time.Now()
	result := s.db.WithContext(ctx).
		Model(&APIKey{}).
		Where("id = ? AND user_id = ? AND revoked_at IS NULL", id, userID).
		Update("revoked_at", now)
	if result.RowsAffected == 0 {
		return fmt.Errorf("API key not found or already revoked")
	}
	return result.Error
}

// UpdateAPIKeyLastUsed updates the last used timestamp and IP
func (s *GormStorage) UpdateAPIKeyLastUsed(ctx context.Context, id uuid.UUID, ip string) error {
	now := time.Now()
	return s.db.WithContext(ctx).
		Model(&APIKey{}).
		Where("id = ?", id).
		Updates(map[string]interface{}{
			"last_used_at": now,
			"last_used_ip": ip,
		}).Error
}
