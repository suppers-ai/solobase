package services

import (
	"context"
	"database/sql"

	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	auth "github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/internal/pkg/logger"
)

// Service provides a unified interface for middleware to access individual services
type Service struct {
	auth     *AuthService
	user     *UserService
	database *DatabaseService
	logger   *DBLogger
	config   *Config
	sqlDB    *sql.DB
}

// Config represents the configuration needed by handlers
type Config struct {
	JWTSecret    string
	EnableSignup bool
}

// Session represents a user session (simplified for middleware compatibility)
type Session struct {
	ID     string
	UserID uuid.UUID
}

// SessionsInterface defines the interface for session management
type SessionsInterface interface {
	GetSession(ctx context.Context, sessionID string) (*Session, error)
}

// UsersInterface defines the interface for user management
type UsersInterface interface {
	GetUserByID(ctx context.Context, id uuid.UUID) (*auth.User, error)
}

// simpleSessionService provides a minimal session implementation
type simpleSessionService struct {
	sqlDB *sql.DB
}

// simpleUserService wraps the UserService for the interface
type simpleUserService struct {
	userService *UserService
}

// NewService creates a unified service wrapper
func NewService(auth *AuthService, user *UserService, database *DatabaseService, logger *DBLogger, config *Config, sqlDB *sql.DB) *Service {
	return &Service{
		auth:     auth,
		user:     user,
		database: database,
		logger:   logger,
		config:   config,
		sqlDB:    sqlDB,
	}
}

// SQLDB returns the SQL database connection
func (s *Service) SQLDB() *sql.DB {
	return s.sqlDB
}

// Logger returns the logger
func (s *Service) Logger() logger.Logger {
	return s.logger
}

// Sessions returns a session service interface
func (s *Service) Sessions() SessionsInterface {
	return &simpleSessionService{sqlDB: s.sqlDB}
}

// Users returns a user service interface
func (s *Service) Users() UsersInterface {
	return &simpleUserService{userService: s.user}
}

// Auth returns the auth service
func (s *Service) Auth() *AuthService {
	return s.auth
}

// Config returns the config
func (s *Service) Config() *Config {
	return s.config
}

// Database returns the database service
func (s *Service) Database() *DatabaseService {
	return s.database
}

// GetSession retrieves a session by ID (simplified implementation)
func (s *simpleSessionService) GetSession(ctx context.Context, sessionID string) (*Session, error) {
	// This is a simplified implementation - in a real app you'd have a sessions table
	// For now, we'll treat this as a JWT or simple session token
	// You might want to implement proper session storage later
	return nil, sql.ErrNoRows // Force fallback to other auth methods
}

// GetUserByID retrieves a user by UUID
func (s *simpleUserService) GetUserByID(ctx context.Context, id uuid.UUID) (*auth.User, error) {
	return s.userService.GetUserByID(id.String())
}
