package api

import (
	"fmt"
	"net/http"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/database"
	"github.com/suppers-ai/solobase/extensions/core"
	"github.com/suppers-ai/solobase/services"
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
	productHandlers   *ProductsExtensionHandlers
	analyticsHandlers *AnalyticsHandlers
	storageHandlers   *StorageHandlers
	sharesHandler     *SharesHandler
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
		ExtensionRegistry: extensionRegistry,
	}

	// Initialize storage handlers with hook support
	api.storageHandlers = NewStorageHandlers(storageService, db, extensionRegistry)

	// Initialize shares handler
	api.sharesHandler = NewSharesHandler(db)

	api.setupRoutes()
	return api
}

func (a *API) setupRoutes() {
	// The router is already mounted at /api in main.go, so we don't need the prefix here
	apiRouter := a.Router

	// Apply CORS and Metrics middleware to all API routes
	apiRouter.Use(CORSMiddleware)
	apiRouter.Use(MetricsMiddleware)

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
	apiRouter.HandleFunc("/auth/login", HandleLogin(a.AuthService, a.StorageService, a.ExtensionRegistry)).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/auth/signup", HandleSignup(a.AuthService)).Methods("POST", "OPTIONS")

	// Temporarily make dashboard public for testing
	apiRouter.HandleFunc("/dashboard/stats", HandleGetDashboardStats(
		a.UserService,
		a.StorageService,
		a.DatabaseService,
	)).Methods("GET", "OPTIONS")

	// Protected routes (auth required)
	protected := apiRouter.PathPrefix("").Subrouter()
	protected.Use(AuthMiddleware(a.AuthService))

	// Auth routes
	protected.HandleFunc("/auth/logout", HandleLogout()).Methods("POST", "OPTIONS")
	protected.HandleFunc("/auth/me", HandleGetCurrentUser()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/auth/change-password", HandleChangePassword(a.AuthService)).Methods("POST", "OPTIONS")

	// User routes
	protected.HandleFunc("/users", HandleGetUsers(a.UserService)).Methods("GET", "OPTIONS")
	protected.HandleFunc("/users/{id}", HandleGetUser(a.UserService)).Methods("GET", "OPTIONS")
	protected.HandleFunc("/users/{id}", HandleUpdateUser(a.UserService)).Methods("PATCH", "OPTIONS")
	protected.HandleFunc("/users/{id}", HandleDeleteUser(a.UserService)).Methods("DELETE", "OPTIONS")

	// Dashboard routes
	protected.HandleFunc("/dashboard/stats", HandleGetDashboardStats(
		a.UserService,
		a.StorageService,
		a.DatabaseService,
	)).Methods("GET", "OPTIONS")

	// System metrics (temporarily public for development)
	apiRouter.HandleFunc("/system/metrics", HandleGetSystemMetrics()).Methods("GET", "OPTIONS")
	apiRouter.Handle("/metrics", HandlePrometheusMetrics()).Methods("GET", "OPTIONS")

	// Database routes (temporarily public for development)
	apiRouter.HandleFunc("/database/info", HandleGetDatabaseInfo(a.DatabaseService)).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/database/tables", HandleGetDatabaseTables(a.DatabaseService)).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/database/tables/{table}/columns", HandleGetTableColumns(a.DatabaseService)).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/database/query", HandleExecuteQuery(a.DatabaseService)).Methods("POST", "OPTIONS")

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
	apiRouter.HandleFunc("/logs", HandleGetLogs(a.LogsService)).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/logs/requests", HandleGetRequestLogs(a.LogsService)).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/logs/stats", HandleGetLogStats(a.LogsService)).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/logs/details", HandleGetLogDetails(a.LogsService)).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/logs/clear", HandleClearLogs(a.LogsService)).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/logs/export", HandleExportLogs(a.LogsService)).Methods("GET", "OPTIONS")

	// Collection routes
	protected.HandleFunc("/collections", HandleGetCollections(a.CollectionService)).Methods("GET", "OPTIONS")
	protected.HandleFunc("/collections", HandleCreateCollection(a.CollectionService)).Methods("POST", "OPTIONS")
	protected.HandleFunc("/collections/{id}", HandleGetCollection(a.CollectionService)).Methods("GET", "OPTIONS")
	protected.HandleFunc("/collections/{id}", HandleUpdateCollection(a.CollectionService)).Methods("PATCH", "OPTIONS")
	protected.HandleFunc("/collections/{id}", HandleDeleteCollection(a.CollectionService)).Methods("DELETE", "OPTIONS")

	// Settings routes
	protected.HandleFunc("/settings", HandleGetSettings(a.SettingsService)).Methods("GET", "OPTIONS")
	protected.HandleFunc("/settings", HandleUpdateSettings(a.SettingsService)).Methods("PATCH", "OPTIONS")
	protected.HandleFunc("/settings", HandleSetSetting(a.SettingsService)).Methods("POST", "OPTIONS")
	protected.HandleFunc("/settings/reset", HandleResetSettings(a.SettingsService)).Methods("POST", "OPTIONS")
	protected.HandleFunc("/settings/{key}", HandleGetSetting(a.SettingsService)).Methods("GET", "OPTIONS")

	// Extensions routes (temporarily public for development)
	apiRouter.HandleFunc("/extensions", HandleGetExtensions()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/extensions/manage", HandleExtensionsManagement()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/extensions/{name}/toggle", HandleToggleExtension()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/extensions/status", HandleExtensionsStatus()).Methods("GET", "OPTIONS")

	// Extension dashboard routes (temporarily public for development)
	// Analytics dashboard
	a.analyticsHandlers = NewAnalyticsHandlers(a.DB)
	// Initialize analytics schema
	if err := a.analyticsHandlers.InitializeSchema(); err != nil {
		// Log error but don't fail startup
		fmt.Printf("Failed to initialize analytics schema: %v\n", err)
	}
	apiRouter.HandleFunc("/ext/analytics/dashboard", HandleAnalyticsDashboard()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/analytics/api/stats", a.analyticsHandlers.HandleStats()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/analytics/api/pageviews", a.analyticsHandlers.HandlePageViews()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/analytics/api/daily", a.analyticsHandlers.HandleDailyStats()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/analytics/api/track", a.analyticsHandlers.HandleTrack()).Methods("POST", "OPTIONS")

	// Webhooks dashboard
	apiRouter.HandleFunc("/ext/webhooks/dashboard", HandleWebhooksDashboard()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/webhooks/api/webhooks", HandleWebhooksList()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/webhooks/api/webhooks/create", HandleWebhooksCreate()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/ext/webhooks/api/webhooks/{id}/toggle", HandleWebhooksToggle()).Methods("POST", "OPTIONS")

	// Products extension routes
	// Initialize products handlers lazily to ensure migrations have run
	a.productHandlers = NewProductsExtensionHandlersWithDB(a.DB.DB)

	// Products basic CRUD (for compatibility)
	apiRouter.HandleFunc("/ext/products/api/products", a.productHandlers.HandleProductsList()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/products/api/products", a.productHandlers.HandleProductsCreate()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/ext/products/api/products/{id}", HandleProductsUpdate()).Methods("PUT", "OPTIONS")
	apiRouter.HandleFunc("/ext/products/api/products/{id}", HandleProductsDelete()).Methods("DELETE", "OPTIONS")
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
	apiRouter.HandleFunc("/ext/hugo/api/sites", HandleHugoSitesList()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/hugo/api/sites", HandleHugoSitesCreate()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/ext/hugo/api/sites/{id}/build", HandleHugoSitesBuild()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/ext/hugo/api/sites/{id}", HandleHugoSitesDelete()).Methods("DELETE", "OPTIONS")
	apiRouter.HandleFunc("/ext/hugo/api/stats", HandleHugoStats()).Methods("GET", "OPTIONS")

	// Hugo file management routes
	apiRouter.HandleFunc("/ext/hugo/api/sites/{id}/files", HandleHugoSiteFiles()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/hugo/api/sites/{id}/files/read", HandleHugoFileRead()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/ext/hugo/api/sites/{id}/files/save", HandleHugoFileSave()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/ext/hugo/api/sites/{id}/files/create", HandleHugoFileCreate()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/ext/hugo/api/sites/{id}/files/delete", HandleHugoFileDelete()).Methods("POST", "OPTIONS")

	// Cloud Storage extension routes
	apiRouter.HandleFunc("/ext/cloudstorage/api/providers", HandleCloudStorageProviders()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/cloudstorage/api/providers", HandleCloudStorageAddProvider()).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/ext/cloudstorage/api/activity", HandleCloudStorageActivity()).Methods("GET", "OPTIONS")
	apiRouter.HandleFunc("/ext/cloudstorage/api/stats", HandleCloudStorageStats()).Methods("GET", "OPTIONS")

	// Shares routes - For SortedStorage and other apps that use sharing
	// These routes are temporarily public for development
	apiRouter.HandleFunc("/shares", a.sharesHandler.HandleShares()).Methods("GET", "POST", "OPTIONS")
	apiRouter.HandleFunc("/shares/{id}", a.sharesHandler.HandleShareByID()).Methods("GET", "DELETE", "OPTIONS")
}

func (a *API) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	a.Router.ServeHTTP(w, r)
}
