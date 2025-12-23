package services

import (
	"context"
	"errors"
	"log"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	auth "github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/internal/pkg/crypto"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type AuthService struct {
	users  repos.UserRepository
	tokens repos.TokenRepository
}

func NewAuthService(users repos.UserRepository, tokens repos.TokenRepository) *AuthService {
	return &AuthService{
		users:  users,
		tokens: tokens,
	}
}

func (s *AuthService) AuthenticateUser(email, password string) (*auth.User, error) {
	ctx := context.Background()

	user, err := s.users.GetByEmail(ctx, email)
	if err != nil {
		if err == repos.ErrNotFound {
			log.Printf("User not found for email: %s", email)
			return nil, errors.New("invalid credentials")
		}
		log.Printf("Error finding user: %v", err)
		return nil, errors.New("invalid credentials")
	}

	log.Printf("Found user: %s", user.Email)

	if err := crypto.ComparePassword(user.Password, password); err != nil {
		log.Printf("Password comparison failed for user: %s, error: %v", email, err)
		return nil, errors.New("invalid credentials")
	}

	// Update last login time
	now := apptime.NowTime()
	if err := s.users.UpdateLastLogin(ctx, user.ID.String(), now); err != nil {
		// Log the error but don't fail the authentication
		log.Printf("Failed to update last_login for user %s: %v", email, err)
	}

	log.Printf("Authentication successful for user: %s", email)
	return user, nil
}

func (s *AuthService) CreateUser(user *auth.User) error {
	ctx := context.Background()
	if user.ID == uuid.Nil {
		user.ID = uuid.New()
	}

	_, err := s.users.Create(ctx, user)
	return err
}

func (s *AuthService) GetUserByID(id string) (*auth.User, error) {
	ctx := context.Background()
	user, err := s.users.GetByID(ctx, id)
	if err != nil {
		if err == repos.ErrNotFound {
			return nil, errors.New("user not found")
		}
		return nil, err
	}
	return user, nil
}

func (s *AuthService) CreateDefaultAdmin(email, password string) error {
	ctx := context.Background()
	log.Printf("CreateDefaultAdmin called with email: %s, password length: %d", email, len(password))

	// Check if ANY admin user already exists
	count, err := s.users.Count(ctx)
	if err != nil {
		log.Printf("Error counting users: %v", err)
		return err
	}

	if count > 0 {
		// If any users exist, only update if this specific email exists
		existingUser, err := s.users.GetByEmail(ctx, email)

		if err == nil {
			// Hash password using adapter
			hashedPassword, err := crypto.HashPassword(password)
			if err != nil {
				log.Printf("Failed to hash password: %v", err)
				return err
			}

			// User exists, update password
			log.Printf("Admin user exists, updating password for: %s", email)
			err = s.users.UpdatePassword(ctx, existingUser.ID.String(), hashedPassword)
			if err != nil {
				log.Printf("Failed to update admin password: %v", err)
				return err
			}

			// Also confirm the user
			err = s.users.ClearConfirmToken(ctx, existingUser.ID.String())
			if err != nil {
				log.Printf("Failed to confirm admin: %v", err)
				return err
			}

			log.Printf("Successfully updated admin password for: %s", email)
			return nil
		} else if err != repos.ErrNotFound {
			log.Printf("Error checking for existing user: %v", err)
			return err
		} else {
			log.Printf("Users already exist in database, skipping creation of %s", email)
			return nil
		}
	}

	// Hash password for new user
	hashedPassword, err := crypto.HashPassword(password)
	if err != nil {
		log.Printf("Failed to hash password: %v", err)
		return err
	}

	log.Printf("Creating default admin user: %s", email)

	// Create admin user
	now := apptime.NowTime()
	user := &auth.User{
		ID:        uuid.New(),
		Email:     email,
		Password:  hashedPassword,
		Confirmed: true,
		CreatedAt: now,
		UpdatedAt: now,
	}

	_, err = s.users.Create(ctx, user)
	if err != nil {
		log.Printf("Failed to create admin user: %v", err)
		return err
	}

	log.Printf("Successfully created default admin user: %s", email)
	return nil
}

func (s *AuthService) UpdateUserPassword(userID, hashedPassword string) error {
	ctx := context.Background()
	return s.users.UpdatePassword(ctx, userID, hashedPassword)
}

// CreateUserWithContext creates a new user with context (for handlers)
func (s *AuthService) CreateUserWithContext(ctx context.Context, email, password string) (*auth.User, error) {
	hashedPassword, err := crypto.HashPassword(password)
	if err != nil {
		return nil, err
	}

	now := apptime.NowTime()
	user := &auth.User{
		ID:        uuid.New(),
		Email:     email,
		Password:  hashedPassword,
		Confirmed: true, // Auto-confirm for API signup
		CreatedAt: now,
		UpdatedAt: now,
	}

	return s.users.Create(ctx, user)
}

// FindUserByEmail finds a user by email address
func (s *AuthService) FindUserByEmail(email string) (*auth.User, error) {
	ctx := context.Background()
	return s.users.GetByEmail(ctx, email)
}

// FindUserByOAuthToken finds a user by OAuth provider and provider UID via Token table
func (s *AuthService) FindUserByOAuthToken(provider, providerUID string) (*auth.User, error) {
	ctx := context.Background()

	// Get token by provider UID
	token, err := s.tokens.GetByProviderUID(ctx, provider, providerUID)
	if err != nil {
		return nil, err
	}

	// Get user by ID
	return s.users.GetByID(ctx, token.UserID.String())
}

// CreateOrUpdateOAuthToken creates or updates an OAuth token for a user
func (s *AuthService) CreateOrUpdateOAuthToken(userID uuid.UUID, provider, providerUID, accessToken string, expiry *apptime.Time) error {
	ctx := context.Background()

	// Revoke any existing OAuth tokens for this provider/user combo
	err := s.tokens.RevokeByType(ctx, userID.String(), string(auth.TokenTypeOAuth))
	if err != nil {
		log.Printf("Warning: failed to revoke existing tokens: %v", err)
	}

	// Create new OAuth token
	now := apptime.NowTime()
	expiresAt := now.Add(365 * 24 * apptime.Hour) // OAuth tokens don't expire in our system

	var oauthExpiry apptime.NullTime
	if expiry != nil {
		oauthExpiry = apptime.NewNullTime(*expiry)
	}

	token := &auth.Token{
		ID:          uuid.New(),
		UserID:      userID,
		Type:        string(auth.TokenTypeOAuth),
		Provider:    &provider,
		ProviderUID: &providerUID,
		AccessToken: &accessToken,
		ExpiresAt:   expiresAt,
		OAuthExpiry: oauthExpiry,
		CreatedAt:   now,
	}

	_, err = s.tokens.Create(ctx, token)
	return err
}

// UpdateUser updates a user's information
func (s *AuthService) UpdateUser(user *auth.User) error {
	ctx := context.Background()
	return s.users.Update(ctx, user)
}
