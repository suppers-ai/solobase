package cloudstorage

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"strings"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
)

// QuotaService manages storage quotas and limits
type QuotaService struct {
	sqlDB   *sql.DB
	queries *db.Queries
}

// NewQuotaService creates a new quota service
func NewQuotaService(sqlDB *sql.DB) *QuotaService {
	return &QuotaService{
		sqlDB:   sqlDB,
		queries: db.New(sqlDB),
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
		var count int64
		err := q.sqlDB.QueryRowContext(ctx,
			"SELECT COUNT(*) FROM ext_cloudstorage_role_quotas WHERE role_name = ?",
			quota.RoleName).Scan(&count)
		if err == nil && count > 0 {
			continue // Already exists
		}

		// Get role ID from database directly
		var roleID string
		err = q.sqlDB.QueryRowContext(ctx,
			"SELECT id FROM iam_roles WHERE name = ?",
			quota.RoleName).Scan(&roleID)
		if err != nil {
			continue // Role doesn't exist, skip
		}
		quota.RoleID = roleID

		// Create quota
		quota.ID = uuid.New().String()
		quota.CreatedAt = apptime.NowTime()
		quota.UpdatedAt = apptime.NowTime()

		_, err = q.queries.CreateRoleQuota(ctx, db.CreateRoleQuotaParams{
			ID:                quota.ID,
			RoleID:            quota.RoleID,
			RoleName:          quota.RoleName,
			MaxStorageBytes:   quota.MaxStorageBytes,
			MaxBandwidthBytes: quota.MaxBandwidthBytes,
			MaxUploadSize:     quota.MaxUploadSize,
			MaxFilesCount:     quota.MaxFilesCount,
			AllowedExtensions: stringPtrOrNil(quota.AllowedExtensions),
			BlockedExtensions: stringPtrOrNil(quota.BlockedExtensions),
			CreatedAt:         apptime.Format(quota.CreatedAt),
			UpdatedAt:         apptime.Format(quota.UpdatedAt),
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
	dbOverride, err := q.queries.GetActiveUserQuotaOverride(ctx, db.GetActiveUserQuotaOverrideParams{
		UserID:    userID,
		ExpiresAt: apptime.NullTime{Time: apptime.NowTime(), Valid: true},
	})
	if err == nil {
		override = dbUserQuotaOverrideToModel(dbOverride)
	}

	// Get user's roles from database directly
	rows, err := q.sqlDB.QueryContext(ctx, `
		SELECT r.name as role_name
		FROM iam_user_roles ur
		JOIN iam_roles r ON ur.role_id = r.id
		WHERE ur.user_id = ?
	`, userID)
	if err != nil {
		return nil, fmt.Errorf("failed to get user roles: %w", err)
	}
	defer rows.Close()

	var roleNames []string
	for rows.Next() {
		var roleName string
		if err := rows.Scan(&roleName); err != nil {
			return nil, err
		}
		roleNames = append(roleNames, roleName)
	}

	// Get quotas for all user's roles and take the maximum values
	var roleQuotas []RoleQuota
	for _, roleName := range roleNames {
		dbQuota, err := q.queries.GetRoleQuotaByRoleName(ctx, roleName)
		if err == nil {
			roleQuotas = append(roleQuotas, dbRoleQuotaToModel(dbQuota))
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
	var storageUsed, bandwidthUsed int64
	q.sqlDB.QueryRowContext(ctx,
		"SELECT COALESCE(storage_used, 0), COALESCE(bandwidth_used, 0) FROM ext_cloudstorage_storage_quotas WHERE user_id = ?",
		userID).Scan(&storageUsed, &bandwidthUsed)

	// Get file count
	var fileCount int64
	q.sqlDB.QueryRowContext(ctx,
		"SELECT COUNT(*) FROM storage_objects WHERE owner_id = ?",
		userID).Scan(&fileCount)

	effective.StorageUsed = storageUsed
	effective.BandwidthUsed = bandwidthUsed
	effective.FilesUsed = fileCount

	return effective, nil
}

// EffectiveQuota represents the calculated quota for a user
type EffectiveQuota struct {
	UserID            string `json:"userId"`
	MaxStorageBytes   int64  `json:"maxStorageBytes"`
	MaxBandwidthBytes int64  `json:"maxBandwidthBytes"`
	MaxUploadSize     int64  `json:"maxUploadSize"`
	MaxFilesCount     int64  `json:"maxFilesCount"`
	AllowedExtensions string `json:"allowedExtensions"`
	BlockedExtensions string `json:"blockedExtensions"`
	StorageUsed       int64  `json:"storageUsed"`
	BandwidthUsed     int64  `json:"bandwidthUsed"`
	FilesUsed         int64  `json:"filesUsed"`
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
	return q.queries.UpdateRoleQuota(ctx, db.UpdateRoleQuotaParams{
		RoleName:          quota.RoleName,
		MaxStorageBytes:   quota.MaxStorageBytes,
		MaxBandwidthBytes: quota.MaxBandwidthBytes,
		MaxUploadSize:     quota.MaxUploadSize,
		MaxFilesCount:     quota.MaxFilesCount,
		AllowedExtensions: stringPtrOrNil(quota.AllowedExtensions),
		BlockedExtensions: stringPtrOrNil(quota.BlockedExtensions),
		UpdatedAt:   apptime.NowString(),
		ID:                roleID,
	})
}

// SyncRoleQuotaFromIAM syncs quota for a role when IAM role is created or updated
func (q *QuotaService) SyncRoleQuotaFromIAM(ctx context.Context, roleName string, roleID string) error {
	// Check if quota already exists for this role
	_, err := q.queries.GetRoleQuotaByRoleID(ctx, roleID)
	if err == nil {
		return nil // Already exists
	}

	// Try by role name
	existingQuota, err := q.queries.GetRoleQuotaByRoleName(ctx, roleName)
	if err == nil {
		// Quota exists with different role_id, update it
		if existingQuota.RoleID != roleID {
			_, err = q.sqlDB.ExecContext(ctx,
				"UPDATE ext_cloudstorage_role_quotas SET role_id = ? WHERE id = ?",
				roleID, existingQuota.ID)
			return err
		}
		return nil
	}

	// Create default quota for new role
	defaultQuota := &RoleQuota{
		ID:                uuid.New().String(),
		RoleID:            roleID,
		RoleName:          roleName,
		MaxStorageBytes:   5 * 1024 * 1024 * 1024,  // 5GB default
		MaxBandwidthBytes: 10 * 1024 * 1024 * 1024, // 10GB default
		MaxUploadSize:     100 * 1024 * 1024,       // 100MB per file
		MaxFilesCount:     1000,                    // 1000 files default
		CreatedAt:         apptime.NowTime(),
		UpdatedAt:         apptime.NowTime(),
	}

	// Special defaults for admin role
	if roleName == "admin" {
		defaultQuota.MaxStorageBytes = 100 * 1024 * 1024 * 1024    // 100GB for admin
		defaultQuota.MaxBandwidthBytes = 1000 * 1024 * 1024 * 1024 // 1TB for admin
		defaultQuota.MaxUploadSize = 1024 * 1024 * 1024            // 1GB per file
		defaultQuota.MaxFilesCount = 0                             // Unlimited files
	}

	_, err = q.queries.CreateRoleQuota(ctx, db.CreateRoleQuotaParams{
		ID:                defaultQuota.ID,
		RoleID:            defaultQuota.RoleID,
		RoleName:          defaultQuota.RoleName,
		MaxStorageBytes:   defaultQuota.MaxStorageBytes,
		MaxBandwidthBytes: defaultQuota.MaxBandwidthBytes,
		MaxUploadSize:     defaultQuota.MaxUploadSize,
		MaxFilesCount:     defaultQuota.MaxFilesCount,
		AllowedExtensions: stringPtrOrNil(defaultQuota.AllowedExtensions),
		BlockedExtensions: stringPtrOrNil(defaultQuota.BlockedExtensions),
		CreatedAt:         apptime.Format(defaultQuota.CreatedAt),
		UpdatedAt:         apptime.Format(defaultQuota.UpdatedAt),
	})

	return err
}

// CreateUserOverride creates a custom quota override for a user
func (q *QuotaService) CreateUserOverride(ctx context.Context, override *UserQuotaOverride) error {
	override.ID = uuid.New().String()
	override.CreatedAt = apptime.NowTime()
	override.UpdatedAt = apptime.NowTime()

	_, err := q.queries.CreateUserQuotaOverride(ctx, db.CreateUserQuotaOverrideParams{
		ID:                override.ID,
		UserID:            override.UserID,
		MaxStorageBytes:   override.MaxStorageBytes,
		MaxBandwidthBytes: override.MaxBandwidthBytes,
		MaxUploadSize:     override.MaxUploadSize,
		MaxFilesCount:     override.MaxFilesCount,
		AllowedExtensions: override.AllowedExtensions,
		BlockedExtensions: override.BlockedExtensions,
		Reason:            override.Reason,
		ExpiresAt:         override.ExpiresAt,
		CreatedBy:         override.CreatedBy,
		CreatedAt:         apptime.Format(override.CreatedAt),
		UpdatedAt:         apptime.Format(override.UpdatedAt),
	})

	return err
}

// Helper functions
func getFileExtension(fileName string) string {
	parts := strings.Split(fileName, ".")
	if len(parts) > 1 {
		return parts[len(parts)-1]
	}
	return ""
}

func mergeExtensions(ext1, ext2 string) string {
	if ext1 == "" {
		return ext2
	}
	if ext2 == "" {
		return ext1
	}

	// Merge and deduplicate
	allExts := make(map[string]bool)
	for _, ext := range strings.Split(ext1, ",") {
		allExts[strings.TrimSpace(ext)] = true
	}
	for _, ext := range strings.Split(ext2, ",") {
		allExts[strings.TrimSpace(ext)] = true
	}

	result := []string{}
	for ext := range allExts {
		if ext != "" {
			result = append(result, ext)
		}
	}

	return strings.Join(result, ",")
}

func intersectExtensions(ext1, ext2 string) string {
	// Only keep extensions that are in both lists
	exts1 := make(map[string]bool)
	for _, ext := range strings.Split(ext1, ",") {
		exts1[strings.TrimSpace(ext)] = true
	}

	result := []string{}
	for _, ext := range strings.Split(ext2, ",") {
		ext = strings.TrimSpace(ext)
		if exts1[ext] {
			result = append(result, ext)
		}
	}

	return strings.Join(result, ",")
}

// UpdateStorageUsage updates the storage usage for a user after upload/delete
func (q *QuotaService) UpdateStorageUsage(ctx context.Context, userID string, sizeChange int64) error {
	// Check if quota record exists
	var quotaID string
	err := q.sqlDB.QueryRowContext(ctx,
		"SELECT id FROM ext_cloudstorage_storage_quotas WHERE user_id = ?",
		userID).Scan(&quotaID)

	if errors.Is(err, sql.ErrNoRows) {
		// Create new quota record
		quota := &StorageQuota{
			ID:                uuid.New().String(),
			UserID:            userID,
			MaxStorageBytes:   5368709120,  // 5GB default
			MaxBandwidthBytes: 10737418240, // 10GB default
			StorageUsed:       sizeChange,
			BandwidthUsed:     0,
			CreatedAt:         apptime.NowTime(),
			UpdatedAt:         apptime.NowTime(),
		}
		if quota.StorageUsed < 0 {
			quota.StorageUsed = 0
		}

		_, err := q.queries.CreateStorageQuota(ctx, db.CreateStorageQuotaParams{
			ID:               quota.ID,
			UserID:           quota.UserID,
			MaxStorageBytes:  quota.MaxStorageBytes,
			MaxBandwidthBytes: quota.MaxBandwidthBytes,
			StorageUsed:      quota.StorageUsed,
			BandwidthUsed:    quota.BandwidthUsed,
			ResetBandwidthAt: apptime.NullTime{},
			CreatedAt:        apptime.Format(quota.CreatedAt),
			UpdatedAt:        apptime.Format(quota.UpdatedAt),
		})
		return err
	} else if err != nil {
		return fmt.Errorf("failed to get user quota: %w", err)
	}

	// Update existing quota
	if sizeChange >= 0 {
		return q.queries.IncrementStorageUsed(ctx, db.IncrementStorageUsedParams{
			StorageUsed: sizeChange,
			UpdatedAt:   apptime.NowString(),
			UserID:      userID,
		})
	} else {
		return q.queries.DecrementStorageUsed(ctx, db.DecrementStorageUsedParams{
			StorageUsed: -sizeChange,
			UpdatedAt:   apptime.NowString(),
			UserID:      userID,
		})
	}
}

// UpdateBandwidthUsage updates the bandwidth usage for a user after download
func (q *QuotaService) UpdateBandwidthUsage(ctx context.Context, userID string, bytes int64) error {
	// Check if quota record exists
	var quotaID string
	err := q.sqlDB.QueryRowContext(ctx,
		"SELECT id FROM ext_cloudstorage_storage_quotas WHERE user_id = ?",
		userID).Scan(&quotaID)

	if errors.Is(err, sql.ErrNoRows) {
		// Create new quota record
		quota := &StorageQuota{
			ID:                uuid.New().String(),
			UserID:            userID,
			MaxStorageBytes:   5368709120,  // 5GB default
			MaxBandwidthBytes: 10737418240, // 10GB default
			StorageUsed:       0,
			BandwidthUsed:     bytes,
			CreatedAt:         apptime.NowTime(),
			UpdatedAt:         apptime.NowTime(),
		}

		_, err := q.queries.CreateStorageQuota(ctx, db.CreateStorageQuotaParams{
			ID:               quota.ID,
			UserID:           quota.UserID,
			MaxStorageBytes:  quota.MaxStorageBytes,
			MaxBandwidthBytes: quota.MaxBandwidthBytes,
			StorageUsed:      quota.StorageUsed,
			BandwidthUsed:    quota.BandwidthUsed,
			ResetBandwidthAt: apptime.NullTime{},
			CreatedAt:        apptime.Format(quota.CreatedAt),
			UpdatedAt:        apptime.Format(quota.UpdatedAt),
		})
		return err
	} else if err != nil {
		return fmt.Errorf("failed to get user quota: %w", err)
	}

	// Update existing quota
	return q.queries.IncrementBandwidthUsed(ctx, db.IncrementBandwidthUsedParams{
		BandwidthUsed: bytes,
		UpdatedAt:   apptime.NowString(),
		UserID:        userID,
	})
}

// CheckStorageQuota checks if user has enough storage quota (compatibility method)
func (q *QuotaService) CheckStorageQuota(ctx context.Context, userID string, fileSize int64) error {
	return q.CheckUploadAllowed(ctx, userID, fileSize, "")
}

// GetOrCreateQuota gets or creates a default quota for a user (compatibility method)
func (q *QuotaService) GetOrCreateQuota(ctx context.Context, userID string) (*StorageQuota, error) {
	// Check if quota exists
	dbQuota, err := q.queries.GetStorageQuotaByUserID(ctx, userID)
	if err == nil {
		return dbStorageQuotaToModel(dbQuota), nil
	}

	if !errors.Is(err, sql.ErrNoRows) {
		return nil, fmt.Errorf("failed to get quota: %w", err)
	}

	// Create new quota
	quota := &StorageQuota{
		ID:                uuid.New().String(),
		UserID:            userID,
		MaxStorageBytes:   5368709120,  // 5GB default
		MaxBandwidthBytes: 10737418240, // 10GB default
		StorageUsed:       0,
		BandwidthUsed:     0,
		CreatedAt:         apptime.NowTime(),
		UpdatedAt:         apptime.NowTime(),
	}

	_, err = q.queries.CreateStorageQuota(ctx, db.CreateStorageQuotaParams{
		ID:               quota.ID,
		UserID:           quota.UserID,
		MaxStorageBytes:  quota.MaxStorageBytes,
		MaxBandwidthBytes: quota.MaxBandwidthBytes,
		StorageUsed:      quota.StorageUsed,
		BandwidthUsed:    quota.BandwidthUsed,
		ResetBandwidthAt: apptime.NullTime{},
		CreatedAt:        apptime.Format(quota.CreatedAt),
		UpdatedAt:        apptime.Format(quota.UpdatedAt),
	})

	if err != nil {
		return nil, fmt.Errorf("failed to create quota: %w", err)
	}

	return quota, nil
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

// QuotaStats represents quota usage statistics
type QuotaStats struct {
	StorageUsed         int64   `json:"storageUsed"`
	StorageLimit        int64   `json:"storageLimit"`
	StoragePercentage   float64 `json:"storagePercentage"`
	BandwidthUsed       int64   `json:"bandwidthUsed"`
	BandwidthLimit      int64   `json:"bandwidthLimit"`
	BandwidthPercentage float64 `json:"bandwidthPercentage"`
}

// Helper functions for conversions
func stringPtrOrNil(s string) *string {
	if s == "" {
		return nil
	}
	return &s
}

func timePtr(t apptime.Time) *apptime.Time {
	return &t
}

func dbRoleQuotaToModel(dbQuota db.ExtCloudstorageRoleQuota) RoleQuota {
	quota := RoleQuota{
		ID:                dbQuota.ID,
		RoleID:            dbQuota.RoleID,
		RoleName:          dbQuota.RoleName,
		MaxStorageBytes:   dbQuota.MaxStorageBytes,
		MaxBandwidthBytes: dbQuota.MaxBandwidthBytes,
		MaxUploadSize:     dbQuota.MaxUploadSize,
		MaxFilesCount:     dbQuota.MaxFilesCount,
		CreatedAt:         apptime.NewTime(apptime.MustParse(dbQuota.CreatedAt)),
		UpdatedAt:         apptime.NewTime(apptime.MustParse(dbQuota.UpdatedAt)),
	}
	if dbQuota.AllowedExtensions != nil {
		quota.AllowedExtensions = *dbQuota.AllowedExtensions
	}
	if dbQuota.BlockedExtensions != nil {
		quota.BlockedExtensions = *dbQuota.BlockedExtensions
	}
	return quota
}

func dbUserQuotaOverrideToModel(dbOverride db.ExtCloudstorageUserQuotaOverride) *UserQuotaOverride {
	override := &UserQuotaOverride{
		ID:                dbOverride.ID,
		UserID:            dbOverride.UserID,
		MaxStorageBytes:   dbOverride.MaxStorageBytes,
		MaxBandwidthBytes: dbOverride.MaxBandwidthBytes,
		MaxUploadSize:     dbOverride.MaxUploadSize,
		MaxFilesCount:     dbOverride.MaxFilesCount,
		AllowedExtensions: dbOverride.AllowedExtensions,
		BlockedExtensions: dbOverride.BlockedExtensions,
		Reason:            dbOverride.Reason,
		ExpiresAt:         apptime.FromTimePtr(dbOverride.ExpiresAt.ToTimePtr()),
		CreatedBy:         dbOverride.CreatedBy,
		CreatedAt:         apptime.NewTime(apptime.MustParse(dbOverride.CreatedAt)),
		UpdatedAt:         apptime.NewTime(apptime.MustParse(dbOverride.UpdatedAt)),
	}
	return override
}

func dbStorageQuotaToModel(dbQuota db.ExtCloudstorageStorageQuota) *StorageQuota {
	return &StorageQuota{
		ID:                dbQuota.ID,
		UserID:            dbQuota.UserID,
		MaxStorageBytes:   dbQuota.MaxStorageBytes,
		MaxBandwidthBytes: dbQuota.MaxBandwidthBytes,
		StorageUsed:       dbQuota.StorageUsed,
		BandwidthUsed:     dbQuota.BandwidthUsed,
		ResetBandwidthAt:  apptime.FromTimePtr(dbQuota.ResetBandwidthAt.ToTimePtr()),
		CreatedAt:         apptime.NewTime(apptime.MustParse(dbQuota.CreatedAt)),
		UpdatedAt:         apptime.NewTime(apptime.MustParse(dbQuota.UpdatedAt)),
	}
}
