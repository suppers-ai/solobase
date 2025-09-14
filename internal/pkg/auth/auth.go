package auth

import (
	"context"
	"fmt"
	"net/http"
	"time"

	"github.com/gorilla/sessions"
	"github.com/volatiletech/authboss/v3"
	"gorm.io/gorm"

	"github.com/suppers-ai/solobase/internal/pkg/auth/middleware"
	"github.com/suppers-ai/solobase/internal/pkg/database"
	"github.com/suppers-ai/solobase/internal/pkg/mailer"
)

// Service provides authentication functionality
type Service struct {
	auth         *Auth
	db           *gorm.DB
	sessionStore sessions.Store
	storage      *GormStorage
}

// Config holds authentication configuration
type Config struct {
	DB              interface{} // Can be *gorm.DB or database.Database
	Mailer          mailer.Mailer
	RootURL         string
	BCryptCost      int
	SessionName     string
	CookieKey       []byte
	SessionKey      []byte
	CSRFKey         []byte
	DatabaseType    string // postgres, sqlite, mysql
	OAuth2Providers map[string]OAuth2Provider
}

// OAuth2Provider configuration
type OAuth2Provider struct {
	ClientID     string
	ClientSecret string
	Scopes       []string
}

// New creates a new authentication service
func New(cfg Config) (*Service, error) {
	// Get GORM database
	var gormDB *gorm.DB
	switch db := cfg.DB.(type) {
	case *gorm.DB:
		gormDB = db
	case database.Database:
		// If using the database package interface, get the underlying GORM
		if dbWithGorm, ok := db.(interface{ GetGORM() *gorm.DB }); ok {
			gormDB = dbWithGorm.GetGORM()
		} else {
			return nil, fmt.Errorf("database does not support GORM")
		}
	default:
		return nil, fmt.Errorf("unsupported database type: %T", cfg.DB)
	}

	// Create GORM storage
	storage := NewGormStorage(gormDB)

	// Run auto-migrations
	if err := storage.AutoMigrate(); err != nil {
		return nil, fmt.Errorf("failed to migrate auth tables: %w", err)
	}

	// Create auth configuration
	authCfg := AuthConfig{
		DB:              cfg.DB,
		Mailer:          cfg.Mailer,
		RootURL:         cfg.RootURL,
		BCryptCost:      cfg.BCryptCost,
		SessionName:     cfg.SessionName,
		CookieKey:       cfg.CookieKey,
		SessionKey:      cfg.SessionKey,
		CSRFKey:         cfg.CSRFKey,
		Storage:         storage, // Use GORM storage
		OAuth2Providers: make(map[string]OAuth2Config),
	}

	// Convert OAuth2 providers
	for name, provider := range cfg.OAuth2Providers {
		authCfg.OAuth2Providers[name] = OAuth2Config{
			ClientID:     provider.ClientID,
			ClientSecret: provider.ClientSecret,
			Scopes:       provider.Scopes,
		}
	}

	// Create auth instance
	auth, err := NewAuth(authCfg)
	if err != nil {
		return nil, err
	}

	return &Service{
		auth:         auth,
		db:           gormDB,
		sessionStore: auth.SessionStore,
		storage:      storage,
	}, nil
}

// Router returns the auth router
func (s *Service) Router() http.Handler {
	return s.auth.AB.Config.Core.Router
}

// LoadClientStateMiddleware loads the client state
func (s *Service) LoadClientStateMiddleware(next http.Handler) http.Handler {
	return s.auth.Middleware(next)
}

// RequireAuth requires authentication
func (s *Service) RequireAuth(next http.Handler) http.Handler {
	return s.auth.RequireAuth(next)
}

// RequireNoAuth requires no authentication
func (s *Service) RequireNoAuth(next http.Handler) http.Handler {
	return s.auth.RequireNoAuth(next)
}

// RequireAdmin requires admin role
func (s *Service) RequireAdmin(adminChecker func(authboss.User) bool) func(http.Handler) http.Handler {
	return middleware.RequireAdmin(s.auth.AB, adminChecker)
}

// RequireAdminSimple provides a simpler interface for admin checking
func (s *Service) RequireAdminSimple(adminChecker func(interface{}) bool) func(http.Handler) http.Handler {
	wrappedChecker := func(user authboss.User) bool {
		return adminChecker(user)
	}
	return s.RequireAdmin(wrappedChecker)
}

// CurrentUser gets the current user from request
func (s *Service) CurrentUser(r *http.Request) (authboss.User, error) {
	return s.auth.AB.CurrentUser(r)
}

// CurrentUserID gets the current user ID from request
func (s *Service) CurrentUserID(r *http.Request) (string, error) {
	user, err := s.CurrentUser(r)
	if err != nil {
		return "", err
	}
	if user == nil {
		return "", fmt.Errorf("no user logged in")
	}
	return user.GetPID(), nil
}

// Logout logs out the current user
func (s *Service) Logout(w http.ResponseWriter, r *http.Request) error {
	_, err := s.auth.AB.Events.FireAfter(authboss.EventLogout, w, r)
	return err
}

// Storage returns the GORM storage instance
func (s *Service) Storage() *GormStorage {
	return s.storage
}

// GetAuthboss returns the underlying authboss instance
func (s *Service) GetAuthboss() *authboss.Authboss {
	return s.auth.AB
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
	user, err := s.storage.Load(ctx, id)
	if err != nil {
		return nil, err
	}
	return user.(*User), nil
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

// Session management

// CreateSession creates a new session
func (s *Service) CreateSession(ctx context.Context, userID string, duration time.Duration) (*Session, error) {
	uid, err := ParseUUID(userID)
	if err != nil {
		return nil, err
	}

	session := &Session{
		ID:        GenerateToken(32),
		UserID:    uid,
		Token:     GenerateToken(64),
		ExpiresAt: time.Now().Add(duration),
		CreatedAt: time.Now(),
		UpdatedAt: time.Now(),
	}

	if err := s.storage.CreateSession(ctx, session); err != nil {
		return nil, err
	}

	return session, nil
}

// GetSession retrieves a session
func (s *Service) GetSession(ctx context.Context, id string) (*Session, error) {
	return s.storage.GetSession(ctx, id)
}

// DeleteSession deletes a session
func (s *Service) DeleteSession(ctx context.Context, id string) error {
	return s.storage.DeleteSession(ctx, id)
}

// CleanupSessions removes expired sessions
func (s *Service) CleanupSessions(ctx context.Context) error {
	return s.storage.DeleteExpiredSessions(ctx)
}

// Token management

// CreateToken creates a new token
func (s *Service) CreateToken(ctx context.Context, userID string, tokenType string, duration time.Duration) (*Token, error) {
	uid, err := ParseUUID(userID)
	if err != nil {
		return nil, err
	}

	token := &Token{
		UserID:    uid,
		Token:     GenerateToken(64),
		Type:      tokenType,
		ExpiresAt: time.Now().Add(duration),
		CreatedAt: time.Now(),
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

	if token.UsedAt != nil {
		return nil, fmt.Errorf("token already used")
	}

	if time.Now().After(token.ExpiresAt) {
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
