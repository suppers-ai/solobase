package cloudstorage

import (
	"context"
	"fmt"
	"strings"
	
	"gorm.io/gorm"
	pkgstorage "github.com/suppers-ai/storage"
)

// QuotaService manages storage quotas and limits
type QuotaService struct {
	db *gorm.DB
}

// NewQuotaService creates a new quota service
func NewQuotaService(db *gorm.DB) *QuotaService {
	return &QuotaService{
		db: db,
	}
}

// InitializeDefaultQuotas creates default quotas for system roles
func (q *QuotaService) InitializeDefaultQuotas() error {
	defaultQuotas := []RoleQuota{
		{
			RoleName:          "admin",
			MaxStorageBytes:   107374182400, // 100GB
			MaxBandwidthBytes: 1099511627776, // 1TB
			MaxUploadSize:     5368709120,   // 5GB
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
			MaxUploadSize:     0,            // No uploads
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
		var existing RoleQuota
		if err := q.db.Where("role_name = ?", quota.RoleName).First(&existing).Error; err == nil {
			continue // Already exists
		}
		
		// Get role ID from database directly
		var role struct {
			ID string
		}
		err := q.db.Table("iam_roles").Where("name = ?", quota.RoleName).Select("id").First(&role).Error
		if err != nil {
			continue // Role doesn't exist, skip
		}
		quota.RoleID = role.ID
		
		if err := q.db.Create(&quota).Error; err != nil {
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
	var override UserQuotaOverride
	hasOverride := false
	
	err := q.db.Where("user_id = ? AND (expires_at IS NULL OR expires_at > NOW())", userID).
		First(&override).Error
	if err == nil {
		hasOverride = true
	}
	
	// Get user's roles from database directly
	var userRoles []struct {
		RoleName string `gorm:"column:role_name"`
	}
	err = q.db.Table("iam_user_roles ur").
		Joins("JOIN iam_roles r ON ur.role_id = r.id").
		Where("ur.user_id = ?", userID).
		Select("r.name as role_name").
		Find(&userRoles).Error
	
	if err != nil {
		return nil, fmt.Errorf("failed to get user roles: %w", err)
	}
	
	// Get quotas for all user's roles and take the maximum values
	var roleQuotas []RoleQuota
	if len(userRoles) > 0 {
		roleNames := make([]string, len(userRoles))
		for i, r := range userRoles {
			roleNames[i] = r.RoleName
		}
		q.db.Where("role_name IN ?", roleNames).Find(&roleQuotas)
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
	if hasOverride {
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
	
	// Get current usage and file count in a single query
	var usage struct {
		StorageUsed   int64
		BandwidthUsed int64
		FileCount     int64
	}
	
	// Get storage usage
	q.db.Table("ext_cloudstorage_storage_quotas").
		Where("user_id = ?", userID).
		Select("storage_used, bandwidth_used").
		Scan(&usage)
	
	// Get file count
	q.db.Model(&pkgstorage.StorageObject{}).
		Where("owner_id = ?", userID).
		Count(&usage.FileCount)
	
	effective.StorageUsed = usage.StorageUsed
	effective.BandwidthUsed = usage.BandwidthUsed
	effective.FilesUsed = usage.FileCount
	
	return effective, nil
}

// EffectiveQuota represents the calculated quota for a user
type EffectiveQuota struct {
	UserID            string `json:"user_id"`
	MaxStorageBytes   int64  `json:"max_storage_bytes"`
	MaxBandwidthBytes int64  `json:"max_bandwidth_bytes"`
	MaxUploadSize     int64  `json:"max_upload_size"`
	MaxFilesCount     int64  `json:"max_files_count"`
	AllowedExtensions string `json:"allowed_extensions"`
	BlockedExtensions string `json:"blocked_extensions"`
	StorageUsed       int64  `json:"storage_used"`
	BandwidthUsed     int64  `json:"bandwidth_used"`
	FilesUsed         int64  `json:"files_used"`
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
	quota.RoleID = roleID
	return q.db.Save(quota).Error
}

// SyncRoleQuotaFromIAM syncs quota for a role when IAM role is created or updated
func (q *QuotaService) SyncRoleQuotaFromIAM(ctx context.Context, roleName string, roleID string) error {
	// Check if quota already exists for this role
	var existingQuota RoleQuota
	err := q.db.Where("role_id = ? OR role_name = ?", roleID, roleName).First(&existingQuota).Error
	
	if err == nil {
		// Quota exists, update role_id if needed
		if existingQuota.RoleID != roleID {
			existingQuota.RoleID = roleID
			return q.db.Save(&existingQuota).Error
		}
		return nil
	}
	
	// Create default quota for new role
	defaultQuota := &RoleQuota{
		RoleID:            roleID,
		RoleName:          roleName,
		MaxStorageBytes:   5 * 1024 * 1024 * 1024,  // 5GB default
		MaxBandwidthBytes: 10 * 1024 * 1024 * 1024, // 10GB default
		MaxUploadSize:     100 * 1024 * 1024,       // 100MB per file
		MaxFilesCount:     1000,                    // 1000 files default
	}
	
	// Special defaults for admin role
	if roleName == "admin" {
		defaultQuota.MaxStorageBytes = 100 * 1024 * 1024 * 1024   // 100GB for admin
		defaultQuota.MaxBandwidthBytes = 1000 * 1024 * 1024 * 1024 // 1TB for admin
		defaultQuota.MaxUploadSize = 1024 * 1024 * 1024           // 1GB per file
		defaultQuota.MaxFilesCount = 0                            // Unlimited files
	}
	
	return q.db.Create(defaultQuota).Error
}

// CreateUserOverride creates a custom quota override for a user
func (q *QuotaService) CreateUserOverride(ctx context.Context, override *UserQuotaOverride) error {
	return q.db.Create(override).Error
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
	var quota StorageQuota
	
	// Find or create user quota record
	err := q.db.Where("user_id = ?", userID).FirstOrCreate(&quota, StorageQuota{
		UserID: userID,
	}).Error
	if err != nil {
		return fmt.Errorf("failed to get user quota: %w", err)
	}
	
	// Update storage used
	quota.StorageUsed += sizeChange
	if quota.StorageUsed < 0 {
		quota.StorageUsed = 0
	}
	
	// Save updated quota
	return q.db.Save(&quota).Error
}

// UpdateBandwidthUsage updates the bandwidth usage for a user after download
func (q *QuotaService) UpdateBandwidthUsage(ctx context.Context, userID string, bytes int64) error {
	var quota StorageQuota
	
	// Find or create user quota record
	err := q.db.Where("user_id = ?", userID).FirstOrCreate(&quota, StorageQuota{
		UserID: userID,
	}).Error
	if err != nil {
		return fmt.Errorf("failed to get user quota: %w", err)
	}
	
	// Update bandwidth used
	quota.BandwidthUsed += bytes
	
	// Save updated quota
	return q.db.Save(&quota).Error
}

// CheckStorageQuota checks if user has enough storage quota (compatibility method)
func (q *QuotaService) CheckStorageQuota(ctx context.Context, userID string, fileSize int64) error {
	return q.CheckUploadAllowed(ctx, userID, fileSize, "")
}

// GetOrCreateQuota gets or creates a default quota for a user (compatibility method)
func (q *QuotaService) GetOrCreateQuota(ctx context.Context, userID string) (*StorageQuota, error) {
	var quota StorageQuota
	
	err := q.db.Where("user_id = ?", userID).FirstOrCreate(&quota, StorageQuota{
		UserID:            userID,
		MaxStorageBytes:   5368709120,  // 5GB default
		MaxBandwidthBytes: 10737418240, // 10GB default
	}).Error
	
	if err != nil {
		return nil, fmt.Errorf("failed to get or create quota: %w", err)
	}
	
	return &quota, nil
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
	StorageUsed         int64   `json:"storage_used"`
	StorageLimit        int64   `json:"storage_limit"`
	StoragePercentage   float64 `json:"storage_percentage"`
	BandwidthUsed       int64   `json:"bandwidth_used"`
	BandwidthLimit      int64   `json:"bandwidth_limit"`
	BandwidthPercentage float64 `json:"bandwidth_percentage"`
}