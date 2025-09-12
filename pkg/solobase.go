package solobase

import (
	"log"
	"net/http"

	"github.com/gorilla/mux"
	auth "github.com/suppers-ai/auth"
	"github.com/suppers-ai/logger"
	"github.com/suppers-ai/solobase/admin"
	"github.com/suppers-ai/solobase/api"
	"github.com/suppers-ai/solobase/config"
	"github.com/suppers-ai/solobase/database"
	"github.com/suppers-ai/solobase/extensions"
	"github.com/suppers-ai/solobase/models"
	"github.com/suppers-ai/solobase/services"
	storage "github.com/suppers-ai/storage"
)

// App represents the Solobase application
type App struct {
	Router   *mux.Router
	DB       *database.DB
	Config   *config.Config
	Services *services.Services
}

// Options for creating a new Solobase app
type Options struct {
	DatabaseType         string
	DatabaseURL          string
	StorageType          string
	StoragePath          string
	DefaultAdminEmail    string
	DefaultAdminPassword string
	JWTSecret            string
}

// New creates a new Solobase application instance
func New(opts *Options) (*App, error) {
	// Set defaults
	if opts.DatabaseType == "" {
		opts.DatabaseType = "sqlite"
	}
	if opts.DatabaseURL == "" {
		opts.DatabaseURL = "file:./.data/solobase.db"
	}
	if opts.StorageType == "" {
		opts.StorageType = "local"
	}
	if opts.StoragePath == "" {
		opts.StoragePath = "./.data/storage"
	}
	if opts.JWTSecret == "" {
		opts.JWTSecret = "your-secret-key-change-in-production"
	}

	// Create config from options
	cfg := &config.Config{
		Database: config.DatabaseConfig{
			Type: opts.DatabaseType,
			URL:  opts.DatabaseURL,
		},
		Storage: config.StorageConfig{
			Type:      opts.StorageType,
			LocalPath: opts.StoragePath,
		},
		DefaultAdmin: config.AdminConfig{
			Email:    opts.DefaultAdminEmail,
			Password: opts.DefaultAdminPassword,
		},
		JWTSecret: opts.JWTSecret,
	}

	// Set JWT secret for API
	api.SetJWTSecret(cfg.JWTSecret)

	// Initialize database
	log.Printf("Initializing Solobase database with type: %s", cfg.Database.Type)
	db, err := database.New(cfg.Database)
	if err != nil {
		return nil, err
	}

	// Run migrations
	if err := db.Migrate(); err != nil {
		db.Close()
		return nil, err
	}

	// Initialize services
	earlyDbLogger := services.NewDBLogger(db)

	// Initialize logger
	logger.Init(&logger.Config{
		Env:      "development",
		LogLevel: logger.DebugLevel,
		DB:       earlyDbLogger,
	})

	// Initialize storage
	storageProvider, err := storage.New(storage.Config{
		Type:      cfg.Storage.Type,
		LocalPath: cfg.Storage.LocalPath,
		S3: storage.S3Config{
			Bucket:          cfg.Storage.S3Bucket,
			Region:          cfg.Storage.S3Region,
			AccessKeyID:     cfg.Storage.S3AccessKey,
			SecretAccessKey: cfg.Storage.S3SecretKey,
			Endpoint:        cfg.Storage.S3Endpoint,
			UsePathStyle:    cfg.Storage.S3UsePathStyle,
		},
	})
	if err != nil {
		db.Close()
		return nil, err
	}

	// Initialize auth
	authProvider := auth.New(&auth.Config{
		JWTSecret: cfg.JWTSecret,
		DB:        db.DB,
	})

	// Initialize services
	svcs := &services.Services{
		DB:      db,
		Storage: storageProvider,
		Auth:    authProvider,
		Logger:  earlyDbLogger,
	}

	// Check and create default admin
	if cfg.DefaultAdmin.Email != "" && cfg.DefaultAdmin.Password != "" {
		if err := checkAndCreateDefaultAdmin(db, cfg.DefaultAdmin.Email, cfg.DefaultAdmin.Password); err != nil {
			log.Printf("Warning: Failed to create default admin: %v", err)
		}
	}

	// Set up router
	router := mux.NewRouter()

	// Set up API routes
	api.SetupRoutes(router, svcs)

	// Initialize extensions
	extensionManager := extensions.NewManager(db, storageProvider, authProvider)
	if err := extensionManager.LoadAll(); err != nil {
		log.Printf("Warning: Failed to load some extensions: %v", err)
	}
	extensionManager.SetupRoutes(router)

	// Set up admin routes (should be last to catch all)
	admin.SetupRoutes(router, svcs, extensionManager)

	return &App{
		Router:   router,
		DB:       db,
		Config:   cfg,
		Services: svcs,
	}, nil
}

// Handler returns the HTTP handler for the application
func (app *App) Handler() http.Handler {
	return app.Router
}

// ServeHTTP implements http.Handler interface
func (app *App) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	app.Router.ServeHTTP(w, r)
}

// Close cleanly shuts down the application
func (app *App) Close() error {
	if app.DB != nil {
		return app.DB.Close()
	}
	return nil
}

func checkAndCreateDefaultAdmin(db *database.DB, email, password string) error {
	// Check if any admin users exist
	var count int64
	if err := db.DB.Model(&models.User{}).Where("role = ?", "admin").Count(&count).Error; err != nil {
		return err
	}

	if count > 0 {
		log.Println("Admin user already exists, skipping default admin creation")
		return nil
	}

	// Create default admin
	user := &models.User{
		Email:     email,
		Password:  password, // Will be hashed by BeforeCreate hook
		Role:      "admin",
		Confirmed: true,
	}

	if err := db.DB.Create(user).Error; err != nil {
		return err
	}

	log.Printf("Created default admin user: %s", email)
	return nil
}
