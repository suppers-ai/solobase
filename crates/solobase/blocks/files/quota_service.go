package files

import (
	"context"
	"errors"
	"fmt"
	"strings"

	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/solobase/core/uuid"
	"github.com/wafer-run/wafer-go/services/database"
)

// QuotaService manages storage quotas and limits
type QuotaService struct {
	db database.Service
}

// NewQuotaService creates a new quota service
func NewQuotaService(db database.Service) *QuotaService {
	return &QuotaService{
		db: db,
	}
}

// InitializeDefaultQuotas creates default quotas for system roles
func (q *QuotaService) InitializeDefaultQuotas() error {
	ctx := context.Background()
	defaultQuotas := []RoleQuota{
		{
			RoleName:          "admin",
			MaxStorageBytes:   107374182400,  // 100GB
			MaxBandwidthBytes: 1099511627776, // 1TB
			MaxUploadSize:     5368709120,    // 5GB
			MaxFilesCount:     100000,
		},
		{
			RoleName:          "user",
			MaxStorageBytes:   5368709120,  // 5GB
			MaxBandwidthBytes: 53687091200, // 50GB
			MaxUploadSize:     104857600,   // 100MB
			MaxFilesCount:     1000,
		},
		{
			RoleName:          "editor",
			MaxStorageBytes:   10737418240,  // 10GB
			MaxBandwidthBytes: 107374182400, // 100GB
			MaxUploadSize:     1073741824,   // 1GB
			MaxFilesCount:     5000,
		},
		{
			RoleName:          "viewer",
			MaxStorageBytes:   1073741824,  // 1GB
			MaxBandwidthBytes: 10737418240, // 10GB
			MaxUploadSize:     0,           // No uploads
			MaxFilesCount:     0,
		},
		{
			RoleName:          "restricted",
			MaxStorageBytes:   104857600,  // 100MB
			MaxBandwidthBytes: 1073741824, // 1GB
			MaxUploadSize:     10485760,   // 10MB
			MaxFilesCount:     100,
			BlockedExtensions: "exe,bat,sh,cmd,ps1", // Block executables
		},
	}

	for _, quota := range defaultQuotas {
		// Check if quota already exists
		count, err := database.CountByField(ctx, q.db, "ext_cloudstorage_role_quotas", "role_name", quota.RoleName)
		if err == nil && count > 0 {
			continue // Already exists
		}

		// Get role ID from database directly
		roleRec, err := database.GetByField(ctx, q.db, "iam_roles", "name", quota.RoleName)
		if err != nil {
			continue // Role doesn't exist, skip
		}
		quota.RoleID = roleRec.ID

		// Create quota
		quota.ID = uuid.New().String()
		quota.CreatedAt = apptime.NowTime()
		quota.UpdatedAt = apptime.NowTime()

		_, err = q.db.Create(ctx, "ext_cloudstorage_role_quotas", map[string]any{
			"id":                  quota.ID,
			"role_id":             quota.RoleID,
			"role_name":           quota.RoleName,
			"max_storage_bytes":   quota.MaxStorageBytes,
			"max_bandwidth_bytes": quota.MaxBandwidthBytes,
			"max_upload_size":     quota.MaxUploadSize,
			"max_files_count":     quota.MaxFilesCount,
			"allowed_extensions":  stringPtrOrNil(quota.AllowedExtensions),
			"blocked_extensions":  stringPtrOrNil(quota.BlockedExtensions),
		})
		if err != nil {
			return fmt.Errorf("failed to create quota for role %s: %w", quota.RoleName, err)
		}
	}

	return nil
}

// GetUserQuota gets the effective quota for a user (considering role and overrides)
func (q *QuotaService) GetUserQuota(ctx context.Context, userID string) (*EffectiveQuota, error) {
	// Validate input
	if userID == "" {
		return nil, fmt.Errorf("user ID is required")
	}

	// First check for user-specific override
	var override *UserQuotaOverride
	now := apptime.NowString()
	overrideRecords, err := q.db.QueryRaw(ctx, `
		SELECT id, user_id, max_storage_bytes, max_bandwidth_bytes, max_upload_size, max_files_count,
		       allowed_extensions, blocked_extensions, reason, expires_at, created_by, created_at, updated_at
		FROM ext_cloudstorage_user_quota_overrides
		WHERE user_id = ? AND (expires_at IS NULL OR expires_at > ?)`,
		userID, now,
	)
	if err == nil && len(overrideRecords) > 0 {
		o := recordToUserQuotaOverride(overrideRecords[0])
		override = &o
	}

	// Get user's roles from database directly
	roleRecords, err := q.db.QueryRaw(ctx, `
		SELECT r.name as role_name
		FROM iam_user_roles ur
		JOIN iam_roles r ON ur.role_id = r.id
		WHERE ur.user_id = ?
	`, userID)
	if err != nil {
		return nil, fmt.Errorf("failed to get user roles: %w", err)
	}

	var roleNames []string
	for _, rec := range roleRecords {
		roleNames = append(roleNames, stringVal(rec.Data["role_name"]))
	}

	// Get quotas for all user's roles and take the maximum values
	var roleQuotas []RoleQuota
	for _, roleName := range roleNames {
		quotaRec, err := database.GetByField(ctx, q.db, "ext_cloudstorage_role_quotas", "role_name", roleName)
		if err == nil {
			roleQuotas = append(roleQuotas, recordToRoleQuota(quotaRec))
		}
	}

	// Calculate effective quota (taking max from all roles)
	effective := &EffectiveQuota{
		UserID:            userID,
		MaxStorageBytes:   0,
		MaxBandwidthBytes: 0,
		MaxUploadSize:     0,
		MaxFilesCount:     0,
	}

	for _, quota := range roleQuotas {
		if quota.MaxStorageBytes > effective.MaxStorageBytes {
			effective.MaxStorageBytes = quota.MaxStorageBytes
		}
		if quota.MaxBandwidthBytes > effective.MaxBandwidthBytes {
			effective.MaxBandwidthBytes = quota.MaxBandwidthBytes
		}
		if quota.MaxUploadSize > effective.MaxUploadSize {
			effective.MaxUploadSize = quota.MaxUploadSize
		}
		if quota.MaxFilesCount > effective.MaxFilesCount {
			effective.MaxFilesCount = quota.MaxFilesCount
		}

		// Merge allowed extensions
		if quota.AllowedExtensions != "" {
			effective.AllowedExtensions = mergeExtensions(effective.AllowedExtensions, quota.AllowedExtensions)
		}

		// Merge blocked extensions (intersection - only block if all roles block it)
		if quota.BlockedExtensions != "" {
			if effective.BlockedExtensions == "" {
				effective.BlockedExtensions = quota.BlockedExtensions
			} else {
				effective.BlockedExtensions = intersectExtensions(effective.BlockedExtensions, quota.BlockedExtensions)
			}
		}
	}

	// Apply user-specific overrides if they exist
	if override != nil {
		if override.MaxStorageBytes != nil {
			effective.MaxStorageBytes = *override.MaxStorageBytes
		}
		if override.MaxBandwidthBytes != nil {
			effective.MaxBandwidthBytes = *override.MaxBandwidthBytes
		}
		if override.MaxUploadSize != nil {
			effective.MaxUploadSize = *override.MaxUploadSize
		}
		if override.MaxFilesCount != nil {
			effective.MaxFilesCount = *override.MaxFilesCount
		}
		if override.AllowedExtensions != nil {
			effective.AllowedExtensions = *override.AllowedExtensions
		}
		if override.BlockedExtensions != nil {
			effective.BlockedExtensions = *override.BlockedExtensions
		}
	}

	// Get current usage and file count
	usageRecords, err := q.db.QueryRaw(ctx,
		"SELECT COALESCE(storage_used, 0) as storage_used, COALESCE(bandwidth_used, 0) as bandwidth_used FROM ext_cloudstorage_storage_quotas WHERE user_id = ?",
		userID)
	if err == nil && len(usageRecords) > 0 {
		effective.StorageUsed = toInt64Val(usageRecords[0].Data["storage_used"])
		effective.BandwidthUsed = toInt64Val(usageRecords[0].Data["bandwidth_used"])
	}

	// Get file count (fix: user_id, not owner_id)
	fileCount, err := database.CountByField(ctx, q.db, "storage_objects", "user_id", userID)
	if err == nil {
		effective.FilesUsed = int64(fileCount)
	}

	return effective, nil
}

// CheckUploadAllowed checks if a user can upload a file
func (q *QuotaService) CheckUploadAllowed(ctx context.Context, userID string, fileSize int64, fileName string) error {
	// Validate input
	if userID == "" {
		return fmt.Errorf("user ID is required")
	}
	if fileSize < 0 {
		return fmt.Errorf("invalid file size")
	}
	quota, err := q.GetUserQuota(ctx, userID)
	if err != nil {
		return fmt.Errorf("failed to get user quota: %w", err)
	}

	// Check file size limit
	if quota.MaxUploadSize > 0 && fileSize > quota.MaxUploadSize {
		return fmt.Errorf("file size %d exceeds maximum upload size %d", fileSize, quota.MaxUploadSize)
	}

	// Check storage quota
	if quota.MaxStorageBytes > 0 && (quota.StorageUsed+fileSize) > quota.MaxStorageBytes {
		return fmt.Errorf("upload would exceed storage quota (used: %d, max: %d)", quota.StorageUsed, quota.MaxStorageBytes)
	}

	// Check file count limit
	if quota.MaxFilesCount > 0 && quota.FilesUsed >= quota.MaxFilesCount {
		return fmt.Errorf("maximum file count reached (%d)", quota.MaxFilesCount)
	}

	// Check file extension
	ext := getFileExtension(fileName)
	if ext != "" {
		// Check blocked extensions
		if quota.BlockedExtensions != "" {
			blocked := strings.Split(strings.ToLower(quota.BlockedExtensions), ",")
			for _, blockedExt := range blocked {
				if strings.TrimSpace(blockedExt) == strings.ToLower(ext) {
					return fmt.Errorf("file type .%s is not allowed", ext)
				}
			}
		}

		// Check allowed extensions (if specified, only these are allowed)
		if quota.AllowedExtensions != "" {
			allowed := strings.Split(strings.ToLower(quota.AllowedExtensions), ",")
			found := false
			for _, allowedExt := range allowed {
				if strings.TrimSpace(allowedExt) == strings.ToLower(ext) {
					found = true
					break
				}
			}
			if !found {
				return fmt.Errorf("file type .%s is not in allowed list", ext)
			}
		}
	}

	return nil
}

// UpdateRoleQuota updates quota for a role
func (q *QuotaService) UpdateRoleQuota(ctx context.Context, roleID string, quota *RoleQuota) error {
	// Find the record by role_id
	rec, err := database.GetByField(ctx, q.db, "ext_cloudstorage_role_quotas", "role_id", roleID)
	if err != nil {
		return err
	}

	_, err = q.db.Update(ctx, "ext_cloudstorage_role_quotas", rec.ID, map[string]any{
		"role_name":           quota.RoleName,
		"max_storage_bytes":   quota.MaxStorageBytes,
		"max_bandwidth_bytes": quota.MaxBandwidthBytes,
		"max_upload_size":     quota.MaxUploadSize,
		"max_files_count":     quota.MaxFilesCount,
		"allowed_extensions":  stringPtrOrNil(quota.AllowedExtensions),
		"blocked_extensions":  stringPtrOrNil(quota.BlockedExtensions),
	})
	return err
}

// SyncRoleQuotaFromIAM syncs quota when a role is created or updated in IAM
func (q *QuotaService) SyncRoleQuotaFromIAM(ctx context.Context, roleName string, roleID string) error {
	// Check if quota already exists for this role by ID
	_, err := database.GetByField(ctx, q.db, "ext_cloudstorage_role_quotas", "role_id", roleID)
	if err == nil {
		return nil // Already exists
	}

	// Try by role name
	existingRec, err := database.GetByField(ctx, q.db, "ext_cloudstorage_role_quotas", "role_name", roleName)
	if err == nil {
		// Quota exists with different role_id, update it
		existingRoleID := stringVal(existingRec.Data["role_id"])
		if existingRoleID != roleID {
			_, err = q.db.Update(ctx, "ext_cloudstorage_role_quotas", existingRec.ID, map[string]any{
				"role_id": roleID,
			})
			return err
		}
		return nil
	}

	// Create default quota for new role
	defaultMaxStorage := int64(5 * 1024 * 1024 * 1024)    // 5GB default
	defaultMaxBandwidth := int64(10 * 1024 * 1024 * 1024)  // 10GB default
	defaultMaxUpload := int64(100 * 1024 * 1024)            // 100MB per file
	defaultMaxFiles := int64(1000)                          // 1000 files default

	// Special defaults for admin role
	if roleName == "admin" {
		defaultMaxStorage = 100 * 1024 * 1024 * 1024    // 100GB for admin
		defaultMaxBandwidth = 1000 * 1024 * 1024 * 1024 // 1TB for admin
		defaultMaxUpload = 1024 * 1024 * 1024            // 1GB per file
		defaultMaxFiles = 0                              // Unlimited files
	}

	_, err = q.db.Create(ctx, "ext_cloudstorage_role_quotas", map[string]any{
		"id":                  uuid.New().String(),
		"role_id":             roleID,
		"role_name":           roleName,
		"max_storage_bytes":   defaultMaxStorage,
		"max_bandwidth_bytes": defaultMaxBandwidth,
		"max_upload_size":     defaultMaxUpload,
		"max_files_count":     defaultMaxFiles,
	})

	return err
}

// CreateUserOverride creates a custom quota override for a user
func (q *QuotaService) CreateUserOverride(ctx context.Context, override *UserQuotaOverride) error {
	override.ID = uuid.New().String()
	override.CreatedAt = apptime.NowTime()
	override.UpdatedAt = apptime.NowTime()

	data := map[string]any{
		"id":                  override.ID,
		"user_id":             override.UserID,
		"max_storage_bytes":   override.MaxStorageBytes,
		"max_bandwidth_bytes": override.MaxBandwidthBytes,
		"max_upload_size":     override.MaxUploadSize,
		"max_files_count":     override.MaxFilesCount,
		"allowed_extensions":  override.AllowedExtensions,
		"blocked_extensions":  override.BlockedExtensions,
		"reason":              override.Reason,
		"expires_at":          nullTimeToAny(override.ExpiresAt),
		"created_by":          override.CreatedBy,
	}

	_, err := q.db.Create(ctx, "ext_cloudstorage_user_quota_overrides", data)
	return err
}

// UpdateStorageUsage updates the storage usage for a user after upload/delete
func (q *QuotaService) UpdateStorageUsage(ctx context.Context, userID string, sizeChange int64) error {
	// Check if quota record exists
	existing, err := database.GetByField(ctx, q.db, "ext_cloudstorage_storage_quotas", "user_id", userID)

	if errors.Is(err, database.ErrNotFound) {
		// Create new quota record
		storageUsed := sizeChange
		if storageUsed < 0 {
			storageUsed = 0
		}

		_, err := q.db.Create(ctx, "ext_cloudstorage_storage_quotas", map[string]any{
			"id":                  uuid.New().String(),
			"user_id":             userID,
			"max_storage_bytes":   int64(5368709120),  // 5GB default
			"max_bandwidth_bytes": int64(10737418240), // 10GB default
			"storage_used":        storageUsed,
			"bandwidth_used":      int64(0),
		})
		return err
	} else if err != nil {
		return fmt.Errorf("failed to get user quota: %w", err)
	}

	// Update existing quota using ExecRaw for arithmetic operations
	_ = existing
	if sizeChange >= 0 {
		_, err = q.db.ExecRaw(ctx, `
			UPDATE ext_cloudstorage_storage_quotas
			SET storage_used = storage_used + ?, updated_at = ?
			WHERE user_id = ?`,
			sizeChange, apptime.NowString(), userID,
		)
	} else {
		_, err = q.db.ExecRaw(ctx, `
			UPDATE ext_cloudstorage_storage_quotas
			SET storage_used = MAX(0, storage_used - ?), updated_at = ?
			WHERE user_id = ?`,
			-sizeChange, apptime.NowString(), userID,
		)
	}
	return err
}

// UpdateBandwidthUsage updates the bandwidth usage for a user after download
func (q *QuotaService) UpdateBandwidthUsage(ctx context.Context, userID string, bytes int64) error {
	// Check if quota record exists
	existing, err := database.GetByField(ctx, q.db, "ext_cloudstorage_storage_quotas", "user_id", userID)

	if errors.Is(err, database.ErrNotFound) {
		// Create new quota record
		_, err := q.db.Create(ctx, "ext_cloudstorage_storage_quotas", map[string]any{
			"id":                  uuid.New().String(),
			"user_id":             userID,
			"max_storage_bytes":   int64(5368709120),  // 5GB default
			"max_bandwidth_bytes": int64(10737418240), // 10GB default
			"storage_used":        int64(0),
			"bandwidth_used":      bytes,
		})
		return err
	} else if err != nil {
		return fmt.Errorf("failed to get user quota: %w", err)
	}

	// Update existing quota
	_ = existing
	_, err = q.db.ExecRaw(ctx, `
		UPDATE ext_cloudstorage_storage_quotas
		SET bandwidth_used = bandwidth_used + ?, updated_at = ?
		WHERE user_id = ?`,
		bytes, apptime.NowString(), userID,
	)
	return err
}

// CheckStorageQuota checks if user has enough storage quota (compatibility method)
func (q *QuotaService) CheckStorageQuota(ctx context.Context, userID string, fileSize int64) error {
	return q.CheckUploadAllowed(ctx, userID, fileSize, "")
}

// GetOrCreateQuota gets or creates a default quota for a user (compatibility method)
func (q *QuotaService) GetOrCreateQuota(ctx context.Context, userID string) (*StorageQuota, error) {
	// Check if quota exists
	rec, err := database.GetByField(ctx, q.db, "ext_cloudstorage_storage_quotas", "user_id", userID)
	if err == nil {
		quota := recordToStorageQuota(rec)
		return &quota, nil
	}

	if !errors.Is(err, database.ErrNotFound) {
		return nil, fmt.Errorf("failed to get quota: %w", err)
	}

	// Create new quota
	newQuota := &StorageQuota{
		ID:                uuid.New().String(),
		UserID:            userID,
		MaxStorageBytes:   5368709120,  // 5GB default
		MaxBandwidthBytes: 10737418240, // 10GB default
		StorageUsed:       0,
		BandwidthUsed:     0,
		CreatedAt:         apptime.NowTime(),
		UpdatedAt:         apptime.NowTime(),
	}

	_, err = q.db.Create(ctx, "ext_cloudstorage_storage_quotas", map[string]any{
		"id":                  newQuota.ID,
		"user_id":             newQuota.UserID,
		"max_storage_bytes":   newQuota.MaxStorageBytes,
		"max_bandwidth_bytes": newQuota.MaxBandwidthBytes,
		"storage_used":        newQuota.StorageUsed,
		"bandwidth_used":      newQuota.BandwidthUsed,
	})

	if err != nil {
		return nil, fmt.Errorf("failed to create quota: %w", err)
	}

	return newQuota, nil
}

// GetQuotaStats retrieves quota statistics for a user
func (q *QuotaService) GetQuotaStats(ctx context.Context, userID string) (*QuotaStats, error) {
	effectiveQuota, err := q.GetUserQuota(ctx, userID)
	if err != nil {
		return nil, err
	}

	// Calculate percentages safely (avoid division by zero)
	storagePercentage := float64(0)
	if effectiveQuota.MaxStorageBytes > 0 {
		storagePercentage = float64(effectiveQuota.StorageUsed) / float64(effectiveQuota.MaxStorageBytes) * 100
	}

	bandwidthPercentage := float64(0)
	if effectiveQuota.MaxBandwidthBytes > 0 {
		bandwidthPercentage = float64(effectiveQuota.BandwidthUsed) / float64(effectiveQuota.MaxBandwidthBytes) * 100
	}

	return &QuotaStats{
		StorageUsed:         effectiveQuota.StorageUsed,
		StorageLimit:        effectiveQuota.MaxStorageBytes,
		StoragePercentage:   storagePercentage,
		BandwidthUsed:       effectiveQuota.BandwidthUsed,
		BandwidthLimit:      effectiveQuota.MaxBandwidthBytes,
		BandwidthPercentage: bandwidthPercentage,
	}, nil
}
