package cloudstorage

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"time"

	"github.com/google/uuid"
	"github.com/suppers-ai/solobase/extensions/core"
	pkgstorage "github.com/suppers-ai/solobase/internal/pkg/storage"
	"gorm.io/gorm"
)

// CloudStorageConfig holds extension-specific configuration
type CloudStorageConfig struct {
	DefaultStorageLimit   int64  // Default storage limit per user in bytes (default: 5GB)
	DefaultBandwidthLimit int64  // Default bandwidth limit per user in bytes (default: 10GB)
	EnableSharing         bool   // Enable file sharing features (default: true)
	EnableAccessLogs      bool   // Enable access logging (default: true)
	EnableQuotas          bool   // Enable storage quotas (default: true)
	BandwidthResetPeriod  string // Period for bandwidth reset: "daily", "weekly", "monthly" (default: "monthly")
}

// CloudStorageExtension provides enhanced cloud storage capabilities
type CloudStorageExtension struct {
	services *core.ExtensionServices
	db       *gorm.DB
	manager  *pkgstorage.Manager
	config   *CloudStorageConfig

	// Core services for extending storage functionality
	shareService     *ShareService
	quotaService     *QuotaService
	accessLogService *AccessLogService
}

// GetQuotaService returns the quota service
func (e *CloudStorageExtension) GetQuotaService() *QuotaService {
	return e.quotaService
}

// GetAccessLogService returns the access log service
func (e *CloudStorageExtension) GetAccessLogService() *AccessLogService {
	return e.accessLogService
}

// Metadata returns extension metadata
func (e *CloudStorageExtension) Metadata() core.ExtensionMetadata {
	return core.ExtensionMetadata{
		Name:         "cloudstorage",
		Version:      "2.0.0",
		Description:  "Enterprise-level storage management with advanced sharing capabilities, granular access control, storage quotas, bandwidth monitoring, and detailed analytics. Create public links, share with specific users, track file access, and manage storage limits.",
		Author:       "Solobase Team",
		License:      "MIT",
		Tags:         []string{"storage", "sharing", "quotas", "analytics", "access-control", "bandwidth", "file-management"},
		Homepage:     "https://github.com/suppers-ai/solobase",
		MinVersion:   "1.0.0",
		MaxVersion:   "3.0.0",
		Dependencies: []string{"storage", "auth"},
	}
}

// Initialize sets up the extension
func (e *CloudStorageExtension) Initialize(ctx context.Context, services *core.ExtensionServices) error {
	e.services = services

	// Log initialization
	if services != nil {
		services.Logger().Info(ctx, "CloudStorage extension initializing")

		// Initialize storage manager if we have storage service
		if services.Storage() != nil {
			// TODO: Get storage manager from services.Storage()
			// For now, we'll skip ShareService initialization
			// e.manager = services.Storage().GetManager()
			// e.shareService = NewShareService(e.db, e.manager)
		}

		// Initialize quota service
		if e.config.EnableQuotas {
			e.quotaService = NewQuotaService(e.db)

			// Migrate quota tables
			if err := e.db.AutoMigrate(&RoleQuota{}, &UserQuotaOverride{}, &StorageQuota{}); err != nil {
				services.Logger().Error(ctx, fmt.Sprintf("Failed to migrate quota tables: %v", err))
				return err
			}

			// Initialize default quotas for system roles
			if err := e.quotaService.InitializeDefaultQuotas(); err != nil {
				services.Logger().Error(ctx, fmt.Sprintf("Failed to initialize default quotas: %v", err))
				// Don't fail initialization, just log the error
			}
		}

		// Initialize default settings for this extension
		// This setting controls whether storage usage is shown in user profile
		// We set it to true by default when the extension is initialized
		// Note: In production, this would be done through proper settings service
	}

	return nil
}

// Start begins the extension's operations
func (e *CloudStorageExtension) Start(ctx context.Context) error {
	return nil
}

// Stop gracefully shuts down the extension
func (e *CloudStorageExtension) Stop(ctx context.Context) error {
	return nil
}

// Health returns the health status
func (e *CloudStorageExtension) Health(ctx context.Context) (*core.HealthStatus, error) {
	// Check if we have a manager configured
	if e.manager == nil {
		return &core.HealthStatus{
			Status:      "healthy",
			Message:     "CloudStorage tables ready (storage manager not yet initialized)",
			LastChecked: time.Now(),
		}, nil
	}

	// Check if we can list buckets
	_, err := e.manager.ListBuckets(ctx)
	if err != nil {
		return &core.HealthStatus{
			Status:      "unhealthy",
			Message:     "Storage provider error: " + err.Error(),
			LastChecked: time.Now(),
		}, nil
	}

	return &core.HealthStatus{
		Status:      "healthy",
		Message:     "CloudStorage is operational",
		LastChecked: time.Now(),
	}, nil
}

// RegisterRoutes registers HTTP routes
func (e *CloudStorageExtension) RegisterRoutes(router core.ExtensionRouter) error {
	// Core storage routes
	router.HandleFunc("/buckets", e.handleBuckets)
	router.HandleFunc("/upload", e.handleUpload)
	router.HandleFunc("/download", e.handleDownload)
	router.HandleFunc("/stats", e.handleStats)

	// Sharing routes
	router.HandleFunc("/shares", e.handleShares)
	router.HandleFunc("/share/*", e.handleShareAccess) // Public share access

	// Quota management routes
	router.HandleFunc("/quota", e.handleQuota)
	router.HandleFunc("/quotas/roles", e.handleRoleQuotas)
	router.HandleFunc("/quotas/roles/*", e.handleUpdateRoleQuota)
	router.HandleFunc("/quotas/overrides", e.handleUserOverrides)
	router.HandleFunc("/quotas/overrides/*", e.handleDeleteUserOverride)
	router.HandleFunc("/quotas/user", e.handleGetUserQuota)

	// Access logging routes
	router.HandleFunc("/access-logs", e.handleAccessLogs)
	router.HandleFunc("/access-stats", e.handleAccessStats)

	// Admin routes
	router.HandleFunc("/users/search", e.handleUserSearch)

	// Log that routes were registered
	fmt.Printf("CloudStorage extension routes registered successfully\n")

	return nil
}

// RegisterMiddleware returns middleware registrations
func (e *CloudStorageExtension) RegisterMiddleware() []core.MiddlewareRegistration {
	return nil
}

// RegisterHooks returns hook registrations
func (e *CloudStorageExtension) RegisterHooks() []core.HookRegistration {
	hooks := []core.HookRegistration{}

	// Register user lifecycle hooks
	hooks = append(hooks, core.HookRegistration{
		Extension: "cloudstorage",
		Name:      "setup_user_resources",
		Type:      core.HookPostLogin,
		Priority:  10,
		Handler:   e.setupUserResourcesHook,
	})

	// Check share permissions (including inheritance) before downloads
	if e.config.EnableSharing {
		hooks = append(hooks, core.HookRegistration{
			Extension: "cloudstorage",
			Name:      "check_share_permissions",
			Type:      core.HookBeforeDownload,
			Priority:  5, // Run before quota checks
			Handler:   e.checkSharePermissionsHook,
		})
	}

	// Only register hooks if quotas are enabled
	if e.config.EnableQuotas {
		// Before upload - check storage quota
		hooks = append(hooks, core.HookRegistration{
			Extension: "cloudstorage",
			Name:      "check_storage_quota",
			Type:      core.HookBeforeUpload,
			Priority:  10,
			Handler:   e.checkStorageQuotaHook,
		})

		// After upload - update storage usage
		hooks = append(hooks, core.HookRegistration{
			Extension: "cloudstorage",
			Name:      "update_storage_usage",
			Type:      core.HookAfterUpload,
			Priority:  10,
			Handler:   e.updateStorageUsageHook,
		})

		// After download - update bandwidth usage
		hooks = append(hooks, core.HookRegistration{
			Extension: "cloudstorage",
			Name:      "update_bandwidth_usage",
			Type:      core.HookAfterDownload,
			Priority:  10,
			Handler:   e.updateBandwidthUsageHook,
		})
	}

	// Access logging hooks
	if e.config.EnableAccessLogs {
		hooks = append(hooks, core.HookRegistration{
			Extension: "cloudstorage",
			Name:      "log_upload_access",
			Type:      core.HookAfterUpload,
			Priority:  20,
			Handler:   e.logUploadAccessHook,
		})

		hooks = append(hooks, core.HookRegistration{
			Extension: "cloudstorage",
			Name:      "log_download_access",
			Type:      core.HookAfterDownload,
			Priority:  20,
			Handler:   e.logDownloadAccessHook,
		})
	}

	return hooks
}

// SyncRoleQuota syncs quota when a role is created or updated in IAM
func (e *CloudStorageExtension) SyncRoleQuota(ctx context.Context, roleName string, roleID string) error {
	if e.quotaService == nil {
		return nil // Quotas not enabled
	}

	return e.quotaService.SyncRoleQuotaFromIAM(ctx, roleName, roleID)
}

// RegisterTemplates returns template registrations
func (e *CloudStorageExtension) RegisterTemplates() []core.TemplateRegistration {
	return []core.TemplateRegistration{
		{
			Name:    "cloudstorage-dashboard",
			Content: []byte(dashboardHTML),
			Path:    "/dashboard",
		},
	}
}

// RegisterStaticAssets returns static asset registrations
func (e *CloudStorageExtension) RegisterStaticAssets() []core.StaticAssetRegistration {
	return nil
}

// ConfigSchema returns the configuration schema
func (e *CloudStorageExtension) ConfigSchema() json.RawMessage {
	schema := `{
		"type": "object",
		"properties": {
			"defaultStorageLimit": {"type": "integer", "description": "Default storage limit in bytes"},
			"defaultBandwidthLimit": {"type": "integer", "description": "Default bandwidth limit in bytes"},
			"enableSharing": {"type": "boolean", "default": true},
			"enableAccessLogs": {"type": "boolean", "default": true},
			"enableQuotas": {"type": "boolean", "default": true},
			"bandwidthResetPeriod": {"type": "string", "enum": ["daily", "weekly", "monthly"], "default": "monthly"}
		}
	}`
	return json.RawMessage(schema)
}

// ValidateConfig validates configuration
func (e *CloudStorageExtension) ValidateConfig(config json.RawMessage) error {
	var cfg CloudStorageConfig
	return json.Unmarshal(config, &cfg)
}

// ApplyConfig applies configuration
func (e *CloudStorageExtension) ApplyConfig(config json.RawMessage) error {
	var cfg CloudStorageConfig
	if err := json.Unmarshal(config, &cfg); err != nil {
		return err
	}
	e.config = &cfg
	return nil
}

// DatabaseSchema returns the database schema name
func (e *CloudStorageExtension) DatabaseSchema() string {
	return "ext_cloudstorage"
}

// RequiredPermissions returns required permissions
func (e *CloudStorageExtension) RequiredPermissions() []core.Permission {
	return []core.Permission{
		{
			Name:        "cloudstorage.admin",
			Description: "Full cloud storage administration",
			Resource:    "cloudstorage",
			Actions:     []string{"create", "read", "update", "delete"},
		},
		{
			Name:        "cloudstorage.upload",
			Description: "Upload files to cloud storage",
			Resource:    "cloudstorage",
			Actions:     []string{"create", "upload"},
		},
		{
			Name:        "cloudstorage.download",
			Description: "Download files from cloud storage",
			Resource:    "cloudstorage",
			Actions:     []string{"read", "download"},
		},
	}
}

// NewCloudStorageExtension creates a new extension instance
func NewCloudStorageExtension(config *CloudStorageConfig) core.Extension {
	if config == nil {
		config = &CloudStorageConfig{
			DefaultStorageLimit:   5368709120,  // 5GB default
			DefaultBandwidthLimit: 10737418240, // 10GB default
			EnableSharing:         true,
			EnableAccessLogs:      true,
			EnableQuotas:          true,
			BandwidthResetPeriod:  "monthly",
		}
	}

	return &CloudStorageExtension{
		config: config,
	}
}

// NewCloudStorageExtensionWithDB creates a new extension instance with database
func NewCloudStorageExtensionWithDB(db *gorm.DB, config *CloudStorageConfig) *CloudStorageExtension {
	if config == nil {
		config = &CloudStorageConfig{
			DefaultStorageLimit:   5368709120,  // 5GB default
			DefaultBandwidthLimit: 10737418240, // 10GB default
			EnableSharing:         true,
			EnableAccessLogs:      true,
			EnableQuotas:          true,
			BandwidthResetPeriod:  "monthly",
		}
	}

	ext := &CloudStorageExtension{
		db:     db,
		config: config,
	}

	if db != nil {
		ext.initializeServices()
	}

	return ext
}

// SetDatabase sets the database and initializes services
func (e *CloudStorageExtension) SetDatabase(db *gorm.DB) {
	e.db = db
	e.initializeServices()

	// Run auto-migration for CloudStorage models (tables have prefix in TableName methods)
	if err := e.db.AutoMigrate(
		&StorageShare{},
		&StorageAccessLog{},
		&StorageQuota{},
	); err != nil {
		// Log error but don't fail
		return
	}

	// Initialize extension settings
	e.initializeExtensionSettings()
}

// initializeExtensionSettings sets up default settings for this extension
func (e *CloudStorageExtension) initializeExtensionSettings() {
	if e.db == nil {
		return
	}

	// Check if the setting already exists
	var count int64
	e.db.Table("settings").Where("key = ?", "ext_cloudstorage_profile_show_usage").Count(&count)

	// If it doesn't exist, create it with default value of true
	if count == 0 {
		setting := map[string]interface{}{
			"id":    uuid.New().String(),
			"key":   "ext_cloudstorage_profile_show_usage",
			"value": "true",
			"type":  "bool",
		}
		e.db.Table("settings").Create(&setting)
	}
}

// initializeServices initializes all services
func (e *CloudStorageExtension) initializeServices() {
	if e.db == nil {
		return
	}

	// Initialize services that depend on the database
	// Note: ShareService requires a storage manager which we don't have access to yet
	// This will be properly initialized when the Initialize method is called with services
	e.quotaService = NewQuotaService(e.db)
	e.accessLogService = NewAccessLogService(e.db)
}

// Public handler methods for router registration
func (e *CloudStorageExtension) HandleStats() http.HandlerFunc {
	return e.handleStats
}

func (e *CloudStorageExtension) HandleShares() http.HandlerFunc {
	return e.handleShares
}

func (e *CloudStorageExtension) HandleShareAccess() http.HandlerFunc {
	return e.handleShareAccess
}

func (e *CloudStorageExtension) HandleQuota() http.HandlerFunc {
	return e.handleQuota
}

func (e *CloudStorageExtension) HandleGetUserQuota() http.HandlerFunc {
	return e.handleGetUserQuota
}

func (e *CloudStorageExtension) HandleAccessLogs() http.HandlerFunc {
	return e.handleAccessLogs
}

func (e *CloudStorageExtension) HandleAccessStats() http.HandlerFunc {
	return e.handleAccessStats
}

func (e *CloudStorageExtension) HandleRoleQuotas() http.HandlerFunc {
	return e.handleRoleQuotas
}

func (e *CloudStorageExtension) HandleUpdateRoleQuota() http.HandlerFunc {
	return e.handleUpdateRoleQuota
}

func (e *CloudStorageExtension) HandleUserOverrides() http.HandlerFunc {
	return e.handleUserOverrides
}

func (e *CloudStorageExtension) HandleDeleteUserOverride() http.HandlerFunc {
	return e.handleDeleteUserOverride
}

func (e *CloudStorageExtension) HandleUserSearch() http.HandlerFunc {
	return e.handleUserSearch
}

func (e *CloudStorageExtension) HandleDefaultQuotas() http.HandlerFunc {
	return e.handleDefaultQuotas
}
