package files

import (
	"context"
	"log"

	"github.com/suppers-ai/solobase/core/config"
	"github.com/suppers-ai/solobase/core/env"
	wafer "github.com/wafer-run/wafer-go"
	"github.com/wafer-run/wafer-go/services/database"
	waferstorage "github.com/wafer-run/wafer-go/services/storage"
)

const BlockName = "files-feature"

// FilesBlock is a unified block combining storage + cloud storage features.
// It handles file upload/download, sharing, quotas, and access logging.
type FilesBlock struct {
	router         *wafer.Router
	storageService *StorageService
	db             database.Service
	storageSvc     waferstorage.Service
	cloudConfig    *CloudStorageConfig

	shareService     *ShareService
	quotaService     *QuotaService
	accessLogService *AccessLogService
}

// NewFilesBlock creates a new files block with zero constructor dependencies.
func NewFilesBlock() *FilesBlock {
	b := &FilesBlock{
		cloudConfig: &CloudStorageConfig{
			DefaultStorageLimit:   5368709120,  // 5GB
			DefaultBandwidthLimit: 10737418240, // 10GB
			EnableSharing:         true,
			EnableAccessLogs:      true,
			EnableQuotas:          true,
			BandwidthResetPeriod:  "monthly",
		},
	}
	b.router = wafer.NewRouter()
	b.registerRoutes()
	return b
}

func (b *FilesBlock) registerRoutes() {
	// Public: direct download via token
	b.router.Retrieve("/storage/direct/{token}", b.handleDirectDownload)

	// Protected (user) storage endpoints
	b.router.Retrieve("/storage/buckets", b.handleGetBuckets)
	b.router.Create("/storage/buckets", b.handleCreateBucket)
	b.router.Delete("/storage/buckets/{bucket}", b.handleDeleteBucket)
	b.router.Retrieve("/storage/buckets/{bucket}/objects", b.handleGetBucketObjects)
	b.router.Create("/storage/buckets/{bucket}/upload", b.handleUploadFile)
	b.router.Create("/storage/buckets/{bucket}/upload-url", b.handleGenerateUploadURL)
	b.router.Retrieve("/storage/buckets/{bucket}/objects/{id}", b.handleGetObject)
	b.router.Delete("/storage/buckets/{bucket}/objects/{id}", b.handleDeleteObject)
	b.router.Retrieve("/storage/buckets/{bucket}/objects/{id}/download", b.handleDownloadObject)
	b.router.Retrieve("/storage/buckets/{bucket}/objects/{id}/download-url", b.handleGenerateDownloadURL)
	b.router.Update("/storage/buckets/{bucket}/objects/{id}/rename", b.handleRenameObject)
	b.router.Update("/storage/buckets/{bucket}/objects/{id}/metadata", b.handleUpdateObjectMetadata)
	b.router.Create("/storage/buckets/{bucket}/folders", b.handleCreateFolder)
	b.router.Retrieve("/storage/search", b.handleSearchObjects)
	b.router.Retrieve("/storage/recently-viewed", b.handleGetRecentlyViewed)
	b.router.Create("/storage/items/{id}/last-viewed", b.handleUpdateLastViewed)
	b.router.Retrieve("/storage/quota", b.handleGetQuota)
	b.router.Retrieve("/storage/stats", b.handleGetStats)

	// Admin storage endpoints
	b.router.Retrieve("/admin/storage/stats", b.handleGetAdminStats)

	// Cloud storage protected routes
	b.router.Retrieve("/ext/cloudstorage/stats", b.handleCloudStats)
	b.router.Retrieve("/ext/cloudstorage/shares", b.handleSharesGet)
	b.router.Create("/ext/cloudstorage/shares", b.handleSharesPost)
	b.router.Retrieve("/ext/cloudstorage/share/{token}", b.handleShareAccess)
	b.router.Retrieve("/ext/cloudstorage/quota", b.handleCloudQuotaGet)
	b.router.Update("/ext/cloudstorage/quota", b.handleCloudQuotaPut)
	b.router.Retrieve("/ext/cloudstorage/quotas/user", b.handleGetUserQuota)
	b.router.Retrieve("/ext/cloudstorage/access-logs", b.handleAccessLogs)
	b.router.Retrieve("/ext/cloudstorage/access-stats", b.handleAccessStats)

	// Cloud storage admin routes
	b.router.Retrieve("/ext/cloudstorage/quotas/roles", b.handleGetRoleQuotas)
	b.router.Update("/ext/cloudstorage/quotas/roles/{role}", b.handleUpdateRoleQuota)
	b.router.Retrieve("/ext/cloudstorage/quotas/overrides", b.handleGetUserOverrides)
	b.router.Create("/ext/cloudstorage/quotas/overrides", b.handleCreateUserOverride)
	b.router.Delete("/ext/cloudstorage/quotas/overrides/{user}", b.handleDeleteUserOverride)
	b.router.Retrieve("/ext/cloudstorage/users/search", b.handleUserSearch)
	b.router.Retrieve("/ext/cloudstorage/default-quotas", b.handleDefaultQuotasGet)
	b.router.Update("/ext/cloudstorage/default-quotas", b.handleDefaultQuotasPut)
}

func (b *FilesBlock) Info() wafer.BlockInfo {
	return wafer.BlockInfo{
		Name:         BlockName,
		Version:      "1.0.0",
		Interface:    "http.handler",
		Summary:      "File storage, sharing, quotas and access logging",
		InstanceMode: wafer.Singleton,
		AllowedModes: []wafer.InstanceMode{wafer.Singleton},
		AdminUI:      &wafer.AdminUIInfo{Path: "/admin/storage", Icon: "hard-drive", Title: "Storage"},
	}
}

func (b *FilesBlock) Handle(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	return b.router.Route(ctx, msg)
}

func (b *FilesBlock) Lifecycle(ctx wafer.Context, evt wafer.LifecycleEvent) error {
	if evt.Type != wafer.Init {
		return nil
	}

	db := ctx.Services().Database
	if db == nil {
		log.Printf("Files block: no database available, storage will be unavailable")
		return nil
	}
	b.db = db

	// Storage provider service (for share downloads via wafer storage)
	b.storageSvc = ctx.Services().Storage

	// Create storage repository and service
	repo := newStorageRepository(db)
	cfg := config.StorageConfig{
		Type:             env.GetEnv("STORAGE_TYPE"),
		LocalStoragePath: env.GetEnv("STORAGE_PATH"),
		S3Endpoint:       env.GetEnv("S3_ENDPOINT"),
		S3AccessKey:      env.GetEnv("S3_ACCESS_KEY"),
		S3SecretKey:      env.GetEnv("S3_SECRET_KEY"),
		S3Region:         env.GetEnv("S3_REGION"),
		S3Bucket:         env.GetEnv("S3_BUCKET"),
		S3UseSSL:         env.GetEnv("S3_USE_SSL") == "true",
	}
	b.storageService = NewStorageService(repo, cfg)

	// Initialize cloud storage services
	b.shareService = NewShareService(db, nil)
	b.quotaService = NewQuotaService(db)
	b.accessLogService = NewAccessLogService(db)

	// Initialize default quotas
	if b.cloudConfig.EnableQuotas && b.quotaService != nil {
		if err := b.quotaService.InitializeDefaultQuotas(); err != nil {
			log.Printf("Warning: Failed to initialize default quotas: %v", err)
		}
	}

	// Initialize extension settings
	b.initializeExtensionSettings()

	return nil
}

// ownershipError returns an appropriate error result for ownership check failures.
func (b *FilesBlock) ownershipError(msg *wafer.Message, err error) wafer.Result {
	switch err {
	case ErrNotOwner:
		return wafer.Error(msg, 403, "forbidden", "Access denied: not owner")
	case ErrAppIDMismatch:
		return wafer.Error(msg, 403, "forbidden", "Access denied: app ID mismatch")
	default:
		return wafer.Error(msg, 403, "forbidden", err.Error())
	}
}

// initializeExtensionSettings sets up default settings.
func (b *FilesBlock) initializeExtensionSettings() {
	if b.db == nil {
		return
	}

	ctx := context.Background()
	count, err := database.CountByField(ctx, b.db, "sys_settings", "key", "ext_cloudstorage_profile_show_usage")
	if err != nil {
		return
	}

	if count == 0 {
		_, _ = b.db.Create(ctx, "sys_settings", map[string]any{
			"id":    "ext_cloudstorage_profile_show_usage",
			"key":   "ext_cloudstorage_profile_show_usage",
			"value": "true",
			"type":  "bool",
		})
	}
}
