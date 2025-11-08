package solobase

import (
	"context"
	"database/sql"
	"embed"
	"fmt"
	"io/fs"
	"log"
	"net/http"
	"os"
	"os/signal"
	"path/filepath"
	"strings"
	"syscall"
	"time"

	"github.com/gorilla/mux"
	"gorm.io/gorm"
	auth "github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/internal/pkg/logger"
	"github.com/suppers-ai/solobase/internal/pkg/database"
	"github.com/suppers-ai/solobase/extensions"
	authHandlers "github.com/suppers-ai/solobase/internal/api/handlers/auth"
	"github.com/suppers-ai/solobase/internal/api/middleware"
	"github.com/suppers-ai/solobase/internal/api/router"
	"github.com/suppers-ai/solobase/internal/config"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/internal/data/models"
	"github.com/suppers-ai/solobase/internal/iam"
	storage "github.com/suppers-ai/solobase/internal/pkg/storage"
)

// App represents the Solobase application
type App struct {
	router           *mux.Router
	db               *database.DB
	config           *config.Config
	sqlDB            *sql.DB // Store sql.DB reference for Close()
	appID            string // Application ID for storage isolation
	services         *AppServices
	extensionManager *extensions.ExtensionManager
	server           *http.Server
	productsSeeder   interface{} // Custom seeder for Products extension

	// Event hooks
	onServeHooks     []func(*ServeEvent) error
	onBeforeAPIHooks []func(*APIEvent) error
	onAfterAPIHooks  []func(*APIEvent) error
	onModelHooks     map[string][]func(*ModelEvent) error
}

// AppServices contains all the services used by the app
type AppServices struct {
	Auth       *services.AuthService
	User       *services.UserService
	Storage    *services.StorageService
	Database   *services.DatabaseService
	Settings   *services.SettingsService
	Logs       *services.LogsService
	Logger     *services.DBLogger
	IAM        *iam.Service
}

// ServeEvent is passed to OnServe hooks
type ServeEvent struct {
	App    *App
	Router *mux.Router
	Next   func() error
}

// APIEvent is passed to API hooks
type APIEvent struct {
	App      *App
	Request  *http.Request
	Response http.ResponseWriter
	Next     func() error
}

// ModelEvent is passed to model hooks
type ModelEvent struct {
	App   *App
	Model interface{}
	Next  func() error
}

// Options for creating a new Solobase app
type Options struct {
	DatabaseType         string
	DatabaseURL          string
	StorageType          string
	AppID                string // Application ID for storage isolation (defaults to "solobase")
	S3Config             *S3Config
	DefaultAdminEmail    string
	DefaultAdminPassword string
	JWTSecret            string
	Port                 string
	DisableUI            bool
	DisableHome          bool        // Disable the root "/" handler while keeping other UI routes
	ProductsSeeder       interface{} // Custom seeder for Products extension
}

// S3Config for S3 storage
type S3Config struct {
	Bucket          string
	Region          string
	AccessKeyID     string
	SecretAccessKey string
	Endpoint        string
	UsePathStyle    bool
}

//go:embed all:frontend/build/*
var uiFiles embed.FS


// New creates a new Solobase application instance
func New() *App {
	return NewWithOptions(&Options{})
}

// NewWithOptions creates a new Solobase app with custom options
func NewWithOptions(opts *Options) *App {
	// Set defaults
	if opts.DatabaseType == "" {
		opts.DatabaseType = os.Getenv("DATABASE_TYPE")
		if opts.DatabaseType == "" {
			opts.DatabaseType = "sqlite"
		}
	}
	if opts.DatabaseURL == "" {
		opts.DatabaseURL = os.Getenv("DATABASE_URL")
		if opts.DatabaseURL == "" {
			opts.DatabaseURL = "file:./.data/solobase.db"
		}
	}
	if opts.StorageType == "" {
		opts.StorageType = os.Getenv("STORAGE_TYPE")
		if opts.StorageType == "" {
			opts.StorageType = "local"
		}
	}
	// Remove StoragePath as we're using AppID instead
	// Storage path will be determined based on AppID
	if opts.AppID == "" {
		opts.AppID = os.Getenv("APP_ID")
		if opts.AppID == "" {
			opts.AppID = "solobase"
		}
	}
	if opts.JWTSecret == "" {
		opts.JWTSecret = os.Getenv("JWT_SECRET")
		if opts.JWTSecret == "" {
			opts.JWTSecret = "your-secret-key-change-in-production"
		}
	}
	if opts.Port == "" {
		opts.Port = os.Getenv("PORT")
		if opts.Port == "" {
			opts.Port = "8090"
		}
	}
	if opts.DefaultAdminEmail == "" {
		opts.DefaultAdminEmail = os.Getenv("DEFAULT_ADMIN_EMAIL")
		log.Printf("DEBUG: DEFAULT_ADMIN_EMAIL from env: '%s'", opts.DefaultAdminEmail)
		// Don't set a default if the env var is not set
		// This prevents creating unintended admin accounts
	} else {
		log.Printf("DEBUG: DefaultAdminEmail already set to: '%s'", opts.DefaultAdminEmail)
	}
	if opts.DefaultAdminPassword == "" {
		opts.DefaultAdminPassword = os.Getenv("DEFAULT_ADMIN_PASSWORD")
		// Don't set a default password either
	}

	app := &App{
		appID:          opts.AppID,
		productsSeeder: opts.ProductsSeeder,
		onModelHooks:   make(map[string][]func(*ModelEvent) error),
	}

	// Create config
	app.config = &config.Config{
		Port:        opts.Port,
		Environment: os.Getenv("ENVIRONMENT"),
		Database: &database.Config{
			Type: opts.DatabaseType,
			// Parse DATABASE_URL based on type
		},
		Storage: config.StorageConfig{
			Type:             opts.StorageType,
			LocalStoragePath: "./.data/storage", // Default path, AppID will be used for organization
		},
		JWTSecret:     opts.JWTSecret,
		AdminEmail:    opts.DefaultAdminEmail,
		AdminPassword: opts.DefaultAdminPassword,
		DisableUI:     opts.DisableUI,
		DisableHome:   opts.DisableHome,
	}

	// Set S3 config if provided
	if opts.S3Config != nil {
		app.config.Storage.S3Bucket = opts.S3Config.Bucket
		app.config.Storage.S3Region = opts.S3Config.Region
		app.config.Storage.S3AccessKey = opts.S3Config.AccessKeyID
		app.config.Storage.S3SecretKey = opts.S3Config.SecretAccessKey
		app.config.Storage.S3Endpoint = opts.S3Config.Endpoint
	}

	// Parse database URL
	if opts.DatabaseType == "postgres" {
		// Parse PostgreSQL URL
		app.config.Database = parsePostgresURL(opts.DatabaseURL)
	} else {
		// SQLite
		dbURL := opts.DatabaseURL

		// Add read-only mode if READONLY_MODE is enabled
		if os.Getenv("READONLY_MODE") == "true" {
			// Check if URL already has query parameters
			if strings.Contains(dbURL, "?") {
				dbURL += "&mode=ro"
			} else {
				dbURL += "?mode=ro"
			}
			log.Printf("Read-only mode enabled - database will be opened in read-only mode")
		}

		app.config.Database = &database.Config{
			Type:     "sqlite",
			Database: dbURL,
		}
	}

	return app
}

// Initialize initializes the app (database, services, etc)
func (app *App) Initialize() error {
	// Set JWT secret in both middleware and auth handlers
	if err := middleware.SetJWTSecret(app.config.JWTSecret); err != nil {
		return fmt.Errorf("failed to set JWT secret: %w", err)
	}
	if err := authHandlers.SetJWTSecret(app.config.JWTSecret); err != nil {
		return fmt.Errorf("failed to set JWT secret in auth handlers: %w", err)
	}

	// Ensure .data directory exists for SQLite databases
	// We need to get the database URL from the parsed config or from NewWithOptions
	// The database URL is set up during New/NewWithOptions
	dbURL := os.Getenv("DATABASE_URL")
	if dbURL == "" {
		dbURL = "file:./.data/solobase.db"
	}

	if app.config.Database.Type == "sqlite" && dbURL != "" {
		// Extract directory from database path (e.g., file:./.data/solobase.db -> ./.data)
		dbPath := dbURL
		if strings.HasPrefix(dbPath, "file:") {
			dbPath = strings.TrimPrefix(dbPath, "file:")
		}
		if dir := filepath.Dir(dbPath); dir != "" && dir != "." {
			if err := os.MkdirAll(dir, 0755); err != nil {
				return fmt.Errorf("failed to create database directory: %w", err)
			}
		}
	}

	// Initialize database
	log.Printf("Initializing database with type: %s", app.config.Database.Type)
	db, err := database.New(app.config.Database)
	if err != nil {
		return fmt.Errorf("failed to connect to database: %w", err)
	}
	app.db = db

	// Get underlying sql.DB for connection management
	sqlDB, err := db.DB.DB()
	if err != nil {
		return fmt.Errorf("failed to get sql.DB: %w", err)
	}
	app.sqlDB = sqlDB

	// Skip migrations and auto-migrate in read-only mode
	if os.Getenv("READONLY_MODE") == "true" {
		log.Printf("Read-only mode: Skipping database migrations and table creation")
	} else {
		// Run migrations (handled by AutoMigrate below)

		// Auto-migrate models
		db.AutoMigrate(
			&auth.User{},
			&models.Setting{},
			&models.StorageDownloadToken{},
			&models.StorageUploadToken{},
			&storage.StorageObject{},
			&storage.StorageBucket{},
			&logger.LogModel{},
			&logger.RequestLogModel{},
			// IAM models
			&iam.Role{},
			&iam.UserRole{},
			&iam.IAMAuditLog{},
			// Custom Tables models
			&models.CustomTableDefinition{},
			&models.CustomTableMigration{},
		)
	}

	// Initialize database logger
	dbLogger := services.NewDBLogger(db)

	// Setup database metrics callbacks
	db.DB.Callback().Query().Before("gorm:query").Register("metrics:before_query", beforeQuery)
	db.DB.Callback().Query().After("gorm:query").Register("metrics:after_query", afterQuery)
	db.DB.Callback().Create().Before("gorm:create").Register("metrics:before_create", beforeQuery)
	db.DB.Callback().Create().After("gorm:create").Register("metrics:after_query", afterQuery)
	db.DB.Callback().Update().Before("gorm:update").Register("metrics:before_update", beforeQuery)
	db.DB.Callback().Update().After("gorm:update").Register("metrics:after_query", afterQuery)
	db.DB.Callback().Delete().Before("gorm:delete").Register("metrics:before_delete", beforeQuery)
	db.DB.Callback().Delete().After("gorm:delete").Register("metrics:after_query", afterQuery)

	// Initialize IAM service with Casbin
	iamService, err := iam.NewService(db.DB)
	if err != nil {
		return fmt.Errorf("failed to initialize IAM service: %w", err)
	}

	// Initialize services
	app.services = &AppServices{
		Auth: services.NewAuthService(db),
		User: services.NewUserService(db),
		Storage: services.NewStorageServiceWithOptions(db, app.config.Storage, &services.StorageOptions{
			AppID: app.appID,
		}),
		Database:   services.NewDatabaseService(db),
		Settings:   services.NewSettingsService(db),
		Logs:       services.NewLogsService(db),
		Logger:     dbLogger,
		IAM:        iamService,
	}

	// Create default admin (skip in read-only mode)
	if os.Getenv("READONLY_MODE") == "true" {
		log.Printf("Read-only mode: Skipping default admin creation")
	} else if app.config.AdminEmail != "" && app.config.AdminPassword != "" {
		log.Printf("Creating default admin with email: %s", app.config.AdminEmail)
		if err := app.services.Auth.CreateDefaultAdmin(app.config.AdminEmail, app.config.AdminPassword); err != nil {
			log.Printf("Warning: Failed to create default admin: %v", err)
		} else {
			// Assign admin role in IAM to the default admin user
			var adminUser auth.User
			if err := db.DB.Where("email = ?", app.config.AdminEmail).First(&adminUser).Error; err == nil {
				if err := iamService.AssignRoleToUser(context.Background(), adminUser.ID.String(), "admin"); err != nil {
					log.Printf("Warning: Failed to assign admin role to default admin: %v", err)
				}
			}
		}
	}

	// Initialize extension system
	extensionManager, err := extensions.NewExtensionManagerWithOptions(db.DB, dbLogger, app.productsSeeder)
	if err != nil {
		return fmt.Errorf("failed to create extension manager: %w", err)
	}
	app.extensionManager = extensionManager

	// Initialize extensions
	ctx := context.Background()
	if err := extensionManager.Initialize(ctx); err != nil {
		log.Printf("Warning: Failed to initialize some extensions: %v", err)
	}

	return nil
}

// OnServe adds a hook that runs when the server starts
func (app *App) OnServe() *ServeHook {
	return &ServeHook{app: app}
}

// ServeHook allows binding functions to the serve event
type ServeHook struct {
	app *App
}

// BindFunc binds a function to the serve event
func (h *ServeHook) BindFunc(fn func(*ServeEvent) error) *ServeHook {
	h.app.onServeHooks = append(h.app.onServeHooks, fn)
	return h
}

// OnBeforeAPI adds a hook that runs before API requests
func (app *App) OnBeforeAPI() *APIHook {
	return &APIHook{app: app, hooks: &app.onBeforeAPIHooks}
}

// OnAfterAPI adds a hook that runs after API requests
func (app *App) OnAfterAPI() *APIHook {
	return &APIHook{app: app, hooks: &app.onAfterAPIHooks}
}

// APIHook allows binding functions to API events
type APIHook struct {
	app   *App
	hooks *[]func(*APIEvent) error
}

// BindFunc binds a function to the API event
func (h *APIHook) BindFunc(fn func(*APIEvent) error) *APIHook {
	*h.hooks = append(*h.hooks, fn)
	return h
}

// OnModel adds hooks for model events
func (app *App) OnModel(modelName string) *ModelHook {
	return &ModelHook{app: app, modelName: modelName}
}

// ModelHook allows binding functions to model events
type ModelHook struct {
	app       *App
	modelName string
}

// BindFunc binds a function to the model event
func (h *ModelHook) BindFunc(fn func(*ModelEvent) error) *ModelHook {
	h.app.onModelHooks[h.modelName] = append(h.app.onModelHooks[h.modelName], fn)
	return h
}

// Start initializes and starts the server
func (app *App) Start() error {
	// Initialize if not already done
	if app.db == nil {
		if err := app.Initialize(); err != nil {
			return err
		}
	}

	// Setup router
	app.router = mux.NewRouter()

	// Apply middleware
	app.router.Use(services.HTTPLoggingMiddleware(app.services.Logger))
	// TODO: Setup Prometheus middleware
	// app.router.Use(router.PrometheusMiddleware)

	// Apply IAM middleware for authorization
	iamMiddleware := iam.NewMiddleware(app.services.IAM)
	app.router.Use(iamMiddleware.EnforceQuota())
	app.router.Use(iamMiddleware.RateLimit())

	// Apply extension middleware
	app.router.Use(func(next http.Handler) http.Handler {
		return app.extensionManager.ApplyMiddleware(next)
	})

	// Setup API router
	apiRouter := router.NewAPI(
		app.db,
		app.services.Auth,
		app.services.User,
		app.services.Storage,
		app.services.Database,
		app.services.Settings,
		app.services.Logs,
		app.services.IAM,
		app.extensionManager.GetRegistry(),
	)

	// IMPORTANT: Register more specific routes first

	// Register IAM routes
	iamHandlers := iam.NewHandlers(app.services.IAM)
	iamHandlers.RegisterRoutes(app.router)

	// Extension routes - register on apiRouter so they're under /api/ext/
	log.Println("DEBUG: About to register extension routes")
	if app.extensionManager != nil {
		log.Println("Registering extension routes...")
		app.extensionManager.RegisterRoutes(apiRouter.Router)
	} else {
		log.Println("WARNING: Extension manager is nil, cannot register extension routes")
	}

	// API routes
	log.Println("DEBUG: Setting up API routes")
	app.router.PathPrefix("/api").Handler(http.StripPrefix("/api", apiRouter))

	// Storage files
	storageDir := "./.data/storage/"
	app.router.PathPrefix("/storage/").Handler(http.StripPrefix("/storage/", http.FileServer(http.Dir(storageDir))))


	// Admin UI routes (if not disabled) - These are catch-all routes so must come LAST
	if !app.config.DisableUI {
		// Serve static assets at root (logo.png, logo_long.png, etc)
		app.router.HandleFunc("/logo.png", app.ServeStaticAsset("logo.png"))
		app.router.HandleFunc("/logo_long.png", app.ServeStaticAsset("logo_long.png"))
		app.router.HandleFunc("/favicon.ico", app.ServeStaticAsset("favicon.ico"))

		// Serve UI assets - MUST come before page routes
		app.router.PathPrefix("/_app/").Handler(app.ServeUI())

		// Serve auth pages at root level
		app.router.PathPrefix("/auth/").Handler(app.ServeUI())
		app.router.PathPrefix("/profile").Handler(app.ServeUI())

		// Keep UI pages under /ui
		app.router.PathPrefix("/ui/").Handler(app.ServeUI())

		// Serve root last as catch-all for the main dashboard
		// Note: This must come after ALL other routes
		// Only register the root handler if DisableHome is false
		if !app.config.DisableHome {
			app.router.PathPrefix("/").Handler(app.ServeUI())
		}
		// When DisableHome is true, don't register "/" - let the embedding app handle it
	}

	// Run OnServe hooks
	serveEvent := &ServeEvent{
		App:    app,
		Router: app.router,
		Next:   func() error { return nil },
	}

	for _, hook := range app.onServeHooks {
		if err := hook(serveEvent); err != nil {
			return fmt.Errorf("OnServe hook failed: %w", err)
		}
	}

	// Create HTTP server
	app.server = &http.Server{
		Addr:    ":" + app.config.Port,
		Handler: app.router,
	}

	// Setup graceful shutdown
	go app.handleShutdown()

	// Start server
	log.Printf("🚀 Solobase server starting on port %s", app.config.Port)
	if err := app.server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		return fmt.Errorf("server failed to start: %w", err)
	}

	return nil
}

// Router returns the underlying router
func (app *App) Router() *mux.Router {
	return app.router
}

// DB returns the database connection
func (app *App) DB() *database.DB {
	return app.db
}

// Services returns the app services
func (app *App) Services() *AppServices {
	return app.services
}

// Config returns the app config
func (app *App) Config() *config.Config {
	return app.config
}

// GetAppID returns the application ID for storage isolation
func (app *App) GetAppID() string {
	return app.appID
}

// ServeStaticAsset returns a handler for serving a specific static asset
func (app *App) ServeStaticAsset(assetName string) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Try embedded files
		uiFS, err := fs.Sub(uiFiles, "frontend/build")
		if err != nil {
			http.NotFound(w, r)
			return
		}

		// Read the specific asset
		data, err := fs.ReadFile(uiFS, assetName)
		if err != nil {
			http.NotFound(w, r)
			return
		}

		// Set appropriate content type
		contentType := "application/octet-stream"
		switch {
		case strings.HasSuffix(assetName, ".png"):
			contentType = "image/png"
		case strings.HasSuffix(assetName, ".jpg"), strings.HasSuffix(assetName, ".jpeg"):
			contentType = "image/jpeg"
		case strings.HasSuffix(assetName, ".svg"):
			contentType = "image/svg+xml"
		case strings.HasSuffix(assetName, ".ico"):
			contentType = "image/x-icon"
		case strings.HasSuffix(assetName, ".css"):
			contentType = "text/css"
		case strings.HasSuffix(assetName, ".js"):
			contentType = "application/javascript"
		case strings.HasSuffix(assetName, ".html"):
			contentType = "text/html"
		}

		w.Header().Set("Content-Type", contentType)
		w.Header().Set("Cache-Control", "public, max-age=3600")
		w.Write(data)
	}
}

// ServeUI returns the UI handler
func (app *App) ServeUI() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Try embedded files first
		uiFS, err := fs.Sub(uiFiles, "frontend/build")
		if err != nil {
			http.Error(w, "Admin interface not available", http.StatusNotFound)
			return
		}

		// For SPA routing, always serve index.html for non-asset paths
		path := r.URL.Path

		// Check if it's an asset request (has file extension)
		if strings.Contains(path, ".") {
			// Serve the actual file
			http.FileServer(http.FS(uiFS)).ServeHTTP(w, r)
		} else {
			// Serve index.html for all routes (SPA routing)
			indexData, err := fs.ReadFile(uiFS, "index.html")
			if err != nil {
				http.Error(w, "Admin interface not available", http.StatusNotFound)
				return
			}
			w.Header().Set("Content-Type", "text/html; charset=utf-8")
			w.Write(indexData)
		}
	})
}

// Shutdown gracefully shuts down the server
func (app *App) Shutdown(ctx context.Context) error {
	// Shutdown extensions
	if app.extensionManager != nil {
		if err := app.extensionManager.Shutdown(ctx); err != nil {
			log.Printf("Extension shutdown error: %v", err)
		}
	}

	// Shutdown HTTP server
	if app.server != nil {
		if err := app.server.Shutdown(ctx); err != nil {
			return fmt.Errorf("server shutdown error: %w", err)
		}
	}

	// Close database
	if app.sqlDB != nil {
		if err := app.sqlDB.Close(); err != nil {
			return fmt.Errorf("database close error: %w", err)
		}
	}

	return nil
}

func (app *App) handleShutdown() {
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
	<-sigChan

	log.Println("Shutdown signal received, starting graceful shutdown")

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	if err := app.Shutdown(ctx); err != nil {
		log.Printf("Shutdown error: %v", err)
	}
}

func parsePostgresURL(url string) *database.Config {
	// Simple URL parsing for postgres://user:pass@host:port/dbname
	// This is a simplified version, you might want to use a proper URL parser
	return &database.Config{
		Type:     "postgres",
		Host:     "localhost",
		Port:     5432,
		Database: "solobase",
		Username: "postgres",
		Password: "postgres",
		SSLMode:  "disable",
	}
}

// Database query callback functions
func beforeQuery(db *gorm.DB) {
	db.Set("query_start_time", time.Now())
}

func afterQuery(db *gorm.DB) {
	if startTime, ok := db.Get("query_start_time"); ok {
		if start, ok := startTime.(time.Time); ok {
			duration := time.Since(start)

			// Get operation type
			operation := "query"
			if db.Statement != nil && db.Statement.Schema != nil {
				operation = strings.ToLower(db.Statement.Schema.Table)
			}

			// Record in metrics collector
			middleware.RecordDBQuery(operation, duration.Seconds(), db.Error != nil)

			// Log slow queries (optional)
			if duration > 100*time.Millisecond {
				log.Printf("Slow query (%v): %s\n", duration, db.Statement.SQL.String())
			}
		}
	}
}
