package cloudstorage

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"net/http"

	"github.com/suppers-ai/solobase/extensions/core"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	pkgstorage "github.com/suppers-ai/solobase/internal/pkg/storage"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
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
	sqlDB    *sql.DB
	queries  *db.Queries
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
		}

		// Initialize quota service
		if e.config.EnableQuotas && e.sqlDB != nil {
			e.quotaService = NewQuotaService(e.sqlDB)

			// Run schema migrations using raw SQL
			if err := e.runSchemaMigrations(ctx); err != nil {
				services.Logger().Error(ctx, fmt.Sprintf("Failed to migrate quota tables: %v", err))
				return err
			}

			// Initialize default quotas for system roles
			if err := e.quotaService.InitializeDefaultQuotas(); err != nil {
				services.Logger().Error(ctx, fmt.Sprintf("Failed to initialize default quotas: %v", err))
				// Don't fail initialization, just log the error
			}
		}
	}

	return nil
}

// runSchemaMigrations creates the required tables if they don't exist
func (e *CloudStorageExtension) runSchemaMigrations(ctx context.Context) error {
	if e.sqlDB == nil {
		return nil
	}

	schemas := []string{
		`CREATE TABLE IF NOT EXISTS ext_cloudstorage_storage_shares (
			id TEXT PRIMARY KEY,
			object_id TEXT NOT NULL,
			shared_with_user_id TEXT,
			shared_with_email TEXT,
			permission_level TEXT DEFAULT 'read',
			inherit_to_children INTEGER DEFAULT 0,
			share_token TEXT UNIQUE,
			is_public INTEGER DEFAULT 0,
			expires_at DATETIME,
			created_by TEXT NOT NULL,
			created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
		)`,
		`CREATE TABLE IF NOT EXISTS ext_cloudstorage_storage_access_logs (
			id TEXT PRIMARY KEY,
			object_id TEXT NOT NULL,
			user_id TEXT,
			ip_address TEXT,
			action TEXT NOT NULL,
			user_agent TEXT,
			metadata TEXT,
			created_at DATETIME DEFAULT CURRENT_TIMESTAMP
		)`,
		`CREATE TABLE IF NOT EXISTS ext_cloudstorage_storage_quotas (
			id TEXT PRIMARY KEY,
			user_id TEXT UNIQUE NOT NULL,
			max_storage_bytes INTEGER DEFAULT 5368709120,
			max_bandwidth_bytes INTEGER DEFAULT 10737418240,
			storage_used INTEGER DEFAULT 0,
			bandwidth_used INTEGER DEFAULT 0,
			reset_bandwidth_at DATETIME,
			created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
		)`,
		`CREATE TABLE IF NOT EXISTS ext_cloudstorage_role_quotas (
			id TEXT PRIMARY KEY,
			role_id TEXT UNIQUE,
			role_name TEXT UNIQUE,
			max_storage_bytes INTEGER DEFAULT 5368709120,
			max_bandwidth_bytes INTEGER DEFAULT 10737418240,
			max_upload_size INTEGER DEFAULT 104857600,
			max_files_count INTEGER,
			allowed_extensions TEXT,
			blocked_extensions TEXT,
			created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
		)`,
		`CREATE TABLE IF NOT EXISTS ext_cloudstorage_user_quota_overrides (
			id TEXT PRIMARY KEY,
			user_id TEXT UNIQUE NOT NULL,
			max_storage_bytes INTEGER,
			max_bandwidth_bytes INTEGER,
			max_upload_size INTEGER,
			max_files_count INTEGER,
			allowed_extensions TEXT,
			blocked_extensions TEXT,
			reason TEXT,
			expires_at DATETIME,
			created_by TEXT,
			created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
			updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
		)`,
	}

	for _, schema := range schemas {
		if _, err := e.sqlDB.ExecContext(ctx, schema); err != nil {
			return fmt.Errorf("failed to create table: %w", err)
		}
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
			LastChecked: apptime.NowTime(),
		}, nil
	}

	// Check if we can list buckets
	_, err := e.manager.ListBuckets(ctx)
	if err != nil {
		return &core.HealthStatus{
			Status:      "unhealthy",
			Message:     "Storage provider error: " + err.Error(),
			LastChecked: apptime.NowTime(),
		}, nil
	}

	return &core.HealthStatus{
		Status:      "healthy",
		Message:     "CloudStorage is operational",
		LastChecked: apptime.NowTime(),
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
func NewCloudStorageExtensionWithDB(sqlDB *sql.DB, config *CloudStorageConfig) *CloudStorageExtension {
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
		sqlDB:   sqlDB,
		queries: db.New(sqlDB),
		config:  config,
	}

	if sqlDB != nil {
		ext.initializeServices()
	}

	return ext
}

// SetSQLDatabase sets the SQL database for sqlc queries
func (e *CloudStorageExtension) SetSQLDatabase(sqlDB *sql.DB) {
	e.sqlDB = sqlDB
	e.queries = db.New(sqlDB)
	e.initializeServices()

	// Run schema migrations
	if err := e.runSchemaMigrations(context.Background()); err != nil {
		// Log error but don't fail
		return
	}

	// Initialize extension settings
	e.initializeExtensionSettings()
}

// initializeExtensionSettings sets up default settings for this extension
func (e *CloudStorageExtension) initializeExtensionSettings() {
	if e.sqlDB == nil {
		return
	}

	// Check if the setting already exists
	var count int64
	row := e.sqlDB.QueryRow("SELECT COUNT(*) FROM settings WHERE key = ?", "ext_cloudstorage_profile_show_usage")
	if err := row.Scan(&count); err != nil {
		return
	}

	// If it doesn't exist, create it with default value of true
	if count == 0 {
		_, _ = e.sqlDB.Exec(
			"INSERT INTO settings (id, key, value, type) VALUES (?, ?, ?, ?)",
			uuid.New().String(),
			"ext_cloudstorage_profile_show_usage",
			"true",
			"bool",
		)
	}
}

// initializeServices initializes all services
func (e *CloudStorageExtension) initializeServices() {
	if e.sqlDB == nil {
		return
	}

	// Initialize services that depend on the database
	// Note: ShareService requires a storage manager which we don't have access to yet
	// This will be properly initialized when the Initialize method is called with services
	e.quotaService = NewQuotaService(e.sqlDB)
	e.accessLogService = NewAccessLogService(e.sqlDB)
}

// Database helper methods

// getStorageObjectByID retrieves a storage object by ID
func (e *CloudStorageExtension) getStorageObjectByID(id string) (*pkgstorage.StorageObject, error) {
	if e.sqlDB == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	var obj pkgstorage.StorageObject
	row := e.sqlDB.QueryRow(`SELECT id, bucket_name, object_name, parent_folder_id, size, content_type, checksum, metadata, created_at, updated_at, last_viewed, user_id, app_id FROM storage_objects WHERE id = ?`, id)
	var parentFolderID, checksum, metadata, lastViewed, appID sql.NullString
	err := row.Scan(&obj.ID, &obj.BucketName, &obj.ObjectName, &parentFolderID, &obj.Size, &obj.ContentType, &checksum, &metadata, &obj.CreatedAt, &obj.UpdatedAt, &lastViewed, &obj.UserID, &appID)
	if err != nil {
		return nil, err
	}
	if parentFolderID.Valid {
		obj.ParentFolderID = &parentFolderID.String
	}
	if checksum.Valid {
		obj.Checksum = checksum.String
	}
	if metadata.Valid {
		obj.Metadata = metadata.String
	}
	if appID.Valid {
		obj.AppID = &appID.String
	}
	return &obj, nil
}

// countStorageQuotas returns the count of storage quotas
func (e *CloudStorageExtension) countStorageQuotas() (int64, error) {
	if e.sqlDB == nil {
		return 0, fmt.Errorf("database not initialized")
	}
	var count int64
	err := e.sqlDB.QueryRow("SELECT COUNT(*) FROM ext_cloudstorage_storage_quotas").Scan(&count)
	return count, err
}

// getAllStorageQuotas retrieves all storage quotas
func (e *CloudStorageExtension) getAllStorageQuotas() ([]StorageQuota, error) {
	if e.sqlDB == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	rows, err := e.sqlDB.Query("SELECT id, user_id, max_storage_bytes, max_bandwidth_bytes, storage_used, bandwidth_used, reset_bandwidth_at, created_at, updated_at FROM ext_cloudstorage_storage_quotas")
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var quotas []StorageQuota
	for rows.Next() {
		var q StorageQuota
		var resetAt sql.NullTime
		err := rows.Scan(&q.ID, &q.UserID, &q.MaxStorageBytes, &q.MaxBandwidthBytes, &q.StorageUsed, &q.BandwidthUsed, &resetAt, &q.CreatedAt, &q.UpdatedAt)
		if err != nil {
			return nil, err
		}
		if resetAt.Valid {
			q.ResetBandwidthAt = apptime.NewNullTime(resetAt.Time)
		}
		quotas = append(quotas, q)
	}
	return quotas, nil
}

// saveStorageQuota saves or updates a storage quota
func (e *CloudStorageExtension) saveStorageQuota(quota *StorageQuota) error {
	if e.sqlDB == nil {
		return fmt.Errorf("database not initialized")
	}
	_, err := e.sqlDB.Exec(`
		INSERT INTO ext_cloudstorage_storage_quotas (id, user_id, max_storage_bytes, max_bandwidth_bytes, storage_used, bandwidth_used, reset_bandwidth_at, created_at, updated_at)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
		ON CONFLICT(user_id) DO UPDATE SET
			max_storage_bytes = excluded.max_storage_bytes,
			max_bandwidth_bytes = excluded.max_bandwidth_bytes,
			storage_used = excluded.storage_used,
			bandwidth_used = excluded.bandwidth_used,
			reset_bandwidth_at = excluded.reset_bandwidth_at,
			updated_at = excluded.updated_at
	`, quota.ID, quota.UserID, quota.MaxStorageBytes, quota.MaxBandwidthBytes, quota.StorageUsed, quota.BandwidthUsed, quota.ResetBandwidthAt, quota.CreatedAt, quota.UpdatedAt)
	return err
}

// countStorageObjectsByUser returns the count of storage objects for a user
func (e *CloudStorageExtension) countStorageObjectsByUser(userID string) (int64, error) {
	if e.sqlDB == nil {
		return 0, fmt.Errorf("database not initialized")
	}
	var count int64
	err := e.sqlDB.QueryRow("SELECT COUNT(*) FROM storage_objects WHERE user_id = ?", userID).Scan(&count)
	return count, err
}

// sumStorageSizeByUser returns the total storage size for a user
func (e *CloudStorageExtension) sumStorageSizeByUser(userID string) (int64, error) {
	if e.sqlDB == nil {
		return 0, fmt.Errorf("database not initialized")
	}
	var size sql.NullInt64
	err := e.sqlDB.QueryRow("SELECT COALESCE(SUM(size), 0) FROM storage_objects WHERE user_id = ?", userID).Scan(&size)
	if err != nil {
		return 0, err
	}
	return size.Int64, nil
}

// getUserEmail retrieves a user's email by ID
func (e *CloudStorageExtension) getUserEmail(userID string) (string, error) {
	if e.sqlDB == nil {
		return "", fmt.Errorf("database not initialized")
	}
	var email string
	err := e.sqlDB.QueryRow("SELECT email FROM auth_users WHERE id = ?", userID).Scan(&email)
	return email, err
}

// countShares returns share statistics
func (e *CloudStorageExtension) countShares(condition string, args ...interface{}) (int64, error) {
	if e.sqlDB == nil {
		return 0, fmt.Errorf("database not initialized")
	}
	query := "SELECT COUNT(*) FROM ext_cloudstorage_storage_shares"
	if condition != "" {
		query += " WHERE " + condition
	}
	var count int64
	err := e.sqlDB.QueryRow(query, args...).Scan(&count)
	return count, err
}

// getRoleQuotaByRoleID retrieves a role quota by role ID
func (e *CloudStorageExtension) getRoleQuotaByRoleID(roleID string) (*RoleQuota, error) {
	if e.sqlDB == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	var rq RoleQuota
	var allowedExt, blockedExt sql.NullString
	err := e.sqlDB.QueryRow(`
		SELECT id, role_id, role_name, max_storage_bytes, max_bandwidth_bytes, max_upload_size, max_files_count, allowed_extensions, blocked_extensions, created_at, updated_at
		FROM ext_cloudstorage_role_quotas WHERE role_id = ?`, roleID).Scan(
		&rq.ID, &rq.RoleID, &rq.RoleName, &rq.MaxStorageBytes, &rq.MaxBandwidthBytes, &rq.MaxUploadSize, &rq.MaxFilesCount, &allowedExt, &blockedExt, &rq.CreatedAt, &rq.UpdatedAt)
	if err != nil {
		return nil, err
	}
	if allowedExt.Valid {
		rq.AllowedExtensions = allowedExt.String
	}
	if blockedExt.Valid {
		rq.BlockedExtensions = blockedExt.String
	}
	return &rq, nil
}

// getAllRoleQuotas retrieves all role quotas
func (e *CloudStorageExtension) getAllRoleQuotas() ([]RoleQuota, error) {
	if e.sqlDB == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	rows, err := e.sqlDB.Query(`
		SELECT id, role_id, role_name, max_storage_bytes, max_bandwidth_bytes, max_upload_size, max_files_count, allowed_extensions, blocked_extensions, created_at, updated_at
		FROM ext_cloudstorage_role_quotas`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var quotas []RoleQuota
	for rows.Next() {
		var rq RoleQuota
		var allowedExt, blockedExt sql.NullString
		err := rows.Scan(&rq.ID, &rq.RoleID, &rq.RoleName, &rq.MaxStorageBytes, &rq.MaxBandwidthBytes, &rq.MaxUploadSize, &rq.MaxFilesCount, &allowedExt, &blockedExt, &rq.CreatedAt, &rq.UpdatedAt)
		if err != nil {
			return nil, err
		}
		if allowedExt.Valid {
			rq.AllowedExtensions = allowedExt.String
		}
		if blockedExt.Valid {
			rq.BlockedExtensions = blockedExt.String
		}
		quotas = append(quotas, rq)
	}
	return quotas, nil
}

// getActiveUserQuotaOverrides retrieves active user quota overrides
func (e *CloudStorageExtension) getActiveUserQuotaOverrides() ([]UserQuotaOverride, error) {
	if e.sqlDB == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	rows, err := e.sqlDB.Query(`
		SELECT id, user_id, max_storage_bytes, max_bandwidth_bytes, max_upload_size, max_files_count, allowed_extensions, blocked_extensions, reason, expires_at, created_by, created_at, updated_at
		FROM ext_cloudstorage_user_quota_overrides
		WHERE expires_at IS NULL OR expires_at > ?`, apptime.NowTime())
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var overrides []UserQuotaOverride
	for rows.Next() {
		var o UserQuotaOverride
		var allowedExt, blockedExt, reason, createdBy sql.NullString
		var expiresAt sql.NullTime
		err := rows.Scan(&o.ID, &o.UserID, &o.MaxStorageBytes, &o.MaxBandwidthBytes, &o.MaxUploadSize, &o.MaxFilesCount, &allowedExt, &blockedExt, &reason, &expiresAt, &createdBy, &o.CreatedAt, &o.UpdatedAt)
		if err != nil {
			return nil, err
		}
		if allowedExt.Valid {
			o.AllowedExtensions = &allowedExt.String
		}
		if blockedExt.Valid {
			o.BlockedExtensions = &blockedExt.String
		}
		if reason.Valid {
			o.Reason = &reason.String
		}
		if expiresAt.Valid {
			o.ExpiresAt = apptime.NewNullTime(expiresAt.Time)
		}
		if createdBy.Valid {
			o.CreatedBy = createdBy.String
		}
		overrides = append(overrides, o)
	}
	return overrides, nil
}

// createUserQuotaOverride creates a user quota override
func (e *CloudStorageExtension) createUserQuotaOverride(override *UserQuotaOverride) error {
	if e.sqlDB == nil {
		return fmt.Errorf("database not initialized")
	}
	_, err := e.sqlDB.Exec(`
		INSERT INTO ext_cloudstorage_user_quota_overrides (id, user_id, max_storage_bytes, max_bandwidth_bytes, max_upload_size, max_files_count, allowed_extensions, blocked_extensions, reason, expires_at, created_by, created_at, updated_at)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		override.ID, override.UserID, override.MaxStorageBytes, override.MaxBandwidthBytes, override.MaxUploadSize, override.MaxFilesCount, override.AllowedExtensions, override.BlockedExtensions, override.Reason, override.ExpiresAt, override.CreatedBy, override.CreatedAt, override.UpdatedAt)
	return err
}

// deleteUserQuotaOverrideByID deletes a user quota override by ID
func (e *CloudStorageExtension) deleteUserQuotaOverrideByID(id string) error {
	if e.sqlDB == nil {
		return fmt.Errorf("database not initialized")
	}
	_, err := e.sqlDB.Exec("DELETE FROM ext_cloudstorage_user_quota_overrides WHERE id = ?", id)
	return err
}

// listSharesWithObjects retrieves shares with their object info
func (e *CloudStorageExtension) listSharesWithObjects(objectID, userID string, isAdmin bool) ([]map[string]interface{}, error) {
	if e.sqlDB == nil {
		return nil, fmt.Errorf("database not initialized")
	}

	var query string
	var args []interface{}
	if isAdmin {
		if objectID != "" {
			query = "SELECT ss.*, u.email as shared_by_email FROM ext_cloudstorage_storage_shares ss LEFT JOIN auth_users u ON ss.created_by = u.id WHERE ss.object_id = ? ORDER BY ss.created_at DESC"
			args = []interface{}{objectID}
		} else {
			query = "SELECT ss.*, u.email as shared_by_email FROM ext_cloudstorage_storage_shares ss LEFT JOIN auth_users u ON ss.created_by = u.id ORDER BY ss.created_at DESC"
		}
	} else {
		if objectID != "" {
			query = "SELECT ss.*, u.email as shared_by_email FROM ext_cloudstorage_storage_shares ss LEFT JOIN auth_users u ON ss.created_by = u.id WHERE ss.object_id = ? AND ss.created_by = ? ORDER BY ss.created_at DESC"
			args = []interface{}{objectID, userID}
		} else {
			query = "SELECT ss.*, u.email as shared_by_email FROM ext_cloudstorage_storage_shares ss LEFT JOIN auth_users u ON ss.created_by = u.id WHERE ss.created_by = ? ORDER BY ss.created_at DESC"
			args = []interface{}{userID}
		}
	}

	rows, err := e.sqlDB.Query(query, args...)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var results []map[string]interface{}
	for rows.Next() {
		var share StorageShare
		var sharedByEmail sql.NullString
		var sharedWithUserID, sharedWithEmail, shareToken sql.NullString
		var expiresAt sql.NullTime

		err := rows.Scan(&share.ID, &share.ObjectID, &sharedWithUserID, &sharedWithEmail, &share.PermissionLevel, &share.InheritToChildren, &shareToken, &share.IsPublic, &expiresAt, &share.CreatedBy, &share.CreatedAt, &share.UpdatedAt, &sharedByEmail)
		if err != nil {
			continue
		}
		if sharedWithUserID.Valid {
			share.SharedWithUserID = &sharedWithUserID.String
		}
		if sharedWithEmail.Valid {
			share.SharedWithEmail = &sharedWithEmail.String
		}
		if shareToken.Valid {
			share.ShareToken = &shareToken.String
		}
		if expiresAt.Valid {
			share.ExpiresAt = apptime.NewNullTime(expiresAt.Time)
		}

		result := map[string]interface{}{
			"id":               share.ID,
			"objectId":         share.ObjectID,
			"sharedWithUserId": share.SharedWithUserID,
			"sharedWithEmail":  share.SharedWithEmail,
			"permissionLevel":  share.PermissionLevel,
			"shareToken":       share.ShareToken,
			"isPublic":         share.IsPublic,
			"expiresAt":        share.ExpiresAt,
			"createdBy":        share.CreatedBy,
			"createdAt":        share.CreatedAt,
		}
		if sharedByEmail.Valid {
			result["sharedByEmail"] = sharedByEmail.String
		}
		results = append(results, result)
	}
	return results, nil
}

// getStorageStats returns storage statistics
func (e *CloudStorageExtension) getStorageStats(userID string, isAdmin bool) (totalObjects int64, totalSize int64, err error) {
	if e.sqlDB == nil {
		return 0, 0, fmt.Errorf("database not initialized")
	}
	var query string
	var args []interface{}
	if isAdmin {
		query = "SELECT COUNT(*), COALESCE(SUM(size), 0) FROM storage_objects"
	} else {
		query = "SELECT COUNT(*), COALESCE(SUM(size), 0) FROM storage_objects WHERE user_id = ?"
		args = []interface{}{userID}
	}
	err = e.sqlDB.QueryRow(query, args...).Scan(&totalObjects, &totalSize)
	return totalObjects, totalSize, err
}

// getQuotaAggregateStats returns aggregate quota statistics for admins
func (e *CloudStorageExtension) getQuotaAggregateStats() (totalUsers, totalStorageUsed, totalStorageLimit, totalBandwidthUsed, totalBandwidthLimit int64, err error) {
	if e.sqlDB == nil {
		return 0, 0, 0, 0, 0, fmt.Errorf("database not initialized")
	}
	err = e.sqlDB.QueryRow(`
		SELECT
			COUNT(*),
			COALESCE(SUM(storage_used), 0),
			COALESCE(SUM(max_storage_bytes), 0),
			COALESCE(SUM(bandwidth_used), 0),
			COALESCE(SUM(max_bandwidth_bytes), 0)
		FROM ext_cloudstorage_storage_quotas
	`).Scan(&totalUsers, &totalStorageUsed, &totalStorageLimit, &totalBandwidthUsed, &totalBandwidthLimit)
	return
}

// countUsersNearQuotaLimit counts users at >80% quota usage
func (e *CloudStorageExtension) countUsersNearQuotaLimit() (int64, error) {
	if e.sqlDB == nil {
		return 0, fmt.Errorf("database not initialized")
	}
	var count int64
	err := e.sqlDB.QueryRow(`
		SELECT COUNT(*) FROM ext_cloudstorage_storage_quotas
		WHERE (max_storage_bytes > 0 AND (storage_used * 100.0 / max_storage_bytes) > 80)
		   OR (max_bandwidth_bytes > 0 AND (bandwidth_used * 100.0 / max_bandwidth_bytes) > 80)
	`).Scan(&count)
	return count, err
}

// countSharedFolders counts shared folders vs files
func (e *CloudStorageExtension) countSharedFolders(userID string, isAdmin bool) (int64, error) {
	if e.sqlDB == nil {
		return 0, fmt.Errorf("database not initialized")
	}
	var query string
	var args []interface{}
	if isAdmin {
		query = `SELECT COUNT(*) FROM ext_cloudstorage_storage_shares ss
			JOIN storage_objects so ON ss.object_id = so.id
			WHERE so.content_type = 'application/x-directory'`
	} else {
		query = `SELECT COUNT(*) FROM ext_cloudstorage_storage_shares ss
			JOIN storage_objects so ON ss.object_id = so.id
			WHERE ss.created_by = ? AND so.content_type = 'application/x-directory'`
		args = []interface{}{userID}
	}
	var count int64
	err := e.sqlDB.QueryRow(query, args...).Scan(&count)
	return count, err
}

// getShareByObjectAndUser retrieves a share by object ID and user/public access
func (e *CloudStorageExtension) getShareByObjectAndUser(objectID, userID string) (*StorageShare, error) {
	if e.sqlDB == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	var share StorageShare
	var sharedWithUserID, sharedWithEmail, shareToken sql.NullString
	var expiresAt sql.NullTime
	var inheritToChildren int64

	err := e.sqlDB.QueryRow(`
		SELECT id, object_id, shared_with_user_id, shared_with_email, permission_level, inherit_to_children, share_token, is_public, expires_at, created_by, created_at, updated_at
		FROM ext_cloudstorage_storage_shares
		WHERE object_id = ? AND (is_public = 1 OR shared_with_user_id = ?)
		LIMIT 1`,
		objectID, userID).Scan(&share.ID, &share.ObjectID, &sharedWithUserID, &sharedWithEmail, &share.PermissionLevel, &inheritToChildren, &shareToken, &share.IsPublic, &expiresAt, &share.CreatedBy, &share.CreatedAt, &share.UpdatedAt)
	if err != nil {
		return nil, err
	}
	share.InheritToChildren = inheritToChildren != 0
	if sharedWithUserID.Valid {
		share.SharedWithUserID = &sharedWithUserID.String
	}
	if sharedWithEmail.Valid {
		share.SharedWithEmail = &sharedWithEmail.String
	}
	if shareToken.Valid {
		share.ShareToken = &shareToken.String
	}
	if expiresAt.Valid {
		share.ExpiresAt = apptime.NewNullTime(expiresAt.Time)
	}
	return &share, nil
}

// getShareByObjectPublicOnly retrieves a public share by object ID
func (e *CloudStorageExtension) getShareByObjectPublicOnly(objectID string) (*StorageShare, error) {
	if e.sqlDB == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	var share StorageShare
	var sharedWithUserID, sharedWithEmail, shareToken sql.NullString
	var expiresAt sql.NullTime
	var inheritToChildren int64

	err := e.sqlDB.QueryRow(`
		SELECT id, object_id, shared_with_user_id, shared_with_email, permission_level, inherit_to_children, share_token, is_public, expires_at, created_by, created_at, updated_at
		FROM ext_cloudstorage_storage_shares
		WHERE object_id = ? AND is_public = 1
		LIMIT 1`,
		objectID).Scan(&share.ID, &share.ObjectID, &sharedWithUserID, &sharedWithEmail, &share.PermissionLevel, &inheritToChildren, &shareToken, &share.IsPublic, &expiresAt, &share.CreatedBy, &share.CreatedAt, &share.UpdatedAt)
	if err != nil {
		return nil, err
	}
	share.InheritToChildren = inheritToChildren != 0
	if sharedWithUserID.Valid {
		share.SharedWithUserID = &sharedWithUserID.String
	}
	if sharedWithEmail.Valid {
		share.SharedWithEmail = &sharedWithEmail.String
	}
	if shareToken.Valid {
		share.ShareToken = &shareToken.String
	}
	if expiresAt.Valid {
		share.ExpiresAt = apptime.NewNullTime(expiresAt.Time)
	}
	return &share, nil
}

// searchUsers searches for users by email, name, or ID
func (e *CloudStorageExtension) searchUsers(query string) ([]map[string]string, error) {
	if e.sqlDB == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	rows, err := e.sqlDB.Query(`
		SELECT id, email FROM auth_users
		WHERE email LIKE ? OR id LIKE ?
		LIMIT 10`,
		"%"+query+"%", "%"+query+"%")
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var users []map[string]string
	for rows.Next() {
		var id, email string
		if err := rows.Scan(&id, &email); err != nil {
			continue
		}
		users = append(users, map[string]string{"id": id, "email": email})
	}
	return users, nil
}

// upsertRoleQuota inserts or updates a role quota
func (e *CloudStorageExtension) upsertRoleQuota(quota *RoleQuota) error {
	if e.sqlDB == nil {
		return fmt.Errorf("database not initialized")
	}
	now := apptime.NowTime()
	_, err := e.sqlDB.Exec(`
		INSERT INTO ext_cloudstorage_role_quotas (id, role_id, role_name, max_storage_bytes, max_bandwidth_bytes, max_upload_size, max_files_count, allowed_extensions, blocked_extensions, created_at, updated_at)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
		ON CONFLICT(role_id) DO UPDATE SET
			role_name = excluded.role_name,
			max_storage_bytes = excluded.max_storage_bytes,
			max_bandwidth_bytes = excluded.max_bandwidth_bytes,
			max_upload_size = excluded.max_upload_size,
			max_files_count = excluded.max_files_count,
			allowed_extensions = excluded.allowed_extensions,
			blocked_extensions = excluded.blocked_extensions,
			updated_at = excluded.updated_at
	`, quota.ID, quota.RoleID, quota.RoleName, quota.MaxStorageBytes, quota.MaxBandwidthBytes, quota.MaxUploadSize, quota.MaxFilesCount, quota.AllowedExtensions, quota.BlockedExtensions, now, now)
	return err
}

// createAccessLog creates an access log entry
func (e *CloudStorageExtension) createAccessLog(log *StorageAccessLog) error {
	if e.sqlDB == nil {
		return fmt.Errorf("database not initialized")
	}
	_, err := e.sqlDB.Exec(`
		INSERT INTO ext_cloudstorage_storage_access_logs (id, object_id, user_id, ip_address, action, user_agent, metadata, created_at)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
		log.ID, log.ObjectID, log.UserID, log.IPAddress, log.Action, log.UserAgent, log.Metadata, log.CreatedAt)
	return err
}

// getMyFilesFolder checks if the user's "My Files" folder exists
func (e *CloudStorageExtension) getMyFilesFolder(userID, appID string) (*pkgstorage.StorageObject, error) {
	if e.sqlDB == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	var obj pkgstorage.StorageObject
	var parentFolderID, checksum, metadata, lastViewed, appIDVal sql.NullString
	err := e.sqlDB.QueryRow(`
		SELECT id, bucket_name, object_name, parent_folder_id, size, content_type, checksum, metadata, created_at, updated_at, last_viewed, user_id, app_id
		FROM storage_objects
		WHERE bucket_name = 'int_storage' AND user_id = ? AND app_id = ? AND object_name = 'My Files' AND content_type = 'application/x-directory' AND parent_folder_id IS NULL`,
		userID, appID).Scan(&obj.ID, &obj.BucketName, &obj.ObjectName, &parentFolderID, &obj.Size, &obj.ContentType, &checksum, &metadata, &obj.CreatedAt, &obj.UpdatedAt, &lastViewed, &obj.UserID, &appIDVal)
	if err != nil {
		return nil, err
	}
	if parentFolderID.Valid {
		obj.ParentFolderID = &parentFolderID.String
	}
	if checksum.Valid {
		obj.Checksum = checksum.String
	}
	if metadata.Valid {
		obj.Metadata = metadata.String
	}
	if appIDVal.Valid {
		obj.AppID = &appIDVal.String
	}
	return &obj, nil
}

// createMyFilesFolder creates the user's "My Files" folder
func (e *CloudStorageExtension) createMyFilesFolder(obj *pkgstorage.StorageObject) error {
	if e.sqlDB == nil {
		return fmt.Errorf("database not initialized")
	}
	now := apptime.NowTime()
	_, err := e.sqlDB.Exec(`
		INSERT INTO storage_objects (id, bucket_name, object_name, parent_folder_id, size, content_type, checksum, metadata, created_at, updated_at, last_viewed, user_id, app_id)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		obj.ID, obj.BucketName, obj.ObjectName, obj.ParentFolderID, obj.Size, obj.ContentType, obj.Checksum, obj.Metadata, now, now, nil, obj.UserID, obj.AppID)
	return err
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
