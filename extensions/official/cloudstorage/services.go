package cloudstorage

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"net"
	"time"

	"github.com/google/uuid"
	pkgstorage "github.com/suppers-ai/storage"
	"golang.org/x/crypto/bcrypt"
	"gorm.io/datatypes"
	"gorm.io/gorm"
)

// ShareService manages file sharing functionality
type ShareService struct {
	db      *gorm.DB
	manager interface{} // Storage manager interface, can be nil
}

// NewShareService creates a new share service
func NewShareService(db *gorm.DB, manager interface{}) *ShareService {
	return &ShareService{
		db:      db,
		manager: manager,
	}
}

// CreateShare creates a shareable link for a storage object
func (s *ShareService) CreateShare(ctx context.Context, objectID, userID string, opts ShareOptions) (*StorageShare, error) {
	// Verify object exists
	var obj pkgstorage.StorageObject
	if err := s.db.Where("id = ?", objectID).First(&obj).Error; err != nil {
		return nil, fmt.Errorf("object not found: %w", err)
	}

	share := &StorageShare{
		ID:                uuid.New().String(),
		ObjectID:          objectID,
		CreatedBy:         userID,
		PermissionLevel:   opts.PermissionLevel,
		InheritToChildren: opts.InheritToChildren,
		IsPublic:          opts.IsPublic,
		ExpiresAt:         opts.ExpiresAt,
	}

	// Set sharing target
	if opts.SharedWithUserID != "" {
		share.SharedWithUserID = &opts.SharedWithUserID
	} else if opts.SharedWithEmail != "" {
		share.SharedWithEmail = &opts.SharedWithEmail
	} else if opts.GenerateToken {
		// Generate unique share token
		tokenBytes := make([]byte, 16)
		if _, err := rand.Read(tokenBytes); err != nil {
			return nil, fmt.Errorf("failed to generate token: %w", err)
		}
		token := hex.EncodeToString(tokenBytes)
		share.ShareToken = &token
	}

	if err := s.db.Create(share).Error; err != nil {
		return nil, fmt.Errorf("failed to create share: %w", err)
	}

	return share, nil
}

// GetShareByToken retrieves a share by its token
func (s *ShareService) GetShareByToken(ctx context.Context, token string) (*StorageShare, error) {
	var share StorageShare
	if err := s.db.Where("share_token = ?", token).First(&share).Error; err != nil {
		return nil, fmt.Errorf("share not found: %w", err)
	}

	// Check expiration
	if share.ExpiresAt != nil && share.ExpiresAt.Before(time.Now()) {
		return nil, fmt.Errorf("share has expired")
	}

	return &share, nil
}

// GetUserShares retrieves all shares for a user's objects
func (s *ShareService) GetUserShares(ctx context.Context, userID string) ([]StorageShare, error) {
	var shares []StorageShare
	if err := s.db.Where("created_by = ?", userID).Find(&shares).Error; err != nil {
		return nil, fmt.Errorf("failed to get user shares: %w", err)
	}
	return shares, nil
}

// RevokeShare revokes a share
func (s *ShareService) RevokeShare(ctx context.Context, shareID, userID string) error {
	result := s.db.Where("id = ? AND created_by = ?", shareID, userID).Delete(&StorageShare{})
	if result.Error != nil {
		return fmt.Errorf("failed to revoke share: %w", result.Error)
	}
	if result.RowsAffected == 0 {
		return fmt.Errorf("share not found or unauthorized")
	}
	return nil
}

// ShareOptions defines options for creating a share
type ShareOptions struct {
	SharedWithUserID  string
	SharedWithEmail   string
	PermissionLevel   PermissionLevel
	InheritToChildren bool
	GenerateToken     bool
	IsPublic          bool
	ExpiresAt         *time.Time
}

// QuotaService manages storage quotas and bandwidth limits
type QuotaService struct {
	db     *gorm.DB
	config *CloudStorageConfig
}

// NewQuotaService creates a new quota service
func NewQuotaService(db *gorm.DB, config *CloudStorageConfig) *QuotaService {
	if config == nil {
		config = &CloudStorageConfig{
			DefaultStorageLimit:   10737418240, // 10GB
			DefaultBandwidthLimit: 53687091200, // 50GB
		}
	}
	return &QuotaService{
		db:     db,
		config: config,
	}
}

// GetOrCreateQuota gets or creates a quota for a user
func (q *QuotaService) GetOrCreateQuota(ctx context.Context, userID string) (*StorageQuota, error) {
	var quota StorageQuota
	err := q.db.Where("user_id = ?", userID).First(&quota).Error

	if err == gorm.ErrRecordNotFound {
		// Create default quota
		quota = StorageQuota{
			ID:                uuid.New().String(),
			UserID:            userID,
			MaxStorageBytes:   q.config.DefaultStorageLimit,
			MaxBandwidthBytes: q.config.DefaultBandwidthLimit,
			StorageUsed:       0,
			BandwidthUsed:     0,
		}

		if err := q.db.Create(&quota).Error; err != nil {
			return nil, fmt.Errorf("failed to create quota: %w", err)
		}
	} else if err != nil {
		return nil, fmt.Errorf("failed to get quota: %w", err)
	}

	// Check if bandwidth should be reset (monthly reset)
	if quota.ResetBandwidthAt != nil && quota.ResetBandwidthAt.Before(time.Now()) {
		quota.BandwidthUsed = 0
		nextReset := time.Now().AddDate(0, 1, 0) // Next month
		quota.ResetBandwidthAt = &nextReset
		q.db.Save(&quota)
	}

	return &quota, nil
}

// CheckStorageQuota checks if a user has enough storage quota
func (q *QuotaService) CheckStorageQuota(ctx context.Context, userID string, size int64) error {
	quota, err := q.GetOrCreateQuota(ctx, userID)
	if err != nil {
		return err
	}

	if quota.StorageUsed+size > quota.MaxStorageBytes {
		available := quota.MaxStorageBytes - quota.StorageUsed
		return fmt.Errorf("storage quota exceeded: %s available", formatBytes(available))
	}

	return nil
}

// CheckBandwidthQuota checks if a user has enough bandwidth quota
func (q *QuotaService) CheckBandwidthQuota(ctx context.Context, userID string, size int64) error {
	quota, err := q.GetOrCreateQuota(ctx, userID)
	if err != nil {
		return err
	}

	if quota.BandwidthUsed+size > quota.MaxBandwidthBytes {
		available := quota.MaxBandwidthBytes - quota.BandwidthUsed
		return fmt.Errorf("bandwidth quota exceeded: %s available", formatBytes(available))
	}

	return nil
}

// UpdateStorageUsage updates the storage usage for a user
func (q *QuotaService) UpdateStorageUsage(ctx context.Context, userID string, sizeDelta int64) error {
	return q.db.Model(&StorageQuota{}).
		Where("user_id = ?", userID).
		Updates(map[string]interface{}{
			"storage_used": gorm.Expr("storage_used + ?", sizeDelta),
			"updated_at":   time.Now(),
		}).Error
}

// UpdateBandwidthUsage updates the bandwidth usage for a user
func (q *QuotaService) UpdateBandwidthUsage(ctx context.Context, userID string, sizeDelta int64) error {
	return q.db.Model(&StorageQuota{}).
		Where("user_id = ?", userID).
		Updates(map[string]interface{}{
			"bandwidth_used": gorm.Expr("bandwidth_used + ?", sizeDelta),
			"updated_at":     time.Now(),
		}).Error
}

// GetQuotaStats retrieves quota statistics for a user
func (q *QuotaService) GetQuotaStats(ctx context.Context, userID string) (*QuotaStats, error) {
	quota, err := q.GetOrCreateQuota(ctx, userID)
	if err != nil {
		return nil, err
	}

	return &QuotaStats{
		StorageUsed:         quota.StorageUsed,
		StorageLimit:        quota.MaxStorageBytes,
		StoragePercentage:   float64(quota.StorageUsed) / float64(quota.MaxStorageBytes) * 100,
		BandwidthUsed:       quota.BandwidthUsed,
		BandwidthLimit:      quota.MaxBandwidthBytes,
		BandwidthPercentage: float64(quota.BandwidthUsed) / float64(quota.MaxBandwidthBytes) * 100,
		ResetDate:           quota.ResetBandwidthAt,
	}, nil
}

// QuotaStats represents quota usage statistics
type QuotaStats struct {
	StorageUsed         int64      `json:"storage_used"`
	StorageLimit        int64      `json:"storage_limit"`
	StoragePercentage   float64    `json:"storage_percentage"`
	BandwidthUsed       int64      `json:"bandwidth_used"`
	BandwidthLimit      int64      `json:"bandwidth_limit"`
	BandwidthPercentage float64    `json:"bandwidth_percentage"`
	ResetDate           *time.Time `json:"reset_date,omitempty"`
}

// AccessLogService manages access logging for storage operations
type AccessLogService struct {
	db *gorm.DB
}

// NewAccessLogService creates a new access log service
func NewAccessLogService(db *gorm.DB) *AccessLogService {
	return &AccessLogService{db: db}
}

// LogAccess logs an access event for a storage object
func (a *AccessLogService) LogAccess(ctx context.Context, objectID string, action StorageAction, opts LogOptions) error {
	metadata := make(map[string]interface{})
	if opts.ShareID != "" {
		metadata["share_id"] = opts.ShareID
	}
	if opts.Success != nil {
		metadata["success"] = *opts.Success
	}
	if opts.ErrorMsg != "" {
		metadata["error_msg"] = opts.ErrorMsg
	}
	if opts.BytesSize > 0 {
		metadata["bytes_size"] = opts.BytesSize
	}
	if opts.Duration > 0 {
		metadata["duration_ms"] = opts.Duration.Milliseconds()
	}

	metadataJSON, _ := json.Marshal(metadata)

	log := &StorageAccessLog{
		ID:        uuid.New().String(),
		ObjectID:  objectID,
		Action:    action,
		Metadata:  datatypes.JSON(metadataJSON),
		CreatedAt: time.Now(),
	}

	if opts.UserID != "" {
		log.UserID = &opts.UserID
	}
	if opts.IPAddress != "" {
		log.IPAddress = &opts.IPAddress
	}
	if opts.UserAgent != "" {
		log.UserAgent = &opts.UserAgent
	}

	return a.db.Create(log).Error
}

// GetAccessLogs retrieves access logs with filters
func (a *AccessLogService) GetAccessLogs(ctx context.Context, filters AccessLogFilters) ([]StorageAccessLog, error) {
	query := a.db.Model(&StorageAccessLog{})

	if filters.ObjectID != "" {
		query = query.Where("object_id = ?", filters.ObjectID)
	}
	if filters.UserID != "" {
		query = query.Where("user_id = ?", filters.UserID)
	}
	if filters.Action != "" {
		query = query.Where("action = ?", filters.Action)
	}
	if filters.StartDate != nil {
		query = query.Where("created_at >= ?", filters.StartDate)
	}
	if filters.EndDate != nil {
		query = query.Where("created_at <= ?", filters.EndDate)
	}

	if filters.Limit <= 0 {
		filters.Limit = 100
	}
	query = query.Order("created_at DESC").Limit(filters.Limit)

	var logs []StorageAccessLog
	if err := query.Find(&logs).Error; err != nil {
		return nil, fmt.Errorf("failed to get access logs: %w", err)
	}

	return logs, nil
}

// GetAccessStats retrieves access statistics for an object or user
func (a *AccessLogService) GetAccessStats(ctx context.Context, filters StatsFilters) (*AccessStats, error) {
	query := a.db.Model(&StorageAccessLog{})

	if filters.ObjectID != "" {
		query = query.Where("object_id = ?", filters.ObjectID)
	}
	if filters.UserID != "" {
		query = query.Where("user_id = ?", filters.UserID)
	}
	if filters.StartDate != nil {
		query = query.Where("created_at >= ?", filters.StartDate)
	}
	if filters.EndDate != nil {
		query = query.Where("created_at <= ?", filters.EndDate)
	}

	var stats AccessStats

	// Get total access count
	query.Count(&stats.TotalAccess)

	// Get action breakdown
	var actionCounts []struct {
		Action StorageAction
		Count  int64
	}
	query.Select("action, COUNT(*) as count").Group("action").Scan(&actionCounts)

	stats.ActionBreakdown = make(map[string]int64)
	for _, ac := range actionCounts {
		stats.ActionBreakdown[string(ac.Action)] = ac.Count
	}

	// Get unique users count
	query.Select("COUNT(DISTINCT user_id)").Scan(&stats.UniqueUsers)

	return &stats, nil
}

// LogOptions defines options for logging access
type LogOptions struct {
	UserID    string
	ShareID   string
	IPAddress string
	UserAgent string
	Success   *bool
	ErrorMsg  string
	BytesSize int64
	Duration  time.Duration
}

// AccessLogFilters defines filters for access log queries
type AccessLogFilters struct {
	ObjectID  string
	UserID    string
	Action    string
	StartDate *time.Time
	EndDate   *time.Time
	Limit     int
}

// StatsFilters defines filters for statistics queries
type StatsFilters struct {
	ObjectID  string
	UserID    string
	StartDate *time.Time
	EndDate   *time.Time
}

// AccessStats represents access statistics
type AccessStats struct {
	TotalAccess     int64            `json:"total_access"`
	UniqueUsers     int64            `json:"unique_users"`
	ActionBreakdown map[string]int64 `json:"action_breakdown"`
}

// Helper function to format bytes
func formatBytes(bytes int64) string {
	const unit = 1024
	if bytes < unit {
		return fmt.Sprintf("%d B", bytes)
	}
	div, exp := int64(unit), 0
	for n := bytes / unit; n >= unit; n /= unit {
		div *= unit
		exp++
	}
	return fmt.Sprintf("%.1f %cB", float64(bytes)/float64(div), "KMGTPE"[exp])
}

// Helper function to hash password
func hashPassword(password string) (string, error) {
	bytes, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
	return string(bytes), err
}

// Helper function to check password
func checkPassword(password, hash string) bool {
	err := bcrypt.CompareHashAndPassword([]byte(hash), []byte(password))
	return err == nil
}

// Helper function to parse IP address
func parseIPAddress(addr string) string {
	host, _, err := net.SplitHostPort(addr)
	if err != nil {
		return addr
	}
	return host
}
