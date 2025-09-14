package auth

import (
	"context"
	"fmt"
	"net/http"
	"strings"
	"time"

	"github.com/gorilla/sessions"
	"github.com/suppers-ai/solobase/internal/pkg/database"
	"github.com/suppers-ai/solobase/internal/pkg/mailer"
	"github.com/volatiletech/authboss/v3"
	"github.com/volatiletech/authboss/v3/defaults"
	"gorm.io/gorm"
)

type AuthConfig struct {
	DB              interface{} // Can be database.Database or *gorm.DB
	SessionStore    sessions.Store
	Storage         authboss.ServerStorer
	Mailer          mailer.Mailer
	RootURL         string
	BCryptCost      int
	SessionName     string
	CookieKey       []byte
	SessionKey      []byte
	CSRFKey         []byte
	OAuth2Providers map[string]OAuth2Config
}

type OAuth2Config struct {
	ClientID     string
	ClientSecret string
	Scopes       []string
}

type Auth struct {
	AB           *authboss.Authboss
	SessionStore sessions.Store
	Storage      authboss.ServerStorer
}

func NewAuth(cfg AuthConfig) (*Auth, error) {
	if cfg.DB == nil && cfg.Storage == nil {
		return nil, fmt.Errorf("database connection or storage is required")
	}

	if len(cfg.SessionKey) == 0 {
		cfg.SessionKey = []byte("default-session-key-change-in-production")
	}

	if len(cfg.CookieKey) == 0 {
		cfg.CookieKey = []byte("default-cookie-key-change-in-production")
	}

	if cfg.BCryptCost == 0 {
		cfg.BCryptCost = 12
	}

	if cfg.SessionName == "" {
		cfg.SessionName = "auth"
	}

	ab := authboss.New()

	// Use provided storage or create default one
	var storer authboss.ServerStorer
	if cfg.Storage != nil {
		storer = cfg.Storage
	} else if gormDB, ok := cfg.DB.(*gorm.DB); ok {
		storer = NewGormStorage(gormDB)
	} else if db, ok := cfg.DB.(database.Database); ok {
		// If using the database package interface, get the underlying GORM
		if dbWithGorm, ok := db.(interface{ GetGORM() *gorm.DB }); ok {
			storer = NewGormStorage(dbWithGorm.GetGORM())
		} else {
			return nil, fmt.Errorf("database does not support GORM")
		}
	} else {
		return nil, fmt.Errorf("invalid database type for default storage")
	}

	// Create a session store for authboss
	var sessionStore sessions.Store
	if cfg.SessionStore != nil {
		sessionStore = cfg.SessionStore
	} else {
		// Use cookie store as default
		sessionStore = sessions.NewCookieStore(cfg.SessionKey)
	}

	ab.Config.Storage.Server = storer

	// Set session and cookie state storage using adapter
	sessionAdapter := NewSessionStoreAdapter(sessionStore, cfg.SessionName)
	ab.Config.Storage.SessionState = sessionAdapter
	ab.Config.Storage.CookieState = sessionAdapter

	ab.Config.Core.ViewRenderer = defaults.JSONRenderer{}

	ab.Config.Paths.Mount = "/auth"
	ab.Config.Paths.RootURL = cfg.RootURL
	ab.Config.Paths.NotAuthorized = "/auth/unauthorized"

	ab.Config.Modules.LogoutMethod = "POST"
	ab.Config.Modules.LockAfter = 3
	ab.Config.Modules.LockWindow = 5 * time.Minute
	ab.Config.Modules.LockDuration = 12 * time.Hour
	ab.Config.Modules.RecoverTokenDuration = 24 * time.Hour
	// Note: ConfirmTokenDuration is not available in authboss v3
	// Confirmation tokens use ExpireAfter setting instead
	ab.Config.Modules.ExpireAfter = 24 * time.Hour

	// BCrypt cost will be set by defaults.SetCore
	// Custom cost can be configured through environment or after initialization

	if cfg.Mailer != nil {
		ab.Config.Core.Mailer = NewAuthbossMailer(cfg.Mailer)
	}

	// Initialize OAuth2Providers map if we have providers configured
	if len(cfg.OAuth2Providers) > 0 {
		ab.Config.Modules.OAuth2Providers = make(map[string]authboss.OAuth2Provider)
		// TODO: Properly configure OAuth2 providers with OAuth2Config
		// For now, leave it empty until we implement proper OAuth2 configuration
	}

	// SetCore will set up defaults including the hasher if not already set
	defaults.SetCore(&ab.Config, false, false)

	if err := ab.Init(); err != nil {
		return nil, fmt.Errorf("failed to initialize authboss: %w", err)
	}

	return &Auth{
		AB:           ab,
		SessionStore: sessionStore,
		Storage:      storer,
	}, nil
}

func (a *Auth) LoadClientState(w http.ResponseWriter, r *http.Request) (*http.Request, error) {
	// LoadClientState modifies the request and returns it
	return a.AB.LoadClientState(w, r)
}

func (a *Auth) CurrentUser(w http.ResponseWriter, r *http.Request) (authboss.User, error) {
	return a.AB.CurrentUser(r)
}

func (a *Auth) CurrentUserID(w http.ResponseWriter, r *http.Request) (string, error) {
	return a.AB.CurrentUserID(r)
}

func (a *Auth) LoadCurrentUser(r **http.Request) (authboss.User, error) {
	return a.AB.LoadCurrentUser(r)
}

func (a *Auth) LoadCurrentUserID(r **http.Request) (string, error) {
	return a.AB.LoadCurrentUserID(r)
}

func (a *Auth) Middleware(h http.Handler) http.Handler {
	return a.AB.LoadClientStateMiddleware(h)
}

func (a *Auth) RequireAuth(h http.Handler) http.Handler {
	// Simple auth check using authboss CurrentUser
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Load current user - this modifies the request context
		user, err := a.AB.LoadCurrentUser(&r)
		if err != nil || user == nil {
			// Check if this is an API request
			if strings.HasPrefix(r.URL.Path, "/api/") ||
				r.Header.Get("Accept") == "application/json" ||
				r.Header.Get("X-Requested-With") == "XMLHttpRequest" {
				// Return 401 for API requests
				http.Error(w, "Unauthorized", http.StatusUnauthorized)
				return
			}
			// Redirect for web pages
			http.Redirect(w, r, "/auth/login?redir="+r.URL.Path, http.StatusFound)
			return
		}

		// User authenticated, proceed with the handler
		h.ServeHTTP(w, r)
	})
}

func (a *Auth) RequireNoAuth(h http.Handler) http.Handler {
	return authboss.Middleware(a.AB, true, false, false)(h)
}

func (a *Auth) MountRoutes(mux *http.ServeMux) {
	mux.Handle("/auth", http.StripPrefix("/auth", a.AB.Config.Core.Router))
}

func (a *Auth) CleanupSessions(ctx context.Context) error {
	// Session cleanup is handled by the storage layer if needed
	return nil
}
