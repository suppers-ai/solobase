//go:build !wasm

package auth

import (
	"context"
	"database/sql"
	"fmt"
	"net/http"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/mailer"
	"github.com/suppers-ai/solobase/pkg/interfaces"
)

// Service provides authentication functionality
type Service struct {
	sqlDB   *sql.DB
	storage *Storage
	mailer  mailer.Mailer
	rootURL string
}

// Config holds authentication configuration
type Config struct {
	DB           interface{} // Can be *sql.DB or interfaces.Database
	Mailer       mailer.Mailer
	RootURL      string
	BCryptCost   int
	DatabaseType string // postgres, sqlite, mysql
}

// New creates a new authentication service
func New(cfg Config) (*Service, error) {
	// Get SQL database
	var sqlDB *sql.DB
	switch db := cfg.DB.(type) {
	case *sql.DB:
		sqlDB = db
	case interfaces.Database:
		// Get the underlying SQL DB from the interface
		sqlDB = db.GetDB()
		if sqlDB == nil {
			return nil, fmt.Errorf("database does not provide SQL access (WASM mode?)")
		}
	default:
		return nil, fmt.Errorf("unsupported database type: %T", cfg.DB)
	}

	// Create sqlc storage
	storage := NewStorage(sqlDB)

	return &Service{
		sqlDB:   sqlDB,
		storage: storage,
		mailer:  cfg.Mailer,
		rootURL: cfg.RootURL,
	}, nil
}

// Storage returns the storage instance
func (s *Service) Storage() *Storage {
	return s.storage
}

// Middleware returns HTTP middleware for loading auth state from JWT
func (s *Service) Middleware(next http.Handler) http.Handler {
	// JWT auth is handled by the auth middleware in internal/api/middleware
	return next
}

// RequireAuth is a no-op since JWT auth is handled elsewhere
func (s *Service) RequireAuth(next http.Handler) http.Handler {
	return next
}

// User management methods

// CreateUser creates a new user
func (s *Service) CreateUser(ctx context.Context, email, password string) (*User, error) {
	user := &User{
		Email:     email,
		Confirmed: true, // Admin-created users are auto-confirmed
	}

	if err := user.SetPassword(password); err != nil {
		return nil, err
	}

	if err := s.storage.Create(ctx, user); err != nil {
		return nil, err
	}

	return user, nil
}

// GetUser retrieves a user by ID
func (s *Service) GetUser(ctx context.Context, id string) (*User, error) {
	return s.storage.Load(ctx, id)
}

// GetUserByEmail retrieves a user by email
func (s *Service) GetUserByEmail(ctx context.Context, email string) (*User, error) {
	return s.storage.GetUserByEmail(ctx, email)
}

// UpdateUser updates a user
func (s *Service) UpdateUser(ctx context.Context, user *User) error {
	return s.storage.UpdateUser(ctx, user)
}

// DeleteUser deletes a user
func (s *Service) DeleteUser(ctx context.Context, id string) error {
	uid, err := ParseUUID(id)
	if err != nil {
		return err
	}
	return s.storage.DeleteUser(ctx, uid)
}

// ListUsers lists all users
func (s *Service) ListUsers(ctx context.Context, offset, limit int) ([]User, int64, error) {
	return s.storage.ListUsers(ctx, offset, limit)
}

// Token management

// CreateToken creates a new token
func (s *Service) CreateToken(ctx context.Context, userID string, tokenType string, duration apptime.Duration) (*Token, error) {
	uid, err := ParseUUID(userID)
	if err != nil {
		return nil, err
	}

	tokenStr, err := GenerateToken(64)
	if err != nil {
		return nil, fmt.Errorf("failed to generate token: %w", err)
	}

	token := &Token{
		UserID:    uid,
		Token:     tokenStr,
		Type:      tokenType,
		ExpiresAt: apptime.NewTime(apptime.NowTime().Add(duration)),
		CreatedAt: apptime.NowTime(),
	}

	if err := s.storage.CreateToken(ctx, token); err != nil {
		return nil, err
	}

	return token, nil
}

// ValidateToken validates and uses a token
func (s *Service) ValidateToken(ctx context.Context, tokenString string) (*Token, error) {
	token, err := s.storage.GetToken(ctx, tokenString)
	if err != nil {
		return nil, err
	}

	if token.UsedAt.Valid {
		return nil, fmt.Errorf("token already used")
	}

	if apptime.NowTime().After(token.ExpiresAt) {
		return nil, fmt.Errorf("token expired")
	}

	if err := s.storage.UseToken(ctx, tokenString); err != nil {
		return nil, err
	}

	return token, nil
}

// CleanupTokens removes expired tokens
func (s *Service) CleanupTokens(ctx context.Context) error {
	return s.storage.DeleteExpiredTokens(ctx)
}

// ScheduledCleanup starts background cleanup tasks
func (s *Service) ScheduledCleanup(ctx context.Context, interval apptime.Duration) {
	go func() {
		ticker := apptime.NewTicker(interval)
		defer ticker.Stop()

		for {
			select {
			case <-ctx.Done():
				return
			case <-ticker.C:
				_ = s.CleanupTokens(ctx)
			}
		}
	}()
}

// CreateDefaultAdmin creates the default admin user if it doesn't exist
func (s *Service) CreateDefaultAdmin(email, password string) error {
	ctx := context.Background()

	// Check if user already exists
	existingUser, err := s.storage.GetUserByEmail(ctx, email)
	if err == nil && existingUser != nil {
		return nil // User already exists
	}

	// Create the admin user
	_, err = s.CreateUser(ctx, email, password)
	return err
}
