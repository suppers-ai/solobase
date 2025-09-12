package services

import (
	"context"
	"errors"
	"github.com/google/uuid"
	auth "github.com/suppers-ai/auth"
	"github.com/suppers-ai/solobase/database"
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
	// Check if admin already exists
	var existingUser auth.User
	result := s.db.Where("email = ?", email).First(&existingUser)

	// Hash password
	hashedPassword, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
	if err != nil {
		log.Printf("Failed to hash password: %v", err)
		return err
	}

	if result.Error == nil {
		// User exists, update password
		log.Printf("Admin user exists, updating password for: %s", email)
		existingUser.Password = string(hashedPassword)
		existingUser.Confirmed = true
		if err := s.db.Save(&existingUser).Error; err != nil {
			log.Printf("Failed to update admin password: %v", err)
			return err
		}
		log.Printf("Successfully updated admin password for: %s", email)
		return nil
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
	return s.db.Model(&auth.User{}).Where("id = ?", userID).Update("password", hashedPassword).Error
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
