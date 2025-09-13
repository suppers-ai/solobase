package router

import (
	"fmt"
	"net/http"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/database"
	"github.com/suppers-ai/solobase/extensions/core"
	"github.com/suppers-ai/solobase/internal/api/handlers/analytics"
	"github.com/suppers-ai/solobase/internal/api/handlers/auth"
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

	api.setupRoutesWithAdmin()
	return api
}

// setupRoutesWithAdmin sets up all routes with proper admin namespace
func (a *API) setupRoutesWithAdmin() {
	apiRouter := a.Router

	// Apply CORS and Metrics middleware to all API routes
	apiRouter.Use(middleware.CORSMiddleware)
	apiRouter.Use(middleware.MetricsMiddleware)

	// ==================================
	// PUBLIC ROUTES (no auth required)
	// ==================================
	
	// Health & Monitoring
	apiRouter.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"status":"ok","message":"API is running"}`))
	}).Methods("GET", "OPTIONS")
	
	apiRouter.Handle("/metrics", system.HandlePrometheusMetrics()).Methods("GET", "OPTIONS")

	// Authentication (public endpoints)
	apiRouter.HandleFunc("/auth/login", auth.HandleLogin(a.AuthService, a.StorageService, a.ExtensionRegistry, a.IAMService)).Methods("POST", "OPTIONS")
	apiRouter.HandleFunc("/auth/signup", auth.HandleSignup(a.AuthService, a.IAMService)).Methods("POST", "OPTIONS")

	// Direct download with tokens (public but token-protected)
	apiRouter.HandleFunc("/storage/direct/{token}", a.storageHandlers.HandleDirectDownload).Methods("GET", "OPTIONS")

	// ==================================
	// PROTECTED ROUTES (auth required)
	// ==================================
	
	protected := apiRouter.PathPrefix("").Subrouter()
	protected.Use(middleware.AuthMiddleware(a.AuthService))

	// ---- Current User Operations (any authenticated user) ----
	
	// Auth operations
	protected.HandleFunc("/auth/logout", auth.HandleLogout()).Methods("POST", "OPTIONS")
	protected.HandleFunc("/auth/me", auth.HandleGetCurrentUser()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/auth/me", auth.HandleUpdateCurrentUser(a.UserService)).Methods("PATCH", "OPTIONS")
	protected.HandleFunc("/auth/change-password", auth.HandleChangePassword(a.AuthService)).Methods("POST", "OPTIONS")
	

	// ---- User Storage (regular user access) ----
	
	// Buckets
	protected.HandleFunc("/storage/buckets", a.storageHandlers.HandleGetStorageBuckets).Methods("GET", "OPTIONS")
	protected.HandleFunc("/storage/buckets", a.storageHandlers.HandleCreateBucket).Methods("POST", "OPTIONS")
	protected.HandleFunc("/storage/buckets/{bucket}", a.storageHandlers.HandleDeleteBucket).Methods("DELETE", "OPTIONS")
	
	// Objects in buckets
	protected.HandleFunc("/storage/buckets/{bucket}/objects", a.storageHandlers.HandleGetBucketObjects).Methods("GET", "OPTIONS")
	protected.HandleFunc("/storage/buckets/{bucket}/upload", a.storageHandlers.HandleUploadFile).Methods("POST", "OPTIONS")
	protected.HandleFunc("/storage/buckets/{bucket}/upload-url", a.storageHandlers.HandleGenerateUploadURL).Methods("POST", "OPTIONS")
	protected.HandleFunc("/storage/buckets/{bucket}/objects/{id}", a.storageHandlers.HandleGetObject).Methods("GET", "OPTIONS")
	protected.HandleFunc("/storage/buckets/{bucket}/objects/{id}", a.storageHandlers.HandleDeleteObject).Methods("DELETE", "OPTIONS")
	protected.HandleFunc("/storage/buckets/{bucket}/objects/{id}/download", a.storageHandlers.HandleDownloadObject).Methods("GET", "OPTIONS")
	protected.HandleFunc("/storage/buckets/{bucket}/objects/{id}/download-url", a.storageHandlers.HandleGenerateDownloadURL).Methods("GET", "OPTIONS")
	protected.HandleFunc("/storage/buckets/{bucket}/objects/{id}/rename", a.storageHandlers.HandleRenameObject).Methods("PATCH", "OPTIONS")
	protected.HandleFunc("/storage/buckets/{bucket}/objects/{id}/metadata", a.storageHandlers.HandleUpdateObjectMetadata).Methods("PATCH", "OPTIONS")
	protected.HandleFunc("/storage/buckets/{bucket}/folders", a.storageHandlers.HandleCreateFolder).Methods("POST", "OPTIONS")
	
	// Storage utilities
	protected.HandleFunc("/storage/search", a.storageHandlers.HandleSearchStorageObjects).Methods("GET", "OPTIONS")
	protected.HandleFunc("/storage/recently-viewed", a.storageHandlers.HandleGetRecentlyViewed).Methods("GET", "OPTIONS")
	protected.HandleFunc("/storage/items/{id}/last-viewed", a.storageHandlers.HandleUpdateLastViewed).Methods("POST", "OPTIONS")
	protected.HandleFunc("/storage/quota", a.storageHandlers.HandleGetStorageQuota).Methods("GET", "OPTIONS")
	protected.HandleFunc("/storage/stats", a.storageHandlers.HandleGetStorageStats).Methods("GET", "OPTIONS")

	// ---- Settings (read-only for users) ----
	
	protected.HandleFunc("/settings", settings.HandleGetSettings(a.SettingsService)).Methods("GET", "OPTIONS")
	protected.HandleFunc("/settings/{key}", settings.HandleGetSetting(a.SettingsService)).Methods("GET", "OPTIONS")

	// ---- Dashboard (available to all authenticated users) ----
	
	protected.HandleFunc("/dashboard/stats", system.HandleGetDashboardStats(
		a.UserService,
		a.StorageService,
		a.DatabaseService,
	)).Methods("GET", "OPTIONS")

	// ==================================
	// ADMIN ROUTES (admin role required)
	// ==================================
	
	admin := apiRouter.PathPrefix("/admin").Subrouter()
	admin.Use(middleware.AuthMiddleware(a.AuthService))
	admin.Use(middleware.AdminMiddleware(a.IAMService))
	
	// ---- User Management ----
	admin.HandleFunc("/users", users.HandleGetUsers(a.UserService)).Methods("GET", "OPTIONS")
	admin.HandleFunc("/users/{id}", users.HandleGetUser(a.UserService)).Methods("GET", "OPTIONS")
	admin.HandleFunc("/users/{id}", users.HandleUpdateUser(a.UserService)).Methods("PATCH", "OPTIONS")
	admin.HandleFunc("/users/{id}", users.HandleDeleteUser(a.UserService)).Methods("DELETE", "OPTIONS")
	
	// ---- Database Management ----
	admin.HandleFunc("/database/info", dbhandlers.HandleGetDatabaseInfo(a.DatabaseService)).Methods("GET", "OPTIONS")
	admin.HandleFunc("/database/tables", dbhandlers.HandleGetDatabaseTables(a.DatabaseService)).Methods("GET", "OPTIONS")
	admin.HandleFunc("/database/tables/{table}/columns", dbhandlers.HandleGetTableColumns(a.DatabaseService)).Methods("GET", "OPTIONS")
	admin.HandleFunc("/database/query", dbhandlers.HandleExecuteQuery(a.DatabaseService)).Methods("POST", "OPTIONS")
	
	// ---- Logs ----
	admin.HandleFunc("/logs", logs.HandleGetLogs(a.LogsService)).Methods("GET", "OPTIONS")
	admin.HandleFunc("/logs/requests", logs.HandleGetRequestLogs(a.LogsService)).Methods("GET", "OPTIONS")
	admin.HandleFunc("/logs/stats", logs.HandleGetLogStats(a.LogsService)).Methods("GET", "OPTIONS")
	admin.HandleFunc("/logs/details", logs.HandleGetLogDetails(a.LogsService)).Methods("GET", "OPTIONS")
	admin.HandleFunc("/logs/export", logs.HandleExportLogs(a.LogsService)).Methods("GET", "OPTIONS")
	admin.HandleFunc("/logs/clear", logs.HandleClearLogs(a.LogsService)).Methods("POST", "OPTIONS")
	
	// ---- System Metrics ----
	admin.HandleFunc("/system/metrics", system.HandleGetSystemMetrics()).Methods("GET", "OPTIONS")
	
	// ---- Settings Management ----
	admin.HandleFunc("/settings", settings.HandleUpdateSettings(a.SettingsService)).Methods("PATCH", "OPTIONS")
	admin.HandleFunc("/settings", settings.HandleSetSetting(a.SettingsService)).Methods("POST", "OPTIONS")
	admin.HandleFunc("/settings/reset", settings.HandleResetSettings(a.SettingsService)).Methods("POST", "OPTIONS")
	
	// ---- Storage Admin ----
	admin.HandleFunc("/storage/stats", a.storageHandlers.HandleGetAdminStorageStats).Methods("GET", "OPTIONS")
	
	// ---- IAM (Identity & Access Management) ----
	routes.RegisterIAMRoutes(admin, a.IAMService)

	// Initialize handlers if needed
	if a.analyticsHandlers == nil {
		a.analyticsHandlers = analytics.NewAnalyticsHandlers(a.DB)
		if err := a.analyticsHandlers.InitializeSchema(); err != nil {
			fmt.Printf("Failed to initialize analytics schema: %v\n", err)
		}
	}
	
	if a.productHandlers == nil {
		a.productHandlers = products.NewProductsExtensionHandlersWithDB(a.DB.DB)
	}

	// ==================================
	// EXTENSIONS - with admin sub-routes
	// ==================================
	
	// Extensions management (admin only)
	admin.HandleFunc("/extensions", extensions.HandleGetExtensions()).Methods("GET", "OPTIONS")
	admin.HandleFunc("/extensions/manage", extensions.HandleExtensionsManagement()).Methods("GET", "OPTIONS")
	admin.HandleFunc("/extensions/{name}/toggle", extensions.HandleToggleExtension()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/extensions/status", extensions.HandleExtensionsStatus()).Methods("GET", "OPTIONS")
	
	// ---- Analytics Extension ----
	
	// User endpoints
	protected.HandleFunc("/ext/analytics/track", a.analyticsHandlers.HandleTrack()).Methods("POST", "OPTIONS")
	protected.HandleFunc("/ext/analytics/stats", a.analyticsHandlers.HandleStats()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/ext/analytics/pageviews", a.analyticsHandlers.HandlePageViews()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/ext/analytics/daily", a.analyticsHandlers.HandleDailyStats()).Methods("GET", "OPTIONS")
	
	// Admin endpoints
	admin.HandleFunc("/ext/analytics/dashboard", extensions.HandleAnalyticsDashboard()).Methods("GET", "OPTIONS")
	admin.HandleFunc("/ext/analytics/export", extensions.HandleAnalyticsExport()).Methods("GET", "OPTIONS")
	admin.HandleFunc("/ext/analytics/clear", extensions.HandleAnalyticsClear()).Methods("POST", "OPTIONS")
	
	// ---- Products Extension ----
	
	// User endpoints (product browsing and usage)
	protected.HandleFunc("/ext/products/products", a.productHandlers.HandleProductsList()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/ext/products/groups", a.productHandlers.HandleListGroups()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/ext/products/groups", a.productHandlers.HandleCreateGroup()).Methods("POST", "OPTIONS")
	protected.HandleFunc("/ext/products/groups/{id}", a.productHandlers.HandleGetGroup()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/ext/products/groups/{id}", a.productHandlers.HandleUpdateGroup()).Methods("PUT", "OPTIONS")
	protected.HandleFunc("/ext/products/groups/{id}", a.productHandlers.HandleDeleteGroup()).Methods("DELETE", "OPTIONS")
	protected.HandleFunc("/ext/products/groups/{id}/products", a.productHandlers.HandleGroupProducts()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/ext/products/calculate-price", a.productHandlers.HandleCalculatePrice()).Methods("POST", "OPTIONS")
	
	// Admin endpoints (product management)
	admin.HandleFunc("/ext/products/products", a.productHandlers.HandleProductsCreate()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/ext/products/products/{id}", extensions.HandleProductsUpdate()).Methods("PUT", "OPTIONS")
	admin.HandleFunc("/ext/products/products/{id}", extensions.HandleProductsDelete()).Methods("DELETE", "OPTIONS")
	admin.HandleFunc("/ext/products/stats", a.productHandlers.HandleProductsStats()).Methods("GET", "OPTIONS")
	
	// Admin configuration endpoints
	admin.HandleFunc("/ext/products/variables", a.productHandlers.HandleListVariables()).Methods("GET", "OPTIONS")
	admin.HandleFunc("/ext/products/variables", a.productHandlers.HandleCreateVariable()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/ext/products/variables/{id}", a.productHandlers.HandleUpdateVariable()).Methods("PUT", "OPTIONS")
	admin.HandleFunc("/ext/products/variables/{id}", a.productHandlers.HandleDeleteVariable()).Methods("DELETE", "OPTIONS")
	
	admin.HandleFunc("/ext/products/group-types", a.productHandlers.HandleListGroupTypes()).Methods("GET", "OPTIONS")
	admin.HandleFunc("/ext/products/group-types", a.productHandlers.HandleCreateGroupType()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/ext/products/group-types/{id}", a.productHandlers.HandleUpdateGroupType()).Methods("PUT", "OPTIONS")
	admin.HandleFunc("/ext/products/group-types/{id}", a.productHandlers.HandleDeleteGroupType()).Methods("DELETE", "OPTIONS")
	
	admin.HandleFunc("/ext/products/product-types", a.productHandlers.HandleListProductTypes()).Methods("GET", "OPTIONS")
	admin.HandleFunc("/ext/products/product-types", a.productHandlers.HandleCreateProductType()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/ext/products/product-types/{id}", a.productHandlers.HandleUpdateProductType()).Methods("PUT", "OPTIONS")
	admin.HandleFunc("/ext/products/product-types/{id}", a.productHandlers.HandleDeleteProductType()).Methods("DELETE", "OPTIONS")
	
	admin.HandleFunc("/ext/products/pricing-templates", a.productHandlers.HandleListPricingTemplates()).Methods("GET", "OPTIONS")
	admin.HandleFunc("/ext/products/pricing-templates", a.productHandlers.HandleCreatePricingTemplate()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/ext/products/pricing-templates/{id}", a.productHandlers.HandleUpdatePricingTemplate()).Methods("PUT", "OPTIONS")
	admin.HandleFunc("/ext/products/pricing-templates/{id}", a.productHandlers.HandleDeletePricingTemplate()).Methods("DELETE", "OPTIONS")
	
	// ---- Webhooks Extension ----
	
	// User endpoints (view webhooks)
	protected.HandleFunc("/ext/webhooks/webhooks", extensions.HandleWebhooksList()).Methods("GET", "OPTIONS")
	
	// Admin endpoints (manage webhooks)
	admin.HandleFunc("/ext/webhooks/dashboard", extensions.HandleWebhooksDashboard()).Methods("GET", "OPTIONS")
	admin.HandleFunc("/ext/webhooks/webhooks", extensions.HandleWebhooksCreate()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/ext/webhooks/webhooks/{id}/toggle", extensions.HandleWebhooksToggle()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/ext/webhooks/webhooks/{id}", extensions.HandleWebhooksDelete()).Methods("DELETE", "OPTIONS")
	
	// ---- Hugo Extension (all admin) ----
	
	admin.HandleFunc("/ext/hugo/sites", extensions.HandleHugoSitesList()).Methods("GET", "OPTIONS")
	admin.HandleFunc("/ext/hugo/sites", extensions.HandleHugoSitesCreate()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/ext/hugo/sites/{id}", extensions.HandleHugoSitesDelete()).Methods("DELETE", "OPTIONS")
	admin.HandleFunc("/ext/hugo/sites/{id}/build", extensions.HandleHugoSitesBuild()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/ext/hugo/sites/{id}/files", extensions.HandleHugoSiteFiles()).Methods("GET", "OPTIONS")
	admin.HandleFunc("/ext/hugo/sites/{id}/files/read", extensions.HandleHugoFileRead()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/ext/hugo/sites/{id}/files/save", extensions.HandleHugoFileSave()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/ext/hugo/sites/{id}/files/create", extensions.HandleHugoFileCreate()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/ext/hugo/sites/{id}/files/delete", extensions.HandleHugoFileDelete()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/ext/hugo/stats", extensions.HandleHugoStats()).Methods("GET", "OPTIONS")
	
	// ---- Cloud Storage Extension ----
	
	// User endpoints
	protected.HandleFunc("/ext/cloudstorage/shares", a.sharesHandler.HandleShares()).Methods("GET", "POST", "OPTIONS")
	protected.HandleFunc("/ext/cloudstorage/shares/{id}", a.sharesHandler.HandleShareByID()).Methods("GET", "DELETE", "OPTIONS")
	
	// Admin endpoints
	admin.HandleFunc("/ext/cloudstorage/providers", extensions.HandleCloudStorageProviders()).Methods("GET", "OPTIONS")
	admin.HandleFunc("/ext/cloudstorage/providers", extensions.HandleCloudStorageAddProvider()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/ext/cloudstorage/activity", extensions.HandleCloudStorageActivity()).Methods("GET", "OPTIONS")
	admin.HandleFunc("/ext/cloudstorage/stats", extensions.HandleCloudStorageStats()).Methods("GET", "OPTIONS")
}
func (a *API) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	a.Router.ServeHTTP(w, r)
}
