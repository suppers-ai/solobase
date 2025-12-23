//go:build wasm

package auth

import (
	"context"
	"database/sql"
	"errors"
	"net/http"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/mailer"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	"github.com/suppers-ai/solobase/pkg/interfaces"
)

// ErrUserNotFound is returned when a user is not found
var ErrUserNotFound = errors.New("user not found")

// Service provides authentication functionality (stub for WASM/TinyGo)
type Service struct {
	sqlDB   *sql.DB
	storage *Storage
	mailer  mailer.Mailer
	rootURL string
}

// Config holds authentication configuration
type Config struct {
	DB           interface{}
	Mailer       mailer.Mailer
	RootURL      string
	BCryptCost   int
	DatabaseType string
}

// New creates a new authentication service (stub for WASM/TinyGo)
func New(cfg Config) (*Service, error) {
	var sqlDB *sql.DB
	var storage *Storage

	switch d := cfg.DB.(type) {
	case *sql.DB:
		sqlDB = d
		storage = NewStorage(d)
	case interfaces.Database:
		sqlDB = d.GetDB() // May be nil in WASM
		if sqlDB != nil {
			storage = NewStorage(sqlDB)
		}
	}

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

// Middleware returns HTTP middleware for loading auth state
func (s *Service) Middleware(next http.Handler) http.Handler {
	return next
}

// RequireAuth middleware requires authentication
func (s *Service) RequireAuth(next http.Handler) http.Handler {
	return next
}

// User management methods

// CreateUser creates a new user
func (s *Service) CreateUser(ctx context.Context, email, password string) (*User, error) {
	return nil, errors.New("not implemented in WASM")
}

// GetUser retrieves a user by ID
func (s *Service) GetUser(ctx context.Context, id string) (*User, error) {
	if s.storage != nil {
		return s.storage.Load(ctx, id)
	}
	return nil, ErrUserNotFound
}

// GetUserByEmail retrieves a user by email
func (s *Service) GetUserByEmail(ctx context.Context, email string) (*User, error) {
	if s.storage != nil {
		return s.storage.GetUserByEmail(ctx, email)
	}
	return nil, ErrUserNotFound
}

// UpdateUser updates a user
func (s *Service) UpdateUser(ctx context.Context, user *User) error {
	return errors.New("not implemented in WASM")
}

// DeleteUser deletes a user
func (s *Service) DeleteUser(ctx context.Context, id string) error {
	return errors.New("not implemented in WASM")
}

// ListUsers lists all users
func (s *Service) ListUsers(ctx context.Context, offset, limit int) ([]User, int64, error) {
	return nil, 0, errors.New("not implemented in WASM")
}

// Token management

// CreateToken creates a new token
func (s *Service) CreateToken(ctx context.Context, userID string, tokenType string, duration apptime.Duration) (*Token, error) {
	return nil, errors.New("not implemented in WASM")
}

// ValidateToken validates and uses a token
func (s *Service) ValidateToken(ctx context.Context, tokenString string) (*Token, error) {
	return nil, errors.New("not implemented in WASM")
}

// CleanupTokens removes expired tokens
func (s *Service) CleanupTokens(ctx context.Context) error {
	return nil // No-op in WASM
}

// ScheduledCleanup starts background cleanup tasks
func (s *Service) ScheduledCleanup(ctx context.Context, interval apptime.Duration) {
	// No-op in WASM
}

// CreateDefaultAdmin creates the default admin user if it doesn't exist
func (s *Service) CreateDefaultAdmin(email, password string) error {
	return errors.New("not implemented in WASM")
}

// Storage provides user storage (stub for WASM/TinyGo)
type Storage struct {
	db *sql.DB
}

// NewStorage creates a new storage
func NewStorage(sqlDB *sql.DB) *Storage {
	return &Storage{db: sqlDB}
}

// Load loads a user by ID or email
func (s *Storage) Load(ctx context.Context, key string) (*User, error) {
	return nil, ErrUserNotFound
}

// Save saves a user
func (s *Storage) Save(ctx context.Context, user *User) error {
	return errors.New("not implemented in WASM")
}

// Create creates a new user
func (s *Storage) Create(ctx context.Context, user *User) error {
	return errors.New("not implemented in WASM")
}

// GetUserByID retrieves a user by ID
func (s *Storage) GetUserByID(ctx context.Context, id uuid.UUID) (*User, error) {
	return nil, ErrUserNotFound
}

// GetUserByEmail retrieves a user by email
func (s *Storage) GetUserByEmail(ctx context.Context, email string) (*User, error) {
	return nil, ErrUserNotFound
}

// ListUsers retrieves all users
func (s *Storage) ListUsers(ctx context.Context, offset, limit int) ([]User, int64, error) {
	return nil, 0, errors.New("not implemented in WASM")
}

// UpdateUser updates a user
func (s *Storage) UpdateUser(ctx context.Context, user *User) error {
	return errors.New("not implemented in WASM")
}

// DeleteUser soft deletes a user
func (s *Storage) DeleteUser(ctx context.Context, id uuid.UUID) error {
	return errors.New("not implemented in WASM")
}

// Token methods (stubs)

// CreateToken creates a new token
func (s *Storage) CreateToken(ctx context.Context, token *Token) error {
	return errors.New("not implemented in WASM")
}

// GetToken retrieves a token
func (s *Storage) GetToken(ctx context.Context, token string) (*Token, error) {
	return nil, errors.New("not implemented in WASM")
}

// UseToken marks a token as used
func (s *Storage) UseToken(ctx context.Context, token string) error {
	return errors.New("not implemented in WASM")
}

// DeleteExpiredTokens removes expired tokens
func (s *Storage) DeleteExpiredTokens(ctx context.Context) error {
	return nil // No-op
}

// API Key methods (stubs)

// CreateAPIKey creates a new API key
func (s *Storage) CreateAPIKey(ctx context.Context, apiKey *APIKey) error {
	return errors.New("not implemented in WASM")
}

// GetAPIKeyByHash retrieves an API key by hash
func (s *Storage) GetAPIKeyByHash(ctx context.Context, keyHash string) (*APIKey, error) {
	return nil, errors.New("not implemented in WASM")
}

// GetAPIKeysByUserID retrieves all API keys for a user
func (s *Storage) GetAPIKeysByUserID(ctx context.Context, userID uuid.UUID) ([]APIKey, error) {
	return nil, errors.New("not implemented in WASM")
}

// GetAPIKeyByID retrieves an API key by ID
func (s *Storage) GetAPIKeyByID(ctx context.Context, id uuid.UUID) (*APIKey, error) {
	return nil, errors.New("not implemented in WASM")
}

// RevokeAPIKey revokes an API key
func (s *Storage) RevokeAPIKey(ctx context.Context, id uuid.UUID, userID uuid.UUID) error {
	return errors.New("not implemented in WASM")
}

// UpdateAPIKeyLastUsed updates the last used timestamp
func (s *Storage) UpdateAPIKeyLastUsed(ctx context.Context, id uuid.UUID, ip string) error {
	return errors.New("not implemented in WASM")
}
