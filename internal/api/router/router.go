package router

import (
	"fmt"
	"net/http"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/database"
	"github.com/suppers-ai/solobase/extensions/core"
	"github.com/suppers-ai/solobase/internal/api/handlers/analytics"
	"github.com/suppers-ai/solobase/internal/api/handlers/auth"
	"github.com/suppers-ai/solobase/internal/api/handlers/collections"
	dbhandlers "github.com/suppers-ai/solobase/internal/api/handlers/database"
	"github.com/suppers-ai/solobase/internal/api/handlers/extensions"
	"github.com/suppers-ai/solobase/internal/api/handlers/logs"
	"github.com/suppers-ai/solobase/internal/api/handlers/products"
	"github.com/suppers-ai/solobase/internal/api/handlers/settings"
	"github.com/suppers-ai/solobase/internal/api/handlers/shares"
	"github.com/suppers-ai/solobase/internal/api/handlers/storage"
	"github.com/suppers-ai/solobase/internal/api/handlers/system"
	"github.com/suppers-ai/solobase/internal/api/handlers/users"
	"github.com/suppers-ai/solobase/internal/api/middleware"
	"github.com/suppers-ai/solobase/internal/api/routes"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/internal/iam"
)

type API struct {
	Router            *mux.Router
	DB                *database.DB
	AuthService       *services.AuthService
	UserService       *services.UserService
	StorageService    *services.StorageService
	CollectionService *services.CollectionService
	DatabaseService   *services.DatabaseService
	SettingsService   *services.SettingsService
	LogsService       *services.LogsService
	IAMService        *iam.Service
	storageHandlers   *storage.StorageHandlers
	sharesHandler     *shares.SharesHandler
	analyticsHandlers *analytics.AnalyticsHandlers
	productHandlers   *products.ProductsExtensionHandlers
	ExtensionRegistry *core.ExtensionRegistry
}

func NewAPI(
	db *database.DB,
	authService *services.AuthService,
	userService *services.UserService,
	storageService *services.StorageService,
	collectionService *services.CollectionService,
	databaseService *services.DatabaseService,
	settingsService *services.SettingsService,
	logsService *services.LogsService,
	iamService *iam.Service,
	extensionRegistry *core.ExtensionRegistry,
) *API {
	api := &API{
		Router:            mux.NewRouter(),
		DB:                db,
		AuthService:       authService,
		UserService:       userService,
		StorageService:    storageService,
		CollectionService: collectionService,
		DatabaseService:   databaseService,
		SettingsService:   settingsService,
		LogsService:       logsService,
		IAMService:        iamService,
		ExtensionRegistry: extensionRegistry,
	}

	// Initialize storage handlers with hook support
	api.storageHandlers = storage.NewStorageHandlers(storageService, db, extensionRegistry)

	// Initialize shares handler
	api.sharesHandler = shares.NewSharesHandler(db)

	api.setupRoutes()
	return api
}

func (a *API) setupRoutes() {
	// The router is already mounted at /api in main.go, so we don't need the prefix here
	apiRouter := a.Router

	// Apply CORS and Metrics middleware to all API routes
	apiRouter.Use(middleware.CORSMiddleware)
	apiRouter.Use(middleware.MetricsMiddleware)

	// Health check endpoint for debugging
	apiRouter.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"status":"ok","message":"API is running"}`))
	}).Methods("GET", "OPTIONS")

	// Temporary endpoint to create test admin
	apiRouter.HandleFunc("/create-test-admin", func(w http.ResponseWriter, r *http.Request) {
		err := a.AuthService.CreateDefaultAdmin("admin@example.com", "admin123")
		if err != nil {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusInternalServerError)
			w.Write([]byte(`{"error":"` + err.Error() + `"}`))
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"message":"Admin created with password: admin123"}`))
	}).Methods("GET", "OPTIONS")

	// Public routes (no auth required)
	apiRouter.HandleFunc("/auth/login", auth.HandleLogin(a.AuthService, a.StorageService, a.ExtensionRegistry, a.IAMService)).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/auth/signup", auth.HandleSignup(a.AuthService)).Methods("POST", "OPTIONS")

	// Temporarily make dashboard public for testing
	apiRouter.HandleFunc("/dashboard/stats", system.HandleGetDashboardStats(
		a.UserService,
		a.StorageService,
		a.DatabaseService,
	)).Methods("GET", "OPTIONS")

	// Protected routes (auth required)
	protected := apiRouter.PathPrefix("").Subrouter()
	protected.Use(middleware.AuthMiddleware(a.AuthService))

	// Auth routes
	protected.HandleFunc("/auth/logout", auth.HandleLogout()).Methods("POST", "OPTIONS")
	protected.HandleFunc("/auth/me", auth.HandleGetCurrentUser()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/auth/change-password", auth.HandleChangePassword(a.AuthService)).Methods("POST", "OPTIONS")

	// User routes
	protected.HandleFunc("/users", users.HandleGetUsers(a.UserService)).Methods("GET", "OPTIONS")
	protected.HandleFunc("/users/{id}", users.HandleGetUser(a.UserService)).Methods("GET", "OPTIONS")
	protected.HandleFunc("/users/{id}", users.HandleUpdateUser(a.UserService)).Methods("PATCH", "OPTIONS")
	protected.HandleFunc("/users/{id}", users.HandleDeleteUser(a.UserService)).Methods("DELETE", "OPTIONS")

	// Dashboard routes
	protected.HandleFunc("/dashboard/stats", system.HandleGetDashboardStats(
		a.UserService,
		a.StorageService,
		a.DatabaseService,
	)).Methods("GET", "OPTIONS")

	// System metrics (temporarily public for development)
	apiRouter.HandleFunc("/system/metrics", system.HandleGetSystemMetrics()).Methods("GET", "OPTIONS")
	apiRouter.Handle("/metrics", system.HandlePrometheusMetrics()).Methods("GET", "OPTIONS")

	// Database routes (temporarily public for development)
	apiRouter.HandleFunc("/database/info", dbhandlers.HandleGetDatabaseInfo(a.DatabaseService)).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/database/tables", dbhandlers.HandleGetDatabaseTables(a.DatabaseService)).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/database/tables/{table}/columns", dbhandlers.HandleGetTableColumns(a.DatabaseService)).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/database/query", dbhandlers.HandleExecuteQuery(a.DatabaseService)).Methods("POST", "OPTIONS")

	// Storage routes (temporarily public for development)
	apiRouter.HandleFunc("/storage/buckets", a.storageHandlers.HandleGetStorageBuckets).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/storage/buckets", a.storageHandlers.HandleCreateBucket).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/storage/buckets/{bucket}", a.storageHandlers.HandleDeleteBucket).Methods("DELETE", "OPTIONS")
	apiRouter.HandleFunc("/storage/buckets/{bucket}/objects", a.storageHandlers.HandleGetBucketObjects).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/storage/buckets/{bucket}/upload", a.storageHandlers.HandleUploadFile).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/storage/buckets/{bucket}/upload-url", a.storageHandlers.HandleGenerateUploadURL).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/storage/direct-upload/{token}", a.storageHandlers.HandleDirectUpload).Methods("POST", "PUT", "OPTIONS")
	apiRouter.HandleFunc("/storage/buckets/{bucket}/objects/{id}", a.storageHandlers.HandleGetObject).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/storage/buckets/{bucket}/objects/{id}", a.storageHandlers.HandleDeleteObject).Methods("DELETE", "OPTIONS")
	apiRouter.HandleFunc("/storage/buckets/{bucket}/objects/{id}/download", a.storageHandlers.HandleDownloadObject).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/storage/buckets/{bucket}/objects/{id}/download-url", a.storageHandlers.HandleGenerateDownloadURL).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/storage/direct/{token}", a.storageHandlers.HandleDirectDownload).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/storage/buckets/{bucket}/objects/{id}/rename", a.storageHandlers.HandleRenameObject).Methods("PATCH", "OPTIONS")
	apiRouter.HandleFunc("/storage/buckets/{bucket}/objects/{id}/metadata", a.storageHandlers.HandleUpdateObjectMetadata).Methods("PATCH", "OPTIONS")
	apiRouter.HandleFunc("/storage/buckets/{bucket}/folders", a.storageHandlers.HandleCreateFolder).Methods("POST", "OPTIONS")

	// Storage quota and statistics routes
	apiRouter.HandleFunc("/storage/quota", a.storageHandlers.HandleGetStorageQuota).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/storage/stats", a.storageHandlers.HandleGetStorageStats).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/storage/admin/stats", a.storageHandlers.HandleGetAdminStorageStats).Methods("GET", "OPTIONS")

	// Recently viewed routes
	apiRouter.HandleFunc("/storage/recently-viewed", a.storageHandlers.HandleGetRecentlyViewed).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/storage/items/{id}/last-viewed", a.storageHandlers.HandleUpdateLastViewed).Methods("POST", "OPTIONS")

	// Search route
	apiRouter.HandleFunc("/storage/search", a.storageHandlers.HandleSearchStorageObjects).Methods("GET", "OPTIONS")

	// Logs routes (temporarily public for development)
	apiRouter.HandleFunc("/logs", logs.HandleGetLogs(a.LogsService)).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/logs/requests", logs.HandleGetRequestLogs(a.LogsService)).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/logs/stats", logs.HandleGetLogStats(a.LogsService)).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/logs/details", logs.HandleGetLogDetails(a.LogsService)).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/logs/clear", logs.HandleClearLogs(a.LogsService)).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/logs/export", logs.HandleExportLogs(a.LogsService)).Methods("GET", "OPTIONS")

	// Collection routes
	protected.HandleFunc("/collections", collections.HandleGetCollections(a.CollectionService)).Methods("GET", "OPTIONS")
	protected.HandleFunc("/collections", collections.HandleCreateCollection(a.CollectionService)).Methods("POST", "OPTIONS")
	protected.HandleFunc("/collections/{id}", collections.HandleGetCollection(a.CollectionService)).Methods("GET", "OPTIONS")
	protected.HandleFunc("/collections/{id}", collections.HandleUpdateCollection(a.CollectionService)).Methods("PATCH", "OPTIONS")
	protected.HandleFunc("/collections/{id}", collections.HandleDeleteCollection(a.CollectionService)).Methods("DELETE", "OPTIONS")

	// Settings routes
	protected.HandleFunc("/settings", settings.HandleGetSettings(a.SettingsService)).Methods("GET", "OPTIONS")
	protected.HandleFunc("/settings", settings.HandleUpdateSettings(a.SettingsService)).Methods("PATCH", "OPTIONS")
	protected.HandleFunc("/settings", settings.HandleSetSetting(a.SettingsService)).Methods("POST", "OPTIONS")
	protected.HandleFunc("/settings/reset", settings.HandleResetSettings(a.SettingsService)).Methods("POST", "OPTIONS")
	protected.HandleFunc("/settings/{key}", settings.HandleGetSetting(a.SettingsService)).Methods("GET", "OPTIONS")

	// IAM routes
	routes.RegisterIAMRoutes(apiRouter, a.IAMService)

	// Extensions routes (temporarily public for development)
	apiRouter.HandleFunc("/extensions", extensions.HandleGetExtensions()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/extensions/manage", extensions.HandleExtensionsManagement()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/extensions/{name}/toggle", extensions.HandleToggleExtension()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/extensions/status", extensions.HandleExtensionsStatus()).Methods("GET", "OPTIONS")

	// Extension dashboard routes (temporarily public for development)
	// Analytics dashboard
	a.analyticsHandlers = analytics.NewAnalyticsHandlers(a.DB)
	// Initialize analytics schema
	if err := a.analyticsHandlers.InitializeSchema(); err != nil {
		// Log error but don't fail startup
		fmt.Printf("Failed to initialize analytics schema: %v\n", err)
	}
	apiRouter.HandleFunc("/ext/analytics/dashboard", extensions.HandleAnalyticsDashboard()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/analytics/api/stats", a.analyticsHandlers.HandleStats()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/analytics/api/pageviews", a.analyticsHandlers.HandlePageViews()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/analytics/api/daily", a.analyticsHandlers.HandleDailyStats()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/analytics/api/track", a.analyticsHandlers.HandleTrack()).Methods("POST", "OPTIONS")

	// Webhooks dashboard
	apiRouter.HandleFunc("/ext/webhooks/dashboard", extensions.HandleWebhooksDashboard()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/webhooks/api/webhooks", extensions.HandleWebhooksList()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/webhooks/api/webhooks/create", extensions.HandleWebhooksCreate()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/ext/webhooks/api/webhooks/{id}/toggle", extensions.HandleWebhooksToggle()).Methods("POST", "OPTIONS")

	// Products extension routes
	// Initialize products handlers lazily to ensure migrations have run
	a.productHandlers = products.NewProductsExtensionHandlersWithDB(a.DB.DB)

	// Products basic CRUD (for compatibility)
	apiRouter.HandleFunc("/ext/products/api/products", a.productHandlers.HandleProductsList()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/products/api/products", a.productHandlers.HandleProductsCreate()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/ext/products/api/products/{id}", extensions.HandleProductsUpdate()).Methods("PUT", "OPTIONS")
	apiRouter.HandleFunc("/ext/products/api/products/{id}", extensions.HandleProductsDelete()).Methods("DELETE", "OPTIONS")
	apiRouter.HandleFunc("/ext/products/api/stats", a.productHandlers.HandleProductsStats()).Methods("GET", "OPTIONS")

	// Products extension - Variables management
	apiRouter.HandleFunc("/products/variables", a.productHandlers.HandleListVariables()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/products/variables", a.productHandlers.HandleCreateVariable()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/products/variables/{id}", a.productHandlers.HandleUpdateVariable()).Methods("PUT", "OPTIONS")
	apiRouter.HandleFunc("/products/variables/{id}", a.productHandlers.HandleDeleteVariable()).Methods("DELETE", "OPTIONS")

	// Products extension - Group Types
	apiRouter.HandleFunc("/products/group-types", a.productHandlers.HandleListGroupTypes()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/products/group-types", a.productHandlers.HandleCreateGroupType()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/products/group-types/{id}", a.productHandlers.HandleUpdateGroupType()).Methods("PUT", "OPTIONS")
	apiRouter.HandleFunc("/products/group-types/{id}", a.productHandlers.HandleDeleteGroupType()).Methods("DELETE", "OPTIONS")

	// Products extension - Groups (user's groups)
	apiRouter.HandleFunc("/products/groups", a.productHandlers.HandleListGroups()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/products/groups", a.productHandlers.HandleCreateGroup()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/products/groups/{id}", a.productHandlers.HandleUpdateGroup()).Methods("PUT", "OPTIONS")
	apiRouter.HandleFunc("/products/groups/{id}", a.productHandlers.HandleDeleteGroup()).Methods("DELETE", "OPTIONS")

	// User group and product endpoints (for user profile pages)
	apiRouter.HandleFunc("/user/groups/{id}", a.productHandlers.HandleGetGroup()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/user/groups/{id}/products", a.productHandlers.HandleGroupProducts()).Methods("GET", "OPTIONS")

	// Price calculation endpoint
	apiRouter.HandleFunc("/products/calculate-price", a.productHandlers.HandleCalculatePrice()).Methods("POST", "OPTIONS")

	// Products extension - Product Types
	apiRouter.HandleFunc("/products/product-types", a.productHandlers.HandleListProductTypes()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/products/product-types", a.productHandlers.HandleCreateProductType()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/products/product-types/{id}", a.productHandlers.HandleUpdateProductType()).Methods("PUT", "OPTIONS")
	apiRouter.HandleFunc("/products/product-types/{id}", a.productHandlers.HandleDeleteProductType()).Methods("DELETE", "OPTIONS")

	// Products extension - Pricing Templates
	apiRouter.HandleFunc("/products/pricing-templates", a.productHandlers.HandleListPricingTemplates()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/products/pricing-templates", a.productHandlers.HandleCreatePricingTemplate()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/products/pricing-templates/{id}", a.productHandlers.HandleUpdatePricingTemplate()).Methods("PUT", "OPTIONS")
	apiRouter.HandleFunc("/products/pricing-templates/{id}", a.productHandlers.HandleDeletePricingTemplate()).Methods("DELETE", "OPTIONS")

	// Hugo extension routes
	apiRouter.HandleFunc("/ext/hugo/api/sites", extensions.HandleHugoSitesList()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/hugo/api/sites", extensions.HandleHugoSitesCreate()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/ext/hugo/api/sites/{id}/build", extensions.HandleHugoSitesBuild()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/ext/hugo/api/sites/{id}", extensions.HandleHugoSitesDelete()).Methods("DELETE", "OPTIONS")
	apiRouter.HandleFunc("/ext/hugo/api/stats", extensions.HandleHugoStats()).Methods("GET", "OPTIONS")

	// Hugo file management routes
	apiRouter.HandleFunc("/ext/hugo/api/sites/{id}/files", extensions.HandleHugoSiteFiles()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/hugo/api/sites/{id}/files/read", extensions.HandleHugoFileRead()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/ext/hugo/api/sites/{id}/files/save", extensions.HandleHugoFileSave()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/ext/hugo/api/sites/{id}/files/create", extensions.HandleHugoFileCreate()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/ext/hugo/api/sites/{id}/files/delete", extensions.HandleHugoFileDelete()).Methods("POST", "OPTIONS")

	// Cloud Storage extension routes
	apiRouter.HandleFunc("/ext/cloudstorage/api/providers", extensions.HandleCloudStorageProviders()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/cloudstorage/api/providers", extensions.HandleCloudStorageAddProvider()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/ext/cloudstorage/api/activity", extensions.HandleCloudStorageActivity()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/cloudstorage/api/stats", extensions.HandleCloudStorageStats()).Methods("GET", "OPTIONS")

	// Shares routes - For SortedStorage and other apps that use sharing
	// These routes are temporarily public for development
	apiRouter.HandleFunc("/shares", a.sharesHandler.HandleShares()).Methods("GET", "POST", "OPTIONS")
	apiRouter.HandleFunc("/shares/{id}", a.sharesHandler.HandleShareByID()).Methods("GET", "DELETE", "OPTIONS")
}

func (a *API) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	a.Router.ServeHTTP(w, r)
}
