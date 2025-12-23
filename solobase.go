package solobase

import (
	"context"
	"database/sql"
	"embed"
	"fmt"
	"io/fs"
	"net/http"
	"strings"
	"time"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/extensions"
	"github.com/suppers-ai/solobase/extensions/core"
	authHandlers "github.com/suppers-ai/solobase/internal/api/handlers/auth"
	"github.com/suppers-ai/solobase/internal/api/middleware"
	"github.com/suppers-ai/solobase/internal/api/router"
	"github.com/suppers-ai/solobase/internal/config"
	"github.com/suppers-ai/solobase/internal/env"
	coremodels "github.com/suppers-ai/solobase/internal/core/models"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/internal/data/models"
	"github.com/suppers-ai/solobase/internal/iam"
	"github.com/suppers-ai/solobase/internal/pkg/database"
	"github.com/suppers-ai/solobase/internal/pkg/logger"
	"github.com/suppers-ai/solobase/pkg/adapters"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
	"github.com/suppers-ai/solobase/pkg/interfaces"
)

// App represents the Solobase application
type App struct {
	router           *mux.Router
	database         interfaces.Database // Host-provided database implementation
	config           *config.Config
	sqlDB            *sql.DB // Store sql.DB reference for Close() (optional, may be nil in WASM)
	appID            string  // Application ID for storage isolation
	services         *AppServices
	extensionManager *extensions.ExtensionManager
	server           *http.Server
	productsSeeder   interface{} // Custom seeder for Products extension
	externalUIFS     *embed.FS   // External UI files for TinyGo/WASM builds
	platform         Platform    // Platform-specific implementations

	// Event hooks
	onServeHooks     []func(*ServeEvent) error
	onBeforeAPIHooks []func(*APIEvent) error
	onAfterAPIHooks  []func(*APIEvent) error
	onModelHooks     map[string][]func(*ModelEvent) error

	// Custom routes registered before Start()
	customRoutes []customRoute
}

// customRoute holds route registration info
type customRoute struct {
	path      string
	handler   http.HandlerFunc
	methods   []string
	routeType routeType
}

// routeType defines the authentication level for a route
type routeType int

const (
	routeTypePublic routeType = iota
	routeTypeProtected
	routeTypeAdmin
)

// AppServices contains all the services used by the app
type AppServices struct {
	Auth         *services.AuthService
	User         *services.UserService
	Storage      *services.StorageService
	Database     *services.DatabaseService
	Settings     *services.SettingsService
	Logs         *services.LogsService
	Logger       *services.DBLogger
	IAM          *iam.Service
	CustomTables *services.CustomTablesService
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
	// Database is the host-provided database implementation.
	// This is REQUIRED - the host must provide a database that implements interfaces.Database.
	// Use builds/go/database.NewSQLite() for standard Go builds.
	// Use builds/tinygo/database.NewHTTPClient() for TinyGo builds.
	// Use builds/wasm/database.NewHostDB() for WASM builds.
	Database interfaces.Database

	// DatabaseType is used for SQL dialect selection (sqlite, postgres).
	// This is only needed for query generation, not connection.
	DatabaseType string

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

	// Platform provides platform-specific implementations.
	// Defaults to StandardPlatform if not specified.
	// Use WASMPlatform for WASM/WASI deployments.
	Platform Platform

	// ExternalUIFS allows providing an external embed.FS for UI files
	// This is useful for TinyGo/WASM builds where cross-package embeds don't work
	// The FS should contain files at the root level (index.html, _app/, etc.)
	ExternalUIFS *embed.FS
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

// Custom Table types - re-exported for easier access
type (
	// TableDefinition defines the schema for a custom table
	TableDefinition = models.CustomTableDefinition
	// TableField defines a column in a custom table
	TableField = models.CustomTableField
	// TableIndex defines an index on a custom table
	TableIndex = models.CustomTableIndex
	// TableOptions defines additional table options
	TableOptions = models.CustomTableOptions
	// ForeignKey defines a foreign key relationship
	ForeignKey = models.ForeignKeyDef
	// FieldValidation defines validation rules for a field
	FieldValidation = models.FieldValidation
	// DynamicRepository provides CRUD operations for custom tables
	DynamicRepository = coremodels.DynamicRepository
)

//go:embed all:frontend/build/*
var uiFiles embed.FS

// New creates a new Solobase application instance.
// DEPRECATED: Use NewWithOptions and provide a Database.
// This function will panic since Database is now required.
func New() *App {
	logger.StdLogFatal("New() is deprecated. Use NewWithOptions() with a Database implementation")
	return nil
}

// NewWithOptions creates a new Solobase app with custom options.
// The Database field in Options is REQUIRED - the host must provide a database implementation.
func NewWithOptions(opts *Options) *App {
	// Validate required database
	if opts.Database == nil {
		logger.StdLogFatal("Database is required. Provide a database implementation via Options.Database")
		return nil
	}

	// Set defaults
	if opts.DatabaseType == "" {
		opts.DatabaseType = env.GetEnv("DATABASE_TYPE")
		if opts.DatabaseType == "" {
			opts.DatabaseType = "sqlite"
		}
	}
	if opts.StorageType == "" {
		opts.StorageType = env.GetEnv("STORAGE_TYPE")
		if opts.StorageType == "" {
			opts.StorageType = "local"
		}
	}
	if opts.AppID == "" {
		opts.AppID = env.GetEnv("APP_ID")
		if opts.AppID == "" {
			opts.AppID = "solobase"
		}
	}
	if opts.JWTSecret == "" {
		opts.JWTSecret = env.GetEnv("JWT_SECRET")
		if opts.JWTSecret == "" {
			logger.StdLogFatal("JWT_SECRET environment variable is required for security. Please set a strong secret key")
		}
	}
	if opts.Port == "" {
		opts.Port = env.GetEnv("PORT")
		if opts.Port == "" {
			opts.Port = "8090"
		}
	}
	if opts.DefaultAdminEmail == "" {
		opts.DefaultAdminEmail = env.GetEnv("DEFAULT_ADMIN_EMAIL")
		logger.StdLogPrintf("DEBUG: DEFAULT_ADMIN_EMAIL from env: '%s'", opts.DefaultAdminEmail)
	} else {
		logger.StdLogPrintf("DEBUG: DefaultAdminEmail already set to: '%s'", opts.DefaultAdminEmail)
	}
	if opts.DefaultAdminPassword == "" {
		opts.DefaultAdminPassword = env.GetEnv("DEFAULT_ADMIN_PASSWORD")
	}

	// Set default platform if not provided
	platform := opts.Platform
	if platform == nil {
		platform = DefaultPlatform()
	}

	// Register database in the global adapter registry
	adapters.SetDatabase(opts.Database)

	// Get underlying sql.DB if available (may be nil in WASM)
	sqlDB := opts.Database.GetDB()

	app := &App{
		appID:          opts.AppID,
		database:       opts.Database,
		productsSeeder: opts.ProductsSeeder,
		onModelHooks:   make(map[string][]func(*ModelEvent) error),
		sqlDB:          sqlDB,
		externalUIFS:   opts.ExternalUIFS,
		platform:       platform,
	}

	// Create config
	app.config = &config.Config{
		Port:        opts.Port,
		Environment: env.GetEnv("ENVIRONMENT"),
		Database: &database.Config{
			Type: opts.DatabaseType,
		},
		Storage: config.StorageConfig{
			Type:             opts.StorageType,
			LocalStoragePath: "./.data/storage",
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

	// Database is now provided by the host - no need to create directories or connections here
	// The host is responsible for setting up the database before calling NewWithOptions
	sqlDB := app.sqlDB // May be nil in WASM mode

	readonlyMode := env.GetEnv("READONLY_MODE")

	// Skip migrations in read-only mode or WASM mode (host handles schema)
	if readonlyMode == "true" {
		// Read-only mode: skip schema migrations
	} else if sqlDB == nil {
		// WASM mode: no direct sql.DB access, host handles schema
	} else {
		// Run core schema migrations
		if err := runCoreSchemas(sqlDB); err != nil {
			return fmt.Errorf("failed to run core schemas: %w", err)
		}
	}

	// Initialize database logger
	dbLogger := services.NewDBLogger(sqlDB)

	// Adapters are already registered by NewWithOptions (database is set via adapters.SetDatabase)
	// JWT signer and other adapters are set up by the platform
	app.platform.InitializeAdapters(&AdapterConfig{
		JWTSecret: app.config.JWTSecret,
	})

	// Run extension schemas (skip in WASM mode - host handles schema)
	if sqlDB != nil {
		if err := runExtensionSchemas(sqlDB); err != nil {
			logger.StdLogPrintf("Warning: Failed to run extension schemas: %v", err)
		}
	}

	// Create repository factory and register with adapters
	// This must happen before IAM initialization since IAM now uses the repository
	var repoFactory repos.RepositoryFactory
	repoFactory = NewRepoFactory(sqlDB)
	if repoFactory != nil {
		adapters.SetRepos(repoFactory)
	}

	// Initialize IAM service (uses repository pattern)
	// Initialize when repoFactory is available (works for both standard and WASM builds)
	var iamService *iam.Service
	if repoFactory != nil {
		var err error
		iamService, err = app.platform.InitIAM(sqlDB, repoFactory.IAM())
		if err != nil {
			return fmt.Errorf("failed to initialize IAM service: %w", err)
		}
	}

	// Set up middleware dependencies
	middleware.SetAuthDB(sqlDB)
	middleware.SetIAMService(iamService) // May be nil in WASM mode

	// Create services - use repoFactory when available (works for both standard and WASM builds)
	if repoFactory != nil {
		app.services = &AppServices{
			Auth: services.NewAuthService(repoFactory.Users(), repoFactory.Tokens()),
			User: services.NewUserService(repoFactory.Users()),
			Storage: services.NewStorageServiceWithOptions(repoFactory.Storage(), app.config.Storage, &services.StorageOptions{
				AppID: app.appID,
			}),
			Settings: services.NewSettingsService(repoFactory.Settings()),
			Logs:     services.NewLogsService(repoFactory.Logs()),
			Logger:   dbLogger,
			IAM:      iamService,
		}
		// DatabaseService requires sqlDB directly
		if sqlDB != nil {
			app.services.Database = services.NewDatabaseService(sqlDB, app.config.Database.Type)
		}
	} else {
		// Fallback: no repoFactory available (shouldn't happen in normal operation)
		app.services = &AppServices{
			Logger: dbLogger,
			IAM:    iamService,
		}
	}

	// Create default admin (skip in read-only mode and WASM mode)
	// In WASM mode, the host should handle admin creation before WASM init
	if readonlyMode == "true" {
		// Read-only mode: skip admin creation
	} else if sqlDB == nil {
		// WASM mode: host handles admin creation via D1 migrations
		logger.StdLogPrintf("WASM mode: skipping admin creation (host should handle via D1)")
	} else if app.config.AdminEmail != "" && app.config.AdminPassword != "" {
		if err := app.services.Auth.CreateDefaultAdmin(app.config.AdminEmail, app.config.AdminPassword); err != nil {
			logger.StdLogPrintf("Warning: Failed to create default admin: %v", err)
		} else if iamService != nil {
			// Assign admin role in IAM to the default admin user
			var userID string
			err := sqlDB.QueryRow("SELECT id FROM auth_users WHERE email = ? AND deleted_at IS NULL", app.config.AdminEmail).Scan(&userID)
			if err == nil && userID != "" {
				if err := iamService.AssignRoleToUser(context.Background(), userID, "admin"); err != nil {
					logger.StdLogPrintf("Warning: Failed to assign admin role: %v", err)
				}
			}
		}
	}

	// Initialize extension system (skip in WASM mode without DB)
	if sqlDB != nil {
		extensionManager, err := extensions.NewExtensionManagerWithOptions(sqlDB, dbLogger, app.productsSeeder)
		if err != nil {
			return fmt.Errorf("failed to create extension manager: %w", err)
		}
		app.extensionManager = extensionManager

		// Initialize extensions
		ctx := context.Background()
		if err := extensionManager.Initialize(ctx); err != nil {
			logger.StdLogPrintf("Warning: Failed to initialize some extensions: %v", err)
		}
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
	if app.services == nil {
		if err := app.Initialize(); err != nil {
			return err
		}
	}

	// Setup router
	app.router = mux.NewRouter()

	// Apply middleware (only if services are available)
	if app.services != nil && app.services.Logger != nil {
		app.router.Use(services.HTTPLoggingMiddleware(app.services.Logger))
	}

	// Apply IAM middleware for authorization (only if IAM is available)
	if app.services != nil && app.services.IAM != nil {
		iamMiddleware := iam.NewMiddleware(app.services.IAM)
		app.router.Use(iamMiddleware.EnforceQuota())
		app.router.Use(iamMiddleware.RateLimit())
	}

	// Apply extension middleware (only if extension manager is available)
	if app.extensionManager != nil {
		app.router.Use(func(next http.Handler) http.Handler {
			return app.extensionManager.ApplyMiddleware(next)
		})
	}

	// Setup API router
	// Get extension registry (nil if no extension manager)
	var extRegistry *core.ExtensionRegistry
	if app.extensionManager != nil {
		extRegistry = app.extensionManager.GetRegistry()
	}

	apiRouter := router.NewAPI(
		app.sqlDB,
		app.services.Auth,
		app.services.User,
		app.services.Storage,
		app.services.Database,
		app.services.Settings,
		app.services.Logs,
		app.services.IAM,
		extRegistry,
	)

	// IMPORTANT: Register more specific routes first

	// Register IAM routes (only if IAM is available)
	if app.services != nil && app.services.IAM != nil {
		iamHandlers := iam.NewHandlers(app.services.IAM)
		iamHandlers.RegisterRoutes(app.router)
	}

	// Extension routes - register on apiRouter so they're under /api/ext/
	logger.StdLogPrintln("DEBUG: About to register extension routes")
	if app.extensionManager != nil {
		logger.StdLogPrintln("Registering extension routes...")
		app.extensionManager.RegisterRoutes(apiRouter.Router)
	} else {
		logger.StdLogPrintln("WARNING: Extension manager is nil, cannot register extension routes")
	}

	// Register custom routes
	app.registerCustomRoutes(apiRouter)

	// API routes
	logger.StdLogPrintln("DEBUG: Setting up API routes")
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
		app.router.PathPrefix("/app/").Handler(app.ServeUI()) // TinyGo compatibility

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
	app.platform.SetupShutdownHandler(func() {
		ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
		defer cancel()
		if err := app.Shutdown(ctx); err != nil {
			logger.StdLogPrintf("Shutdown error: %v", err)
		}
	})

	// Start scheduled log cleanup (runs daily, keeps logs for 7 days)
	app.platform.StartLogCleanupScheduler(func() {
		if app.services != nil && app.services.Logs != nil {
			if _, err := app.services.Logs.CleanupOldLogs(7); err != nil {
				logger.StdLogPrintf("Log cleanup error: %v", err)
			}
		}
	})

	// Start server
	logger.StdLogPrintf("🚀 Solobase server starting on port %s", app.config.Port)
	if err := app.server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		return fmt.Errorf("server failed to start: %w", err)
	}

	return nil
}

// Router returns the underlying router
func (app *App) Router() *mux.Router {
	return app.router
}

// Handler returns the HTTP handler for the app
// This is useful for WASM/Spin environments where you need to provide the handler
// to an external HTTP runtime instead of starting a server
func (app *App) Handler() http.Handler {
	return app.router
}

// SetupRouter initializes the app and sets up the router without starting the server
// This is useful for WASM/Spin environments
func (app *App) SetupRouter() error {
	// Initialize if not already done
	if app.sqlDB == nil {
		if err := app.Initialize(); err != nil {
			return err
		}
	} else if app.services == nil {
		// External DB provided but services not initialized
		if err := app.Initialize(); err != nil {
			return err
		}
	}

	// Setup router
	app.router = mux.NewRouter()

	// Apply middleware (only if services are available)
	if app.services != nil && app.services.Logger != nil {
		app.router.Use(services.HTTPLoggingMiddleware(app.services.Logger))
	}

	// Apply IAM middleware for authorization (only if IAM is available)
	if app.services != nil && app.services.IAM != nil {
		iamMiddleware := iam.NewMiddleware(app.services.IAM)
		app.router.Use(iamMiddleware.EnforceQuota())
		app.router.Use(iamMiddleware.RateLimit())
	}

	// Apply extension middleware (only if extension manager is available)
	if app.extensionManager != nil {
		app.router.Use(func(next http.Handler) http.Handler {
			return app.extensionManager.ApplyMiddleware(next)
		})
	}

	// Setup API router
	// Get extension registry (nil if no extension manager)
	var extRegistry *core.ExtensionRegistry
	if app.extensionManager != nil {
		extRegistry = app.extensionManager.GetRegistry()
	}

	apiRouter := router.NewAPI(
		app.sqlDB,
		app.services.Auth,
		app.services.User,
		app.services.Storage,
		app.services.Database,
		app.services.Settings,
		app.services.Logs,
		app.services.IAM,
		extRegistry,
	)

	// Register IAM routes (only if IAM is available)
	if app.services != nil && app.services.IAM != nil {
		iamHandlers := iam.NewHandlers(app.services.IAM)
		iamHandlers.RegisterRoutes(app.router)
	}

	// Extension routes
	if app.extensionManager != nil {
		app.extensionManager.RegisterRoutes(apiRouter.Router)
	}

	// Register custom routes
	app.registerCustomRoutes(apiRouter)

	// API routes
	app.router.PathPrefix("/api").Handler(http.StripPrefix("/api", apiRouter))

	// Storage files
	storageDir := "./.data/storage/"
	app.router.PathPrefix("/storage/").Handler(http.StripPrefix("/storage/", http.FileServer(http.Dir(storageDir))))

	// Admin UI routes (if not disabled)
	if !app.config.DisableUI {
		app.router.HandleFunc("/logo.png", app.ServeStaticAsset("logo.png"))
		app.router.HandleFunc("/logo_long.png", app.ServeStaticAsset("logo_long.png"))
		app.router.HandleFunc("/favicon.ico", app.ServeStaticAsset("favicon.ico"))
		app.router.PathPrefix("/_app/").Handler(app.ServeUI())
		app.router.PathPrefix("/app/").Handler(app.ServeUI()) // TinyGo compatibility
		app.router.PathPrefix("/auth/").Handler(app.ServeUI())
		app.router.PathPrefix("/profile").Handler(app.ServeUI())
		app.router.PathPrefix("/ui/").Handler(app.ServeUI())
		if !app.config.DisableHome {
			app.router.PathPrefix("/").Handler(app.ServeUI())
		}
	} else {
		// When UI is disabled, provide a basic API info endpoint at root
		app.router.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
			w.Header().Set("Content-Type", "application/json")
			w.Write([]byte(`{"name":"Solobase","status":"running","api":"/api"}`))
		}).Methods("GET")
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

	return nil
}

// DB returns the database connection
func (app *App) DB() interfaces.Database {
	return app.database
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

// RegisterPublicRoute registers a custom route that does not require authentication.
// The path should start with "/" and will be prefixed with "/api".
// Methods defaults to ["GET"] if not provided.
//
// Example:
//
//	app.RegisterPublicRoute("/custom/hello", func(w http.ResponseWriter, r *http.Request) {
//	    w.Write([]byte(`{"message": "Hello, World!"}`))
//	}, "GET")
func (app *App) RegisterPublicRoute(path string, handler http.HandlerFunc, methods ...string) {
	if len(methods) == 0 {
		methods = []string{"GET"}
	}
	app.customRoutes = append(app.customRoutes, customRoute{
		path:      path,
		handler:   handler,
		methods:   methods,
		routeType: routeTypePublic,
	})
}

// RegisterProtectedRoute registers a custom route that requires authentication.
// The authenticated user's context is available via:
//   - r.Context().Value(constants.ContextKeyUserID).(string) - User ID
//   - r.Context().Value(constants.ContextKeyUserEmail).(string) - User email
//   - r.Context().Value(constants.ContextKeyUserRoles).([]string) - User roles
//   - r.Context().Value("user").(*auth.User) - Full user object
//
// The path should start with "/" and will be prefixed with "/api".
// Methods defaults to ["GET"] if not provided.
//
// Example:
//
//	app.RegisterProtectedRoute("/custom/profile", func(w http.ResponseWriter, r *http.Request) {
//	    userID := r.Context().Value(constants.ContextKeyUserID).(string)
//	    // Use userID...
//	}, "GET")
func (app *App) RegisterProtectedRoute(path string, handler http.HandlerFunc, methods ...string) {
	if len(methods) == 0 {
		methods = []string{"GET"}
	}
	app.customRoutes = append(app.customRoutes, customRoute{
		path:      path,
		handler:   handler,
		methods:   methods,
		routeType: routeTypeProtected,
	})
}

// RegisterAdminRoute registers a custom route that requires admin role.
// The authenticated user's context is available (same as RegisterProtectedRoute).
// Additionally, the user must have the "admin" role.
//
// The path should start with "/" and will be prefixed with "/api".
// Methods defaults to ["GET"] if not provided.
//
// Example:
//
//	app.RegisterAdminRoute("/custom/admin/stats", func(w http.ResponseWriter, r *http.Request) {
//	    // Only admins can access this
//	}, "GET")
func (app *App) RegisterAdminRoute(path string, handler http.HandlerFunc, methods ...string) {
	if len(methods) == 0 {
		methods = []string{"GET"}
	}
	app.customRoutes = append(app.customRoutes, customRoute{
		path:      path,
		handler:   handler,
		methods:   methods,
		routeType: routeTypeAdmin,
	})
}

// registerCustomRoutes registers all custom routes on the API router
func (app *App) registerCustomRoutes(apiRouter *router.API) {
	if len(app.customRoutes) == 0 {
		return
	}

	// Skip custom routes if auth service is not available (WASM mode without database)
	if app.services == nil || app.services.Auth == nil {
		logger.StdLogPrintf("WASM mode: Skipping %d custom routes (auth service not available)", len(app.customRoutes))
		return
	}

	logger.StdLogPrintf("Registering %d custom routes...", len(app.customRoutes))

	// Create subrouters for protected and admin routes
	protectedRouter := apiRouter.Router.PathPrefix("").Subrouter()
	protectedRouter.Use(middleware.AuthMiddleware(app.services.Auth))

	adminRouter := apiRouter.Router.PathPrefix("").Subrouter()
	adminRouter.Use(middleware.AuthMiddleware(app.services.Auth))
	adminRouter.Use(middleware.AdminMiddleware(app.services.IAM))

	for _, route := range app.customRoutes {
		methods := append(route.methods, "OPTIONS") // Always allow OPTIONS for CORS

		switch route.routeType {
		case routeTypePublic:
			apiRouter.Router.HandleFunc(route.path, route.handler).Methods(methods...)
			logger.StdLogPrintf("  Registered public route: %s %v", route.path, route.methods)
		case routeTypeProtected:
			protectedRouter.HandleFunc(route.path, route.handler).Methods(methods...)
			logger.StdLogPrintf("  Registered protected route: %s %v", route.path, route.methods)
		case routeTypeAdmin:
			adminRouter.HandleFunc(route.path, route.handler).Methods(methods...)
			logger.StdLogPrintf("  Registered admin route: %s %v", route.path, route.methods)
		}
	}
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
		var uiFS fs.FS
		var err error

		// Check if external UI FS is provided (for TinyGo/WASM builds)
		if app.externalUIFS != nil {
			// External FS uses the same path structure: frontend/build/*
			uiFS, err = fs.Sub(*app.externalUIFS, "frontend/build")
			if err != nil {
				logger.StdLogPrintf("DEBUG: External fs.Sub failed: %v", err)
				http.Error(w, "Admin interface not available", http.StatusNotFound)
				return
			}
		} else {
			// Try embedded files from the default location
			uiFS, err = fs.Sub(uiFiles, "frontend/build")
			if err != nil {
				logger.StdLogPrintf("DEBUG: fs.Sub failed: %v", err)
				http.Error(w, "Admin interface not available", http.StatusNotFound)
				return
			}
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
				logger.StdLogPrintf("DEBUG: Failed to read index.html: %v", err)
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
			logger.StdLogPrintf("Extension shutdown error: %v", err)
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

// runCoreSchemas executes the core schema SQL to create tables.
// Uses raw SQL for TinyGo/WASI compatibility.
func runCoreSchemas(db *sql.DB) error {
	schemas := []string{
		// Auth tables
		`CREATE TABLE IF NOT EXISTS auth_users (
			id TEXT PRIMARY KEY,
			email TEXT NOT NULL UNIQUE,
			password TEXT NOT NULL,
			username TEXT,
			confirmed INTEGER DEFAULT 0,
			first_name TEXT,
			last_name TEXT,
			display_name TEXT,
			phone TEXT,
			location TEXT,
			confirm_token TEXT,
			confirm_selector TEXT,
			recover_token TEXT,
			recover_token_exp DATETIME,
			recover_selector TEXT,
			attempt_count INTEGER DEFAULT 0,
			last_attempt DATETIME,
			last_login DATETIME,
			metadata TEXT,
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			deleted_at DATETIME,
			totp_secret TEXT,
			totp_secret_backup TEXT,
			sms_phone_number TEXT,
			recovery_codes TEXT
		)`,
		`CREATE INDEX IF NOT EXISTS idx_auth_users_confirm_selector ON auth_users(confirm_selector)`,
		`CREATE INDEX IF NOT EXISTS idx_auth_users_recover_selector ON auth_users(recover_selector)`,
		`CREATE INDEX IF NOT EXISTS idx_auth_users_deleted_at ON auth_users(deleted_at)`,
		`CREATE TABLE IF NOT EXISTS auth_tokens (
			id TEXT PRIMARY KEY,
			user_id TEXT NOT NULL,
			token_hash TEXT,
			token TEXT,
			type TEXT NOT NULL,
			family_id TEXT,
			provider TEXT,
			provider_uid TEXT,
			access_token TEXT,
			oauth_expiry DATETIME,
			expires_at DATETIME NOT NULL,
			used_at DATETIME,
			revoked_at DATETIME,
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			device_info TEXT,
			ip_address TEXT,
			FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE
		)`,
		`CREATE INDEX IF NOT EXISTS idx_auth_tokens_user_id ON auth_tokens(user_id)`,
		`CREATE INDEX IF NOT EXISTS idx_auth_tokens_token_hash ON auth_tokens(token_hash)`,
		`CREATE INDEX IF NOT EXISTS idx_auth_tokens_token ON auth_tokens(token)`,
		`CREATE INDEX IF NOT EXISTS idx_auth_tokens_type ON auth_tokens(type)`,
		`CREATE INDEX IF NOT EXISTS idx_auth_tokens_family_id ON auth_tokens(family_id)`,
		`CREATE INDEX IF NOT EXISTS idx_auth_tokens_provider_uid ON auth_tokens(provider_uid)`,
		`CREATE INDEX IF NOT EXISTS idx_auth_tokens_expires_at ON auth_tokens(expires_at)`,
		`CREATE INDEX IF NOT EXISTS idx_auth_tokens_revoked_at ON auth_tokens(revoked_at)`,
		`CREATE TABLE IF NOT EXISTS api_keys (
			id TEXT PRIMARY KEY,
			user_id TEXT NOT NULL,
			name TEXT NOT NULL,
			key_prefix TEXT NOT NULL,
			key_hash TEXT NOT NULL UNIQUE,
			scopes TEXT,
			expires_at DATETIME,
			last_used_at DATETIME,
			last_used_ip TEXT,
			revoked_at DATETIME,
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE
		)`,
		`CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id)`,
		`CREATE INDEX IF NOT EXISTS idx_api_keys_key_prefix ON api_keys(key_prefix)`,
		`CREATE INDEX IF NOT EXISTS idx_api_keys_revoked_at ON api_keys(revoked_at)`,

		// IAM tables
		`CREATE TABLE IF NOT EXISTS iam_roles (
			id TEXT PRIMARY KEY,
			name TEXT NOT NULL UNIQUE,
			display_name TEXT,
			description TEXT,
			type TEXT,
			metadata TEXT,
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
		)`,
		`CREATE TABLE IF NOT EXISTS iam_user_roles (
			id TEXT PRIMARY KEY,
			user_id TEXT NOT NULL,
			role_id TEXT NOT NULL,
			granted_by TEXT,
			granted_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			expires_at DATETIME,
			UNIQUE(user_id, role_id)
		)`,
		`CREATE INDEX IF NOT EXISTS idx_iam_user_roles_user_id ON iam_user_roles(user_id)`,
		`CREATE INDEX IF NOT EXISTS idx_iam_user_roles_role_id ON iam_user_roles(role_id)`,
		`CREATE TABLE IF NOT EXISTS iam_policies (
			id TEXT PRIMARY KEY,
			ptype TEXT NOT NULL,
			v0 TEXT,
			v1 TEXT,
			v2 TEXT,
			v3 TEXT,
			v4 TEXT,
			v5 TEXT,
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
		)`,
		`CREATE INDEX IF NOT EXISTS idx_iam_policies_ptype ON iam_policies(ptype)`,
		`CREATE INDEX IF NOT EXISTS idx_iam_policies_v0 ON iam_policies(v0)`,
		`CREATE TABLE IF NOT EXISTS iam_audit_logs (
			id TEXT PRIMARY KEY,
			user_id TEXT,
			action TEXT,
			resource TEXT,
			result TEXT,
			reason TEXT,
			ip_address TEXT,
			user_agent TEXT,
			metadata TEXT,
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
		)`,
		`CREATE INDEX IF NOT EXISTS idx_iam_audit_logs_user_id ON iam_audit_logs(user_id)`,
		`CREATE INDEX IF NOT EXISTS idx_iam_audit_logs_created_at ON iam_audit_logs(created_at)`,

		// Settings
		`CREATE TABLE IF NOT EXISTS sys_settings (
			id TEXT PRIMARY KEY,
			key TEXT NOT NULL UNIQUE,
			value TEXT,
			type TEXT DEFAULT 'string',
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			deleted_at DATETIME
		)`,
		`CREATE INDEX IF NOT EXISTS idx_sys_settings_deleted_at ON sys_settings(deleted_at)`,

		// Storage
		`CREATE TABLE IF NOT EXISTS storage_buckets (
			id TEXT PRIMARY KEY,
			name TEXT NOT NULL UNIQUE,
			public INTEGER DEFAULT 0,
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
		)`,
		`CREATE TABLE IF NOT EXISTS storage_objects (
			id TEXT PRIMARY KEY,
			bucket_name TEXT NOT NULL,
			object_name TEXT NOT NULL,
			parent_folder_id TEXT,
			size INTEGER,
			content_type TEXT,
			checksum TEXT,
			metadata TEXT,
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			last_viewed DATETIME,
			user_id TEXT,
			app_id TEXT
		)`,
		`CREATE INDEX IF NOT EXISTS idx_storage_objects_bucket_name ON storage_objects(bucket_name)`,
		`CREATE INDEX IF NOT EXISTS idx_storage_objects_object_name ON storage_objects(object_name)`,
		`CREATE INDEX IF NOT EXISTS idx_storage_objects_parent_folder_id ON storage_objects(parent_folder_id)`,
		`CREATE INDEX IF NOT EXISTS idx_storage_objects_checksum ON storage_objects(checksum)`,
		`CREATE INDEX IF NOT EXISTS idx_storage_objects_last_viewed ON storage_objects(last_viewed)`,
		`CREATE INDEX IF NOT EXISTS idx_storage_objects_user_id ON storage_objects(user_id)`,
		`CREATE INDEX IF NOT EXISTS idx_storage_objects_app_id ON storage_objects(app_id)`,
		`CREATE TABLE IF NOT EXISTS storage_upload_tokens (
			id TEXT PRIMARY KEY,
			token TEXT NOT NULL UNIQUE,
			bucket TEXT NOT NULL,
			parent_folder_id TEXT,
			object_name TEXT NOT NULL,
			user_id TEXT,
			max_size INTEGER,
			content_type TEXT,
			bytes_uploaded INTEGER DEFAULT 0,
			completed INTEGER DEFAULT 0,
			object_id TEXT,
			expires_at DATETIME NOT NULL,
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			completed_at DATETIME,
			client_ip TEXT
		)`,
		`CREATE TABLE IF NOT EXISTS storage_download_tokens (
			id TEXT PRIMARY KEY,
			token TEXT NOT NULL UNIQUE,
			file_id TEXT NOT NULL,
			bucket TEXT NOT NULL,
			parent_folder_id TEXT,
			object_name TEXT NOT NULL,
			user_id TEXT,
			file_size INTEGER,
			bytes_served INTEGER DEFAULT 0,
			completed INTEGER DEFAULT 0,
			expires_at DATETIME NOT NULL,
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			callback_at DATETIME,
			client_ip TEXT
		)`,

		// Logging
		`CREATE TABLE IF NOT EXISTS sys_logs (
			id TEXT PRIMARY KEY,
			level TEXT NOT NULL,
			message TEXT NOT NULL,
			fields TEXT,
			user_id TEXT,
			trace_id TEXT,
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
		)`,
		`CREATE INDEX IF NOT EXISTS idx_sys_logs_level ON sys_logs(level)`,
		`CREATE INDEX IF NOT EXISTS idx_sys_logs_user_id ON sys_logs(user_id)`,
		`CREATE INDEX IF NOT EXISTS idx_sys_logs_trace_id ON sys_logs(trace_id)`,
		`CREATE INDEX IF NOT EXISTS idx_sys_logs_created_at ON sys_logs(created_at)`,
		`CREATE TABLE IF NOT EXISTS sys_request_logs (
			id TEXT PRIMARY KEY,
			level TEXT NOT NULL,
			method TEXT NOT NULL,
			path TEXT NOT NULL,
			query TEXT,
			status_code INTEGER NOT NULL,
			exec_time_ms INTEGER NOT NULL,
			user_ip TEXT NOT NULL,
			user_agent TEXT,
			user_id TEXT,
			trace_id TEXT,
			error TEXT,
			request_body TEXT,
			response_body TEXT,
			headers TEXT,
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
		)`,
		`CREATE INDEX IF NOT EXISTS idx_sys_request_logs_method ON sys_request_logs(method)`,
		`CREATE INDEX IF NOT EXISTS idx_sys_request_logs_path ON sys_request_logs(path)`,
		`CREATE INDEX IF NOT EXISTS idx_sys_request_logs_status_code ON sys_request_logs(status_code)`,
		`CREATE INDEX IF NOT EXISTS idx_sys_request_logs_user_id ON sys_request_logs(user_id)`,
		`CREATE INDEX IF NOT EXISTS idx_sys_request_logs_created_at ON sys_request_logs(created_at)`,

		// Custom tables
		`CREATE TABLE IF NOT EXISTS custom_table_definitions (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			name TEXT NOT NULL UNIQUE,
			display_name TEXT,
			description TEXT,
			fields TEXT,
			indexes TEXT,
			options TEXT,
			created_by TEXT,
			status TEXT DEFAULT 'active',
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
		)`,
		`CREATE TABLE IF NOT EXISTS custom_table_migrations (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			table_id INTEGER,
			version INTEGER,
			migration_type TEXT,
			old_schema TEXT,
			new_schema TEXT,
			executed_by TEXT,
			executed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			rollback_at DATETIME,
			status TEXT,
			error_message TEXT
		)`,
		`CREATE INDEX IF NOT EXISTS idx_custom_table_migrations_table_id ON custom_table_migrations(table_id)`,
	}

	for _, schema := range schemas {
		if _, err := db.Exec(schema); err != nil {
			return fmt.Errorf("failed to execute schema: %w", err)
		}
	}

	return nil
}

// runExtensionSchemas executes the SQL schema files for extensions.
// Uses raw SQL for TinyGo/WASI compatibility.
func runExtensionSchemas(db *sql.DB) error {
	schemas := []string{
		// Products extension
		`CREATE TABLE IF NOT EXISTS ext_products_variables (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			name TEXT NOT NULL UNIQUE,
			display_name TEXT,
			value_type TEXT,
			type TEXT,
			default_value TEXT,
			description TEXT,
			status TEXT DEFAULT 'active',
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
		)`,
		`CREATE TABLE IF NOT EXISTS ext_products_group_templates (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			name TEXT NOT NULL UNIQUE,
			display_name TEXT,
			description TEXT,
			icon TEXT,
			filter_fields_schema TEXT,
			status TEXT DEFAULT 'active',
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
		)`,
		`CREATE TABLE IF NOT EXISTS ext_products_groups (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			user_id TEXT NOT NULL,
			group_template_id INTEGER NOT NULL,
			name TEXT NOT NULL,
			description TEXT,
			filter_numeric_1 REAL, filter_numeric_2 REAL, filter_numeric_3 REAL, filter_numeric_4 REAL, filter_numeric_5 REAL,
			filter_text_1 TEXT, filter_text_2 TEXT, filter_text_3 TEXT, filter_text_4 TEXT, filter_text_5 TEXT,
			filter_boolean_1 INTEGER, filter_boolean_2 INTEGER, filter_boolean_3 INTEGER, filter_boolean_4 INTEGER, filter_boolean_5 INTEGER,
			filter_enum_1 TEXT, filter_enum_2 TEXT, filter_enum_3 TEXT, filter_enum_4 TEXT, filter_enum_5 TEXT,
			filter_location_1 TEXT, filter_location_2 TEXT, filter_location_3 TEXT, filter_location_4 TEXT, filter_location_5 TEXT,
			custom_fields TEXT,
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			FOREIGN KEY (group_template_id) REFERENCES ext_products_group_templates(id)
		)`,
		`CREATE TABLE IF NOT EXISTS ext_products_product_templates (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			name TEXT NOT NULL UNIQUE,
			display_name TEXT,
			description TEXT,
			category TEXT,
			icon TEXT,
			filter_fields_schema TEXT,
			custom_fields_schema TEXT,
			pricing_templates TEXT,
			billing_mode TEXT DEFAULT 'instant' NOT NULL,
			billing_type TEXT DEFAULT 'one-time' NOT NULL,
			billing_recurring_interval TEXT,
			billing_recurring_interval_count INTEGER DEFAULT 1,
			status TEXT DEFAULT 'active',
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
		)`,
		`CREATE TABLE IF NOT EXISTS ext_products_products (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			group_id INTEGER NOT NULL,
			product_template_id INTEGER NOT NULL,
			name TEXT NOT NULL,
			description TEXT,
			base_price REAL,
			base_price_cents INTEGER,
			currency TEXT DEFAULT 'USD',
			filter_numeric_1 REAL, filter_numeric_2 REAL, filter_numeric_3 REAL, filter_numeric_4 REAL, filter_numeric_5 REAL,
			filter_text_1 TEXT, filter_text_2 TEXT, filter_text_3 TEXT, filter_text_4 TEXT, filter_text_5 TEXT,
			filter_boolean_1 INTEGER, filter_boolean_2 INTEGER, filter_boolean_3 INTEGER, filter_boolean_4 INTEGER, filter_boolean_5 INTEGER,
			filter_enum_1 TEXT, filter_enum_2 TEXT, filter_enum_3 TEXT, filter_enum_4 TEXT, filter_enum_5 TEXT,
			filter_location_1 TEXT, filter_location_2 TEXT, filter_location_3 TEXT, filter_location_4 TEXT, filter_location_5 TEXT,
			custom_fields TEXT,
			variables TEXT,
			pricing_formula TEXT,
			active INTEGER DEFAULT 1,
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			FOREIGN KEY (group_id) REFERENCES ext_products_groups(id),
			FOREIGN KEY (product_template_id) REFERENCES ext_products_product_templates(id)
		)`,
		`CREATE TABLE IF NOT EXISTS ext_products_pricing_templates (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			name TEXT NOT NULL UNIQUE,
			display_name TEXT,
			description TEXT,
			price_formula TEXT NOT NULL,
			condition_formula TEXT,
			variables TEXT,
			category TEXT,
			status TEXT DEFAULT 'active',
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
		)`,
		`CREATE TABLE IF NOT EXISTS ext_products_purchases (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			user_id TEXT NOT NULL,
			provider TEXT DEFAULT 'stripe',
			provider_session_id TEXT,
			provider_payment_intent_id TEXT,
			provider_subscription_id TEXT,
			line_items TEXT,
			product_metadata TEXT,
			tax_items TEXT,
			amount_cents INTEGER,
			tax_cents INTEGER,
			total_cents INTEGER,
			currency TEXT DEFAULT 'USD',
			status TEXT DEFAULT 'pending',
			requires_approval INTEGER DEFAULT 0,
			approved_at DATETIME,
			approved_by TEXT,
			refunded_at DATETIME,
			refund_reason TEXT,
			refund_amount INTEGER,
			cancelled_at DATETIME,
			cancel_reason TEXT,
			success_url TEXT,
			cancel_url TEXT,
			customer_email TEXT,
			customer_name TEXT,
			billing_address TEXT,
			shipping_address TEXT,
			payment_method_types TEXT,
			expires_at DATETIME,
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
		)`,
		// LegalPages extension
		`CREATE TABLE IF NOT EXISTS ext_legalpages_legal_documents (
			id TEXT PRIMARY KEY,
			document_type TEXT NOT NULL,
			title TEXT NOT NULL,
			content TEXT,
			version INTEGER NOT NULL DEFAULT 1,
			status TEXT DEFAULT 'draft',
			created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			created_by TEXT
		)`,
	}

	for _, schema := range schemas {
		if _, err := db.Exec(schema); err != nil {
			return fmt.Errorf("failed to execute schema: %w", err)
		}
	}

	return nil
}
