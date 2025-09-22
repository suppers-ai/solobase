package router

import (
	"log"
	"net/http"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/database"
	"github.com/suppers-ai/solobase/extensions/core"
	"github.com/suppers-ai/solobase/extensions/official/cloudstorage"
	"github.com/suppers-ai/solobase/extensions/official/legalpages"
	productsext "github.com/suppers-ai/solobase/extensions/official/products"
	"github.com/suppers-ai/solobase/internal/api/handlers/auth"
	"github.com/suppers-ai/solobase/internal/api/handlers/custom_tables"
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
	Router               *mux.Router
	DB                   *database.DB
	AuthService          *services.AuthService
	UserService          *services.UserService
	StorageService       *services.StorageService
	DatabaseService      *services.DatabaseService
	SettingsService      *services.SettingsService
	LogsService          *services.LogsService
	IAMService           *iam.Service
	CustomTablesService  *services.CustomTablesService
	storageHandlers      *storage.StorageHandlers
	sharesHandler        *shares.SharesHandler
	productHandlers      *products.ProductsExtensionHandlers
	customTablesHandler  *custom_tables.Handler
	ExtensionRegistry    *core.ExtensionRegistry
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

	// Initialize custom tables service and handler
	api.CustomTablesService = services.NewCustomTablesService(db.DB)
	api.customTablesHandler = custom_tables.NewHandler(api.CustomTablesService, db.DB)

	api.setupRoutesWithAdmin()
	return api
}

// setupRoutesWithAdmin sets up all routes with proper admin namespace
func (a *API) setupRoutesWithAdmin() {
	apiRouter := a.Router

	// Apply security middleware first
	apiRouter.Use(middleware.SecurityHeadersMiddleware)
	apiRouter.Use(middleware.ReadOnlyMiddleware)

	// Apply CORS and Metrics middleware to all API routes
	apiRouter.Use(middleware.CORSMiddleware)
	apiRouter.Use(middleware.MetricsMiddleware)

	// Apply rate limiting for demo mode
	rateLimiter := middleware.NewRateLimitMiddleware()
	apiRouter.Use(rateLimiter.Middleware)

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

	// Payment provider webhook endpoint (public, verified by signature)
	apiRouter.HandleFunc("/ext/products/webhooks", a.productHandlers.HandleWebhook()).Methods("POST")

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

	// ---- Custom Tables Management ----
	admin.HandleFunc("/custom-tables", a.customTablesHandler.CreateTable).Methods("POST", "OPTIONS")
	admin.HandleFunc("/custom-tables", a.customTablesHandler.ListTables).Methods("GET", "OPTIONS")
	admin.HandleFunc("/custom-tables/{name}", a.customTablesHandler.GetTableSchema).Methods("GET", "OPTIONS")
	admin.HandleFunc("/custom-tables/{name}", a.customTablesHandler.AlterTable).Methods("PUT", "OPTIONS")
	admin.HandleFunc("/custom-tables/{name}", a.customTablesHandler.DropTable).Methods("DELETE", "OPTIONS")
	admin.HandleFunc("/custom-tables/{name}/migrations", a.customTablesHandler.GetMigrationHistory).Methods("GET", "OPTIONS")

	// Custom table data operations
	admin.HandleFunc("/custom-tables/{name}/data", a.customTablesHandler.InsertData).Methods("POST", "OPTIONS")
	admin.HandleFunc("/custom-tables/{name}/data", a.customTablesHandler.QueryData).Methods("GET", "OPTIONS")
	admin.HandleFunc("/custom-tables/{name}/data/{id}", a.customTablesHandler.GetRecord).Methods("GET", "OPTIONS")
	admin.HandleFunc("/custom-tables/{name}/data/{id}", a.customTablesHandler.UpdateRecord).Methods("PUT", "OPTIONS")
	admin.HandleFunc("/custom-tables/{name}/data/{id}", a.customTablesHandler.DeleteRecord).Methods("DELETE", "OPTIONS")

	// Initialize handlers if needed

	if a.productHandlers == nil {
		// Get the products extension from the registry instead of creating a new one
		if ext, exists := a.ExtensionRegistry.Get("products"); exists {
			if productsExt, ok := ext.(*productsext.ProductsExtension); ok {
				a.productHandlers = products.NewProductsExtensionHandlersWithExtension(productsExt)
			} else {
				// Fallback to empty handler
				a.productHandlers = products.NewProductsExtensionHandlers()
			}
		} else {
			// Create empty handlers
			a.productHandlers = products.NewProductsExtensionHandlers()
		}
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
	// Analytics routes are now handled by the analytics extension via the extension registry
	// The extension automatically registers its routes under /ext/analytics/
	
	// ---- Products Extension ----
	
	// User endpoints (product browsing and usage)
	protected.HandleFunc("/ext/products/products", a.productHandlers.HandleProductsList()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/ext/products/products", a.productHandlers.HandleProductsCreate()).Methods("POST", "OPTIONS")
	protected.HandleFunc("/ext/products/products/{id}", extensions.HandleProductsDelete()).Methods("DELETE", "OPTIONS")
	protected.HandleFunc("/ext/products/groups", a.productHandlers.HandleListGroups()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/ext/products/groups", a.productHandlers.HandleCreateGroup()).Methods("POST", "OPTIONS")
	protected.HandleFunc("/ext/products/groups/{id}", a.productHandlers.HandleGetGroup()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/ext/products/groups/{id}", a.productHandlers.HandleUpdateGroup()).Methods("PUT", "OPTIONS")
	protected.HandleFunc("/ext/products/groups/{id}", a.productHandlers.HandleDeleteGroup()).Methods("DELETE", "OPTIONS")
	protected.HandleFunc("/ext/products/groups/{id}/products", a.productHandlers.HandleGroupProducts()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/ext/products/calculate-price", a.productHandlers.HandleCalculatePrice()).Methods("POST", "OPTIONS")

	// Purchase endpoints
	protected.HandleFunc("/ext/products/purchase", a.productHandlers.HandleCreatePurchase()).Methods("POST", "OPTIONS")
	protected.HandleFunc("/ext/products/purchases", a.productHandlers.HandleListPurchases()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/ext/products/purchases/stats", a.productHandlers.HandlePurchaseStats()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/ext/products/purchases/{id}", a.productHandlers.HandleGetPurchase()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/ext/products/purchases/{id}/cancel", a.productHandlers.HandleCancelPurchase()).Methods("POST", "OPTIONS")

	// User endpoints that also need to be available (read-only access to types)
	protected.HandleFunc("/ext/products/group-types", a.productHandlers.HandleListGroupTypes()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/ext/products/product-types", a.productHandlers.HandleListProductTypes()).Methods("GET", "OPTIONS")
	protected.HandleFunc("/ext/products/variables", a.productHandlers.HandleListVariables()).Methods("GET", "OPTIONS")
	
	// Admin endpoints (product management)
	admin.HandleFunc("/ext/products/products", a.productHandlers.HandleProductsCreate()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/ext/products/products/{id}", extensions.HandleProductsUpdate()).Methods("PUT", "OPTIONS")
	admin.HandleFunc("/ext/products/products/{id}", extensions.HandleProductsDelete()).Methods("DELETE", "OPTIONS")
	admin.HandleFunc("/ext/products/stats", a.productHandlers.HandleProductsStats()).Methods("GET", "OPTIONS")
	
	// Admin configuration endpoints
	admin.HandleFunc("/ext/products/provider/status", a.productHandlers.HandleProviderStatus()).Methods("GET", "OPTIONS")
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

	// Admin purchase management endpoints
	admin.HandleFunc("/ext/products/purchases", a.productHandlers.HandleListAllPurchases()).Methods("GET", "OPTIONS")
	admin.HandleFunc("/ext/products/purchases/{id}/refund", a.productHandlers.HandleRefundPurchase()).Methods("POST", "OPTIONS")
	admin.HandleFunc("/ext/products/purchases/{id}/approve", a.productHandlers.HandleApprovePurchase()).Methods("POST", "OPTIONS")
	
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
	// CloudStorage extension routes need to be registered directly with the Gorilla Mux router
	// Get the CloudStorage extension from the registry and create handler wrappers

	if a.ExtensionRegistry != nil {
		// Register Legal Pages extension routes
		if ext, ok := a.ExtensionRegistry.Get("legalpages"); ok {
			log.Println("Legal Pages extension found in registry")
			if legalPagesExt, ok := ext.(*legalpages.LegalPagesExtension); ok && legalPagesExt != nil {
				log.Println("Legal Pages extension cast successful")
				// Get handlers
				handlers := legalPagesExt.GetHandlers()
				log.Printf("Legal Pages handlers: %v\n", handlers)
				if handlers != nil {
					log.Println("Registering Legal Pages routes...")
					// Admin API routes
					log.Println("Registering route: /ext/legalpages/documents")
					admin.HandleFunc("/ext/legalpages/documents", handlers.HandleGetDocuments).Methods("GET", "OPTIONS")
					log.Println("Registering route: /ext/legalpages/documents/{type}")
					admin.HandleFunc("/ext/legalpages/documents/{type}", handlers.HandleGetDocument).Methods("GET", "OPTIONS")
					admin.HandleFunc("/ext/legalpages/documents/{type}", handlers.HandleSaveDocument).Methods("POST", "OPTIONS")
					admin.HandleFunc("/ext/legalpages/documents/{type}/publish", handlers.HandlePublishDocument).Methods("POST", "OPTIONS")
					admin.HandleFunc("/ext/legalpages/documents/{type}/preview", handlers.HandlePreviewDocument).Methods("GET", "OPTIONS")
					admin.HandleFunc("/ext/legalpages/documents/{type}/history", handlers.HandleGetDocumentHistory).Methods("GET", "OPTIONS")

					// Admin UI route
					admin.HandleFunc("/ext/legalpages/admin", legalPagesExt.HandleAdminUI).Methods("GET", "OPTIONS")

					// Public routes (no auth required)
					apiRouter.HandleFunc("/ext/legalpages/terms", handlers.HandlePublicTerms).Methods("GET", "OPTIONS")
					apiRouter.HandleFunc("/ext/legalpages/privacy", handlers.HandlePublicPrivacy).Methods("GET", "OPTIONS")
					log.Println("Legal Pages routes registered successfully")
				} else {
					log.Println("Legal Pages handlers are nil")
				}
			} else {
				log.Println("Failed to cast Legal Pages extension")
			}
		} else {
			log.Println("Legal Pages extension not found in registry")
		}

		if ext, ok := a.ExtensionRegistry.Get("cloudstorage"); ok {
			if cloudStorageExt, ok := ext.(*cloudstorage.CloudStorageExtension); ok {
				// Register CloudStorage routes
				protected.HandleFunc("/ext/cloudstorage/stats", cloudStorageExt.HandleStats()).Methods("GET", "OPTIONS")
				protected.HandleFunc("/ext/cloudstorage/shares", cloudStorageExt.HandleShares()).Methods("GET", "POST", "OPTIONS")
				protected.HandleFunc("/ext/cloudstorage/share/{token}", cloudStorageExt.HandleShareAccess()).Methods("GET", "OPTIONS")
				protected.HandleFunc("/ext/cloudstorage/quota", cloudStorageExt.HandleQuota()).Methods("GET", "PUT", "OPTIONS")
				protected.HandleFunc("/ext/cloudstorage/quotas/user", cloudStorageExt.HandleGetUserQuota()).Methods("GET", "OPTIONS")
				protected.HandleFunc("/ext/cloudstorage/access-logs", cloudStorageExt.HandleAccessLogs()).Methods("GET", "OPTIONS")
				protected.HandleFunc("/ext/cloudstorage/access-stats", cloudStorageExt.HandleAccessStats()).Methods("GET", "OPTIONS")

				// Admin routes
				admin.HandleFunc("/ext/cloudstorage/quotas/roles", cloudStorageExt.HandleRoleQuotas()).Methods("GET", "OPTIONS")
				admin.HandleFunc("/ext/cloudstorage/quotas/roles/{role}", cloudStorageExt.HandleUpdateRoleQuota()).Methods("PUT", "OPTIONS")
				admin.HandleFunc("/ext/cloudstorage/quotas/overrides", cloudStorageExt.HandleUserOverrides()).Methods("GET", "OPTIONS")
				admin.HandleFunc("/ext/cloudstorage/quotas/overrides/{user}", cloudStorageExt.HandleDeleteUserOverride()).Methods("DELETE", "OPTIONS")
				admin.HandleFunc("/ext/cloudstorage/users/search", cloudStorageExt.HandleUserSearch()).Methods("GET", "OPTIONS")
				admin.HandleFunc("/ext/cloudstorage/default-quotas", cloudStorageExt.HandleDefaultQuotas()).Methods("GET", "PUT", "OPTIONS")
			}
		}
	}

}
func (a *API) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	a.Router.ServeHTTP(w, r)
}
