package services

import (
	"context"
	"errors"
	"time"
	"github.com/google/uuid"
	auth "github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/internal/pkg/database"
	"golang.org/x/crypto/bcrypt"
	"log"
)

type AuthService struct {
	db *database.DB
}

func NewAuthService(db *database.DB) *AuthService {
	return &AuthService{db: db}
}

func (s *AuthService) AuthenticateUser(email, password string) (*auth.User, error) {
	var user auth.User
	if err := s.db.Where("email = ?", email).First(&user).Error; err != nil {
		log.Printf("User not found for email: %s, error: %v", email, err)
		return nil, errors.New("invalid credentials")
	}

	log.Printf("Found user: %s", user.Email)

	if err := bcrypt.CompareHashAndPassword([]byte(user.Password), []byte(password)); err != nil {
		log.Printf("Password comparison failed for user: %s, error: %v", email, err)
		return nil, errors.New("invalid credentials")
	}

	// Update last login time
	now := time.Now()
	user.LastLogin = &now
	if err := s.db.Model(&user).Update("last_login", now).Error; err != nil {
		// Log the error but don't fail the authentication
		log.Printf("Failed to update last_login for user %s: %v", email, err)
	}

	log.Printf("Authentication successful for user: %s", email)
	return &user, nil
}

func (s *AuthService) CreateUser(user *auth.User) error {
	user.ID = uuid.New()
	return s.db.Create(user).Error
}

func (s *AuthService) GetUserByID(id string) (*auth.User, error) {
	var user auth.User
	if err := s.db.Where("id = ?", id).First(&user).Error; err != nil {
		return nil, err
	}
	return &user, nil
}

func (s *AuthService) CreateDefaultAdmin(email, password string) error {
	log.Printf("CreateDefaultAdmin called with email: %s, password length: %d", email, len(password))

	// Check if ANY admin user already exists
	var adminCount int64
	s.db.Model(&auth.User{}).Count(&adminCount)

	if adminCount > 0 {
		// If any users exist, only update if this specific email exists
		var existingUser auth.User
		result := s.db.Where("email = ?", email).First(&existingUser)

		if result.Error == nil {
			// Hash password
			hashedPassword, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
			if err != nil {
				log.Printf("Failed to hash password: %v", err)
				return err
			}

			// User exists, update password using UpdateColumns to skip BeforeUpdate hook
			// (we're already hashing the password here)
			log.Printf("Admin user exists, updating password for: %s", email)
			if err := s.db.Model(&existingUser).UpdateColumns(map[string]interface{}{
				"password":  string(hashedPassword),
				"confirmed": true,
			}).Error; err != nil {
				log.Printf("Failed to update admin password: %v", err)
				return err
			}
			log.Printf("Successfully updated admin password for: %s", email)
			return nil
		} else {
			log.Printf("Users already exist in database, skipping creation of %s", email)
			return nil
		}
	}

	// Hash password for new user
	hashedPassword, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
	if err != nil {
		log.Printf("Failed to hash password: %v", err)
		return err
	}

	log.Printf("Creating default admin user: %s", email)

	// Create admin user
	admin := &auth.User{
		ID:        uuid.New(),
		Email:     email,
		Password:  string(hashedPassword),
		Confirmed: true,
	}

	if err := s.db.Create(admin).Error; err != nil {
		log.Printf("Failed to create admin user: %v", err)
		return err
	}

	log.Printf("Successfully created default admin user: %s", email)
	return nil
}

func (s *AuthService) UpdateUserPassword(userID, hashedPassword string) error {
	// Use UpdateColumn to skip BeforeUpdate hook since password is already hashed
	return s.db.Model(&auth.User{}).Where("id = ?", userID).UpdateColumn("password", hashedPassword).Error
}

// CreateUserWithContext creates a new user with context (for handlers)
func (s *AuthService) CreateUserWithContext(ctx context.Context, email, password string) (*auth.User, error) {
	hashedPassword, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
	if err != nil {
		return nil, err
	}

	user := &auth.User{
		ID:        uuid.New(),
		Email:     email,
		Password:  string(hashedPassword),
		Confirmed: true, // Auto-confirm for API signup
	}

	if err := s.db.Create(user).Error; err != nil {
		return nil, err
	}

	return user, nil
}

// FindUserByEmail finds a user by email address
func (s *AuthService) FindUserByEmail(email string) (*auth.User, error) {
	var user auth.User
	if err := s.db.Where("email = ?", email).First(&user).Error; err != nil {
		return nil, err
	}
	return &user, nil
}

// FindUserByOAuthToken finds a user by OAuth provider and provider UID via Token table
func (s *AuthService) FindUserByOAuthToken(provider, providerUID string) (*auth.User, error) {
	var token auth.Token
	if err := s.db.Where("type = ? AND provider = ? AND provider_uid = ? AND revoked_at IS NULL",
		auth.TokenTypeOAuth, provider, providerUID).
		Order("created_at DESC").
		First(&token).Error; err != nil {
		return nil, err
	}

	var user auth.User
	if err := s.db.Where("id = ?", token.UserID).First(&user).Error; err != nil {
		return nil, err
	}
	return &user, nil
}

// CreateOrUpdateOAuthToken creates or updates an OAuth token for a user
func (s *AuthService) CreateOrUpdateOAuthToken(userID uuid.UUID, provider, providerUID, accessToken string, expiry *time.Time) error {
	// Revoke any existing OAuth tokens for this provider/user combo
	s.db.Model(&auth.Token{}).
		Where("user_id = ? AND type = ? AND provider = ? AND revoked_at IS NULL", userID, auth.TokenTypeOAuth, provider).
		Update("revoked_at", time.Now())

	// Create new OAuth token
	token := &auth.Token{
		ID:          uuid.New(),
		UserID:      userID,
		Type:        auth.TokenTypeOAuth,
		Provider:    &provider,
		ProviderUID: &providerUID,
		AccessToken: &accessToken,
		ExpiresAt:   time.Now().Add(365 * 24 * time.Hour), // OAuth tokens don't expire in our system
		CreatedAt:   time.Now(),
	}
	if expiry != nil {
		token.OAuthExpiry = expiry
	}

	return s.db.Create(token).Error
}

// UpdateUser updates a user's information
func (s *AuthService) UpdateUser(user *auth.User) error {
	return s.db.Save(user).Error
}
