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
	return s.db.AutoMigrate(&User{}, &Session{}, &Token{})
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

// OAuth2ServerStorer interface
func (s *GormStorage) NewFromOAuth2(ctx context.Context, provider string, details map[string]string) (authboss.OAuth2User, error) {
	user := &User{}

	// Set OAuth2 fields
	user.OAuth2Provider = &provider
	if uid, ok := details["uid"]; ok {
		user.OAuth2UID = &uid
	}
	if email, ok := details["email"]; ok {
		user.Email = email
	}
	if name, ok := details["name"]; ok {
		user.Username = name
	}

	// OAuth users are automatically confirmed
	user.Confirmed = true

	return user, nil
}

func (s *GormStorage) SaveOAuth2(ctx context.Context, user authboss.OAuth2User) error {
	u := user.(*User)

	// Check if user exists by OAuth2 UID and provider
	var existing User
	err := s.db.WithContext(ctx).Where("oauth2_uid = ? AND oauth2_provider = ?", u.OAuth2UID, u.OAuth2Provider).First(&existing).Error

	if err == nil {
		// Update existing user
		u.ID = existing.ID
		u.CreatedAt = existing.CreatedAt
		return s.Save(ctx, u)
	}

	// Check if user exists by email
	err = s.db.WithContext(ctx).Where("email = ?", u.Email).First(&existing).Error
	if err == nil {
		// Link OAuth to existing user
		u.ID = existing.ID
		u.CreatedAt = existing.CreatedAt
		return s.Save(ctx, u)
	}

	// Create new user
	return s.Create(ctx, u)
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

// CreateSession creates a new session
func (s *GormStorage) CreateSession(ctx context.Context, session *Session) error {
	return s.db.WithContext(ctx).Create(session).Error
}

// GetSession retrieves a session
func (s *GormStorage) GetSession(ctx context.Context, id string) (*Session, error) {
	var session Session
	err := s.db.WithContext(ctx).Where("id = ?", id).First(&session).Error
	if err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, fmt.Errorf("session not found")
		}
		return nil, err
	}
	return &session, nil
}

// DeleteSession deletes a session
func (s *GormStorage) DeleteSession(ctx context.Context, id string) error {
	return s.db.WithContext(ctx).Delete(&Session{}, "id = ?", id).Error
}

// DeleteExpiredSessions removes expired sessions
func (s *GormStorage) DeleteExpiredSessions(ctx context.Context) error {
	return s.db.WithContext(ctx).Where("expires_at < ?", time.Now()).Delete(&Session{}).Error
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
