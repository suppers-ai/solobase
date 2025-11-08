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
	pkgstorage "github.com/suppers-ai/solobase/internal/pkg/storage"
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

// GetSharedWithMe retrieves all items shared with the current user (both direct and inherited)
func (s *ShareService) GetSharedWithMe(ctx context.Context, userID string) ([]StorageShareWithObject, error) {
	// Get user's email for email-based shares
	var userEmail string
	s.db.Table("users").Where("id = ?", userID).Select("email").Scan(&userEmail)

	// Use a CTE to find both direct shares and inherited shares
	query := `
		WITH RECURSIVE shared_items AS (
			-- Direct shares: items directly shared with the user
			SELECT DISTINCT
				ss.id,
				ss.object_id,
				ss.shared_with_user_id,
				ss.shared_with_email,
				ss.permission_level,
				ss.inherit_to_children,
				ss.share_token,
				ss.is_public,
				ss.expires_at,
				ss.created_by,
				ss.created_at,
				ss.updated_at,
				false as is_inherited,
				so.object_name,
				so.content_type,
				so.size,
				so.created_at as object_created_at,
				so.metadata as object_metadata
			FROM ext_cloudstorage_storage_shares ss
			JOIN storage_objects so ON ss.object_id = so.id
			WHERE (ss.shared_with_user_id = ? OR ss.shared_with_email = ?)
				AND (ss.expires_at IS NULL OR ss.expires_at > datetime('now'))

			UNION

			-- Inherited shares: child items of folders shared with inherit_to_children = true
			SELECT DISTINCT
				ss.id,
				child.id as object_id,
				ss.shared_with_user_id,
				ss.shared_with_email,
				ss.permission_level,
				ss.inherit_to_children,
				ss.share_token,
				ss.is_public,
				ss.expires_at,
				ss.created_by,
				ss.created_at,
				ss.updated_at,
				true as is_inherited,
				child.object_name,
				child.content_type,
				child.size,
				child.created_at as object_created_at,
				child.metadata as object_metadata
			FROM ext_cloudstorage_storage_shares ss
			JOIN storage_objects parent ON ss.object_id = parent.id
			JOIN storage_objects child ON child.parent_folder_id = parent.id
			WHERE ss.inherit_to_children = true
				AND (ss.shared_with_user_id = ? OR ss.shared_with_email = ?)
				AND (ss.expires_at IS NULL OR ss.expires_at > datetime('now'))
		)
		SELECT * FROM shared_items
		ORDER BY object_name
	`

	var shares []StorageShareWithObject
	if err := s.db.Raw(query, userID, userEmail, userID, userEmail).Scan(&shares).Error; err != nil {
		// Fallback to simple query if CTE fails
		return s.getSharedWithMeSimple(ctx, userID, userEmail)
	}

	return shares, nil
}

// getSharedWithMeSimple is the fallback for databases without CTE support
func (s *ShareService) getSharedWithMeSimple(ctx context.Context, userID string, userEmail string) ([]StorageShareWithObject, error) {
	var shares []StorageShareWithObject

	// Query direct shares with joined storage objects
	query := s.db.Table("ext_cloudstorage_storage_shares ss").
		Select("ss.*, so.id as object_id, so.object_name, so.content_type, so.size, so.created_at as object_created_at, so.metadata as object_metadata").
		Joins("JOIN storage_objects so ON ss.object_id = so.id").
		Where("(ss.shared_with_user_id = ? OR ss.shared_with_email = ?) AND (ss.expires_at IS NULL OR ss.expires_at > ?)",
			userID, userEmail, time.Now())

	if err := query.Scan(&shares).Error; err != nil {
		return nil, fmt.Errorf("failed to get shared items: %w", err)
	}

	return shares, nil
}

// GetSharedByMe retrieves all items shared by the current user
func (s *ShareService) GetSharedByMe(ctx context.Context, userID string) ([]StorageShareWithObject, error) {
	var shares []StorageShareWithObject

	// Query shares with joined storage objects
	query := s.db.Table("ext_cloudstorage_storage_shares ss").
		Select("ss.*, so.id as object_id, so.name as object_name, so.content_type, so.size, so.created_at as object_created_at, so.metadata as object_metadata").
		Joins("JOIN storage_objects so ON ss.object_id = so.id").
		Where("ss.created_by = ?", userID)

	if err := query.Scan(&shares).Error; err != nil {
		return nil, fmt.Errorf("failed to get shared items: %w", err)
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

// CheckInheritedPermissions checks if a user has access to an object through parent folder shares
// This uses a recursive CTE query for better performance
func (s *ShareService) CheckInheritedPermissions(ctx context.Context, objectID string, userID string, userEmail string) (*StorageShare, error) {
	// Build the recursive CTE query to find all parent folders and their shares
	query := `
		WITH RECURSIVE parent_hierarchy AS (
			-- Base case: start with the object itself
			SELECT id, parent_folder_id, 0 as depth
			FROM storage_objects
			WHERE id = ?

			UNION ALL

			-- Recursive case: find parent folders
			SELECT so.id, so.parent_folder_id, ph.depth + 1
			FROM storage_objects so
			INNER JOIN parent_hierarchy ph ON so.id = ph.parent_folder_id
			WHERE ph.depth < 20  -- Prevent infinite recursion
		)
		SELECT
			ss.*,
			ph.depth
		FROM parent_hierarchy ph
		INNER JOIN ext_cloudstorage_storage_shares ss ON ss.object_id = ph.id
		WHERE ss.inherit_to_children = true
			AND (ss.expires_at IS NULL OR ss.expires_at > datetime('now'))
	`

	// Add user-specific conditions
	var args []interface{}
	args = append(args, objectID)

	conditions := []string{}
	if userID != "" || userEmail != "" {
		if userID != "" {
			conditions = append(conditions, "(ss.is_public = true OR ss.shared_with_user_id = ?)")
			args = append(args, userID)
		}
		if userEmail != "" {
			if len(conditions) > 0 {
				conditions = append(conditions, "ss.shared_with_email = ?")
			} else {
				conditions = append(conditions, "(ss.is_public = true OR ss.shared_with_email = ?)")
			}
			args = append(args, userEmail)
		}
	} else {
		conditions = append(conditions, "ss.is_public = true")
	}

	if len(conditions) > 0 {
		query += " AND (" + conditions[0]
		for i := 1; i < len(conditions); i++ {
			query += " OR " + conditions[i]
		}
		query += ")"
	}

	query += " ORDER BY ph.depth ASC, " +
		"CASE ss.permission_level " +
		"WHEN 'admin' THEN 3 " +
		"WHEN 'edit' THEN 2 " +
		"WHEN 'view' THEN 1 " +
		"ELSE 0 END DESC"

	// Execute the query
	var shares []struct {
		StorageShare
		Depth int
	}

	if err := s.db.Raw(query, args...).Scan(&shares).Error; err != nil {
		// Fallback to the iterative approach if CTE is not supported
		return s.checkInheritedPermissionsIterative(ctx, objectID, userID, userEmail)
	}

	// Return the best share (first one due to our ordering)
	if len(shares) > 0 {
		return &shares[0].StorageShare, nil
	}

	return nil, nil
}

// checkInheritedPermissionsIterative is the fallback implementation for databases without CTE support
func (s *ShareService) checkInheritedPermissionsIterative(ctx context.Context, objectID string, userID string, userEmail string) (*StorageShare, error) {
	// First, get the object to find its parent
	var obj pkgstorage.StorageObject
	if err := s.db.Where("id = ?", objectID).First(&obj).Error; err != nil {
		return nil, fmt.Errorf("object not found: %w", err)
	}

	// If object has no parent, no inheritance to check
	if obj.ParentFolderID == nil || *obj.ParentFolderID == "" {
		return nil, nil
	}

	// Recursively check parent folders for shares
	currentParentID := obj.ParentFolderID
	maxDepth := 20 // Prevent infinite loops
	depth := 0

	var bestShare *StorageShare
	var bestPermissionLevel PermissionLevel = PermissionView // Start with lowest permission

	for currentParentID != nil && *currentParentID != "" && depth < maxDepth {
		// Check for shares on this parent folder
		var shares []StorageShare
		query := s.db.Where("object_id = ? AND inherit_to_children = ?", *currentParentID, true)

		// Check for public shares, user-specific shares, or email-based shares
		if userID != "" || userEmail != "" {
			subQuery := query.Where("is_public = ?", true)
			if userID != "" {
				subQuery = subQuery.Or("shared_with_user_id = ?", userID)
			}
			if userEmail != "" {
				subQuery = subQuery.Or("shared_with_email = ?", userEmail)
			}
			query = subQuery
		} else {
			// Only check public shares if no user info provided
			query = query.Where("is_public = ?", true)
		}

		if err := query.Find(&shares).Error; err != nil {
			return nil, fmt.Errorf("failed to check parent shares: %w", err)
		}

		// Find the most permissive share
		for _, share := range shares {
			// Check if share is expired
			if share.ExpiresAt != nil && share.ExpiresAt.Before(time.Now()) {
				continue
			}

			// Compare permission levels (Admin > Edit > View)
			if comparePermissionLevel(share.PermissionLevel, bestPermissionLevel) > 0 {
				shareCopy := share
				bestShare = &shareCopy
				bestPermissionLevel = share.PermissionLevel

				// If we found admin permission, no need to check further
				if bestPermissionLevel == PermissionAdmin {
					return bestShare, nil
				}
			}
		}

		// Get the parent folder to continue traversing up
		var parentFolder pkgstorage.StorageObject
		if err := s.db.Where("id = ?", *currentParentID).First(&parentFolder).Error; err != nil {
			// Parent not found, stop traversing
			break
		}

		currentParentID = parentFolder.ParentFolderID
		depth++
	}

	return bestShare, nil
}

// comparePermissionLevel returns 1 if a > b, -1 if a < b, 0 if equal
func comparePermissionLevel(a, b PermissionLevel) int {
	levels := map[PermissionLevel]int{
		PermissionView:  1,
		PermissionEdit:  2,
		PermissionAdmin: 3,
	}

	aLevel := levels[a]
	bLevel := levels[b]

	if aLevel > bLevel {
		return 1
	} else if aLevel < bLevel {
		return -1
	}
	return 0
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

// NOTE: QuotaService has been moved to quota_service.go with enhanced functionality
// including role-based quotas and user overrides

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
