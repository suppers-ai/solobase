package cloudstorage

import (
	"context"
	"crypto/rand"
	"database/sql"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"net"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
)

var (
	ErrShareNotFound = errors.New("share not found")
	ErrShareExpired  = errors.New("share has expired")
	ErrObjectNotFound = errors.New("object not found")
)

// ShareService manages file sharing functionality
type ShareService struct {
	sqlDB   *sql.DB
	queries *db.Queries
	manager interface{} // Storage manager interface, can be nil
}

// NewShareService creates a new share service
func NewShareService(sqlDB *sql.DB, manager interface{}) *ShareService {
	return &ShareService{
		sqlDB:   sqlDB,
		queries: db.New(sqlDB),
		manager: manager,
	}
}

// CreateShare creates a shareable link for a storage object
func (s *ShareService) CreateShare(ctx context.Context, objectID, userID string, opts ShareOptions) (*StorageShare, error) {
	// Verify object exists using raw SQL
	var objID string
	err := s.sqlDB.QueryRowContext(ctx, "SELECT id FROM storage_objects WHERE id = ? LIMIT 1", objectID).Scan(&objID)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, ErrObjectNotFound
		}
		return nil, fmt.Errorf("failed to verify object: %w", err)
	}

	share := &StorageShare{
		ID:                uuid.New().String(),
		ObjectID:          objectID,
		CreatedBy:         userID,
		PermissionLevel:   opts.PermissionLevel,
		InheritToChildren: opts.InheritToChildren,
		IsPublic:          opts.IsPublic,
		ExpiresAt:         apptime.FromTimePtr(opts.ExpiresAt),
		CreatedAt:         apptime.NowTime(),
		UpdatedAt:         apptime.NowTime(),
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

	// Convert bool to int for SQLite
	inheritInt := int64(0)
	if share.InheritToChildren {
		inheritInt = 1
	}
	publicInt := int64(0)
	if share.IsPublic {
		publicInt = 1
	}

	_, err = s.queries.CreateStorageShare(ctx, db.CreateStorageShareParams{
		ID:               share.ID,
		ObjectID:         share.ObjectID,
		SharedWithUserID: share.SharedWithUserID,
		SharedWithEmail:  share.SharedWithEmail,
		PermissionLevel:  string(share.PermissionLevel),
		InheritToChildren: inheritInt,
		ShareToken:       share.ShareToken,
		IsPublic:         publicInt,
		ExpiresAt:        share.ExpiresAt,
		CreatedBy:        share.CreatedBy,
		CreatedAt:        apptime.Format(share.CreatedAt),
		UpdatedAt:        apptime.Format(share.UpdatedAt),
	})
	if err != nil {
		return nil, fmt.Errorf("failed to create share: %w", err)
	}

	return share, nil
}

// GetShareByToken retrieves a share by its token
func (s *ShareService) GetShareByToken(ctx context.Context, token string) (*StorageShare, error) {
	dbShare, err := s.queries.GetStorageShareByToken(ctx, &token)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, ErrShareNotFound
		}
		return nil, fmt.Errorf("share not found: %w", err)
	}

	share := dbShareToStorageShare(dbShare)

	// Check expiration
	if share.ExpiresAt.Valid && share.ExpiresAt.Time.Before(apptime.NowTime()) {
		return nil, ErrShareExpired
	}

	return share, nil
}

// GetUserShares retrieves all shares for a user's objects
func (s *ShareService) GetUserShares(ctx context.Context, userID string) ([]StorageShare, error) {
	// Query shares created by the user
	rows, err := s.sqlDB.QueryContext(ctx, `
		SELECT id, object_id, shared_with_user_id, shared_with_email, permission_level,
		       inherit_to_children, share_token, is_public, expires_at, created_by, created_at, updated_at
		FROM ext_cloudstorage_storage_shares
		WHERE created_by = ?
		ORDER BY created_at DESC
	`, userID)
	if err != nil {
		return nil, fmt.Errorf("failed to get user shares: %w", err)
	}
	defer rows.Close()

	var shares []StorageShare
	for rows.Next() {
		var share StorageShare
		var permLevel string
		var inheritInt, publicInt int64
		if err := rows.Scan(
			&share.ID, &share.ObjectID, &share.SharedWithUserID, &share.SharedWithEmail,
			&permLevel, &inheritInt, &share.ShareToken, &publicInt, &share.ExpiresAt,
			&share.CreatedBy, &share.CreatedAt, &share.UpdatedAt,
		); err != nil {
			return nil, err
		}
		share.PermissionLevel = PermissionLevel(permLevel)
		share.InheritToChildren = inheritInt == 1
		share.IsPublic = publicInt == 1
		shares = append(shares, share)
	}

	return shares, nil
}

// GetSharedWithMe retrieves all items shared with the current user (both direct and inherited)
func (s *ShareService) GetSharedWithMe(ctx context.Context, userID string) ([]StorageShareWithObject, error) {
	// Get user's email for email-based shares
	var userEmail string
	s.sqlDB.QueryRowContext(ctx, "SELECT email FROM users WHERE id = ?", userID).Scan(&userEmail)

	// Use a CTE to find both direct shares and inherited shares
	now := apptime.NowTime()
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
				0 as is_inherited,
				so.object_name,
				so.content_type,
				so.size,
				so.created_at as object_created_at,
				so.metadata as object_metadata
			FROM ext_cloudstorage_storage_shares ss
			JOIN storage_objects so ON ss.object_id = so.id
			WHERE (ss.shared_with_user_id = ? OR ss.shared_with_email = ?)
				AND (ss.expires_at IS NULL OR ss.expires_at > ?)

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
				1 as is_inherited,
				child.object_name,
				child.content_type,
				child.size,
				child.created_at as object_created_at,
				child.metadata as object_metadata
			FROM ext_cloudstorage_storage_shares ss
			JOIN storage_objects parent ON ss.object_id = parent.id
			JOIN storage_objects child ON child.parent_folder_id = parent.id
			WHERE ss.inherit_to_children = 1
				AND (ss.shared_with_user_id = ? OR ss.shared_with_email = ?)
				AND (ss.expires_at IS NULL OR ss.expires_at > ?)
		)
		SELECT * FROM shared_items
		ORDER BY object_name
	`

	rows, err := s.sqlDB.QueryContext(ctx, query, userID, userEmail, now, userID, userEmail, now)
	if err != nil {
		// Fallback to simple query if CTE fails
		return s.getSharedWithMeSimple(ctx, userID, userEmail)
	}
	defer rows.Close()

	var shares []StorageShareWithObject
	for rows.Next() {
		var share StorageShareWithObject
		var permLevel string
		var inheritInt, publicInt, isInheritedInt int64
		if err := rows.Scan(
			&share.ID, &share.ObjectID, &share.SharedWithUserID, &share.SharedWithEmail,
			&permLevel, &inheritInt, &share.ShareToken, &publicInt, &share.ExpiresAt,
			&share.CreatedBy, &share.CreatedAt, &share.UpdatedAt, &isInheritedInt,
			&share.ObjectName, &share.ContentType, &share.Size, &share.ObjectCreatedAt, &share.ObjectMetadata,
		); err != nil {
			return nil, err
		}
		share.PermissionLevel = PermissionLevel(permLevel)
		share.InheritToChildren = inheritInt == 1
		share.IsPublic = publicInt == 1
		shares = append(shares, share)
	}

	return shares, nil
}

// getSharedWithMeSimple is the fallback for databases without CTE support
func (s *ShareService) getSharedWithMeSimple(ctx context.Context, userID string, userEmail string) ([]StorageShareWithObject, error) {
	query := `
		SELECT ss.id, ss.object_id, ss.shared_with_user_id, ss.shared_with_email,
		       ss.permission_level, ss.inherit_to_children, ss.share_token, ss.is_public,
		       ss.expires_at, ss.created_by, ss.created_at, ss.updated_at,
		       so.object_name, so.content_type, so.size, so.created_at, so.metadata
		FROM ext_cloudstorage_storage_shares ss
		JOIN storage_objects so ON ss.object_id = so.id
		WHERE (ss.shared_with_user_id = ? OR ss.shared_with_email = ?)
		  AND (ss.expires_at IS NULL OR ss.expires_at > ?)
	`

	rows, err := s.sqlDB.QueryContext(ctx, query, userID, userEmail, apptime.NowTime())
	if err != nil {
		return nil, fmt.Errorf("failed to get shared items: %w", err)
	}
	defer rows.Close()

	var shares []StorageShareWithObject
	for rows.Next() {
		var share StorageShareWithObject
		var permLevel string
		var inheritInt, publicInt int64
		if err := rows.Scan(
			&share.ID, &share.ObjectID, &share.SharedWithUserID, &share.SharedWithEmail,
			&permLevel, &inheritInt, &share.ShareToken, &publicInt, &share.ExpiresAt,
			&share.CreatedBy, &share.CreatedAt, &share.UpdatedAt,
			&share.ObjectName, &share.ContentType, &share.Size, &share.ObjectCreatedAt, &share.ObjectMetadata,
		); err != nil {
			return nil, err
		}
		share.PermissionLevel = PermissionLevel(permLevel)
		share.InheritToChildren = inheritInt == 1
		share.IsPublic = publicInt == 1
		shares = append(shares, share)
	}

	return shares, nil
}

// GetSharedByMe retrieves all items shared by the current user
func (s *ShareService) GetSharedByMe(ctx context.Context, userID string) ([]StorageShareWithObject, error) {
	query := `
		SELECT ss.id, ss.object_id, ss.shared_with_user_id, ss.shared_with_email,
		       ss.permission_level, ss.inherit_to_children, ss.share_token, ss.is_public,
		       ss.expires_at, ss.created_by, ss.created_at, ss.updated_at,
		       so.object_name, so.content_type, so.size, so.created_at, so.metadata
		FROM ext_cloudstorage_storage_shares ss
		JOIN storage_objects so ON ss.object_id = so.id
		WHERE ss.created_by = ?
	`

	rows, err := s.sqlDB.QueryContext(ctx, query, userID)
	if err != nil {
		return nil, fmt.Errorf("failed to get shared items: %w", err)
	}
	defer rows.Close()

	var shares []StorageShareWithObject
	for rows.Next() {
		var share StorageShareWithObject
		var permLevel string
		var inheritInt, publicInt int64
		if err := rows.Scan(
			&share.ID, &share.ObjectID, &share.SharedWithUserID, &share.SharedWithEmail,
			&permLevel, &inheritInt, &share.ShareToken, &publicInt, &share.ExpiresAt,
			&share.CreatedBy, &share.CreatedAt, &share.UpdatedAt,
			&share.ObjectName, &share.ContentType, &share.Size, &share.ObjectCreatedAt, &share.ObjectMetadata,
		); err != nil {
			return nil, err
		}
		share.PermissionLevel = PermissionLevel(permLevel)
		share.InheritToChildren = inheritInt == 1
		share.IsPublic = publicInt == 1
		shares = append(shares, share)
	}

	return shares, nil
}

// RevokeShare revokes a share
func (s *ShareService) RevokeShare(ctx context.Context, shareID, userID string) error {
	result, err := s.sqlDB.ExecContext(ctx,
		"DELETE FROM ext_cloudstorage_storage_shares WHERE id = ? AND created_by = ?",
		shareID, userID)
	if err != nil {
		return fmt.Errorf("failed to revoke share: %w", err)
	}
	rowsAffected, _ := result.RowsAffected()
	if rowsAffected == 0 {
		return fmt.Errorf("share not found or unauthorized")
	}
	return nil
}

// CheckInheritedPermissions checks if a user has access to an object through parent folder shares
func (s *ShareService) CheckInheritedPermissions(ctx context.Context, objectID string, userID string, userEmail string) (*StorageShare, error) {
	// Build the recursive CTE query to find all parent folders and their shares
	now := apptime.NowTime()
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
			WHERE ph.depth < 20
		)
		SELECT
			ss.id, ss.object_id, ss.shared_with_user_id, ss.shared_with_email,
			ss.permission_level, ss.inherit_to_children, ss.share_token, ss.is_public,
			ss.expires_at, ss.created_by, ss.created_at, ss.updated_at,
			ph.depth
		FROM parent_hierarchy ph
		INNER JOIN ext_cloudstorage_storage_shares ss ON ss.object_id = ph.id
		WHERE ss.inherit_to_children = 1
			AND (ss.expires_at IS NULL OR ss.expires_at > ?)
	`

	// Build conditions based on user info
	var args []interface{}
	args = append(args, objectID, now)

	if userID != "" {
		query += " AND (ss.is_public = 1 OR ss.shared_with_user_id = ?)"
		args = append(args, userID)
	}
	if userEmail != "" {
		query += " OR ss.shared_with_email = ?"
		args = append(args, userEmail)
	}
	if userID == "" && userEmail == "" {
		query += " AND ss.is_public = 1"
	}

	query += ` ORDER BY ph.depth ASC,
		CASE ss.permission_level
		WHEN 'admin' THEN 3
		WHEN 'edit' THEN 2
		WHEN 'view' THEN 1
		ELSE 0 END DESC`

	rows, err := s.sqlDB.QueryContext(ctx, query, args...)
	if err != nil {
		// Fallback to iterative approach
		return s.checkInheritedPermissionsIterative(ctx, objectID, userID, userEmail)
	}
	defer rows.Close()

	if rows.Next() {
		var share StorageShare
		var permLevel string
		var inheritInt, publicInt int64
		var depth int
		if err := rows.Scan(
			&share.ID, &share.ObjectID, &share.SharedWithUserID, &share.SharedWithEmail,
			&permLevel, &inheritInt, &share.ShareToken, &publicInt, &share.ExpiresAt,
			&share.CreatedBy, &share.CreatedAt, &share.UpdatedAt, &depth,
		); err != nil {
			return nil, err
		}
		share.PermissionLevel = PermissionLevel(permLevel)
		share.InheritToChildren = inheritInt == 1
		share.IsPublic = publicInt == 1
		return &share, nil
	}

	return nil, nil
}

// checkInheritedPermissionsIterative is the fallback implementation
func (s *ShareService) checkInheritedPermissionsIterative(ctx context.Context, objectID string, userID string, userEmail string) (*StorageShare, error) {
	// Get the object to find its parent
	var parentFolderID *string
	err := s.sqlDB.QueryRowContext(ctx,
		"SELECT parent_folder_id FROM storage_objects WHERE id = ?", objectID).Scan(&parentFolderID)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, ErrObjectNotFound
		}
		return nil, err
	}

	// If object has no parent, no inheritance to check
	if parentFolderID == nil || *parentFolderID == "" {
		return nil, nil
	}

	// Recursively check parent folders for shares
	currentParentID := parentFolderID
	maxDepth := 20
	depth := 0

	var bestShare *StorageShare
	var bestPermissionLevel PermissionLevel = PermissionView

	for currentParentID != nil && *currentParentID != "" && depth < maxDepth {
		// Build query for shares on this parent folder
		query := `
			SELECT id, object_id, shared_with_user_id, shared_with_email, permission_level,
			       inherit_to_children, share_token, is_public, expires_at, created_by, created_at, updated_at
			FROM ext_cloudstorage_storage_shares
			WHERE object_id = ? AND inherit_to_children = 1
		`
		args := []interface{}{*currentParentID}

		if userID != "" || userEmail != "" {
			query += " AND (is_public = 1"
			if userID != "" {
				query += " OR shared_with_user_id = ?"
				args = append(args, userID)
			}
			if userEmail != "" {
				query += " OR shared_with_email = ?"
				args = append(args, userEmail)
			}
			query += ")"
		} else {
			query += " AND is_public = 1"
		}

		rows, err := s.sqlDB.QueryContext(ctx, query, args...)
		if err != nil {
			return nil, fmt.Errorf("failed to check parent shares: %w", err)
		}

		for rows.Next() {
			var share StorageShare
			var permLevel string
			var inheritInt, publicInt int64
			if err := rows.Scan(
				&share.ID, &share.ObjectID, &share.SharedWithUserID, &share.SharedWithEmail,
				&permLevel, &inheritInt, &share.ShareToken, &publicInt, &share.ExpiresAt,
				&share.CreatedBy, &share.CreatedAt, &share.UpdatedAt,
			); err != nil {
				rows.Close()
				return nil, err
			}
			share.PermissionLevel = PermissionLevel(permLevel)
			share.InheritToChildren = inheritInt == 1
			share.IsPublic = publicInt == 1

			// Check if share is expired
			if share.ExpiresAt.Valid && share.ExpiresAt.Time.Before(apptime.NowTime()) {
				continue
			}

			// Compare permission levels
			if comparePermissionLevel(share.PermissionLevel, bestPermissionLevel) > 0 {
				shareCopy := share
				bestShare = &shareCopy
				bestPermissionLevel = share.PermissionLevel

				if bestPermissionLevel == PermissionAdmin {
					rows.Close()
					return bestShare, nil
				}
			}
		}
		rows.Close()

		// Get the parent folder to continue traversing up
		var nextParentID *string
		err = s.sqlDB.QueryRowContext(ctx,
			"SELECT parent_folder_id FROM storage_objects WHERE id = ?", *currentParentID).Scan(&nextParentID)
		if err != nil {
			break
		}

		currentParentID = nextParentID
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
	ExpiresAt         *apptime.Time
}

// AccessLogService manages access logging for storage operations
type AccessLogService struct {
	sqlDB   *sql.DB
	queries *db.Queries
}

// NewAccessLogService creates a new access log service
func NewAccessLogService(sqlDB *sql.DB) *AccessLogService {
	return &AccessLogService{
		sqlDB:   sqlDB,
		queries: db.New(sqlDB),
	}
}

// LogAccess logs an access event for a storage object
func (a *AccessLogService) LogAccess(ctx context.Context, objectID string, action StorageAction, opts LogOptions) error {
	metadata := make(map[string]interface{})
	if opts.ShareID != "" {
		metadata["shareId"] = opts.ShareID
	}
	if opts.Success != nil {
		metadata["success"] = *opts.Success
	}
	if opts.ErrorMsg != "" {
		metadata["errorMsg"] = opts.ErrorMsg
	}
	if opts.BytesSize > 0 {
		metadata["bytesSize"] = opts.BytesSize
	}
	if opts.Duration > 0 {
		metadata["durationMs"] = opts.Duration.Milliseconds()
	}

	metadataJSON, _ := json.Marshal(metadata)

	id := uuid.New().String()
	now := apptime.NowTime()

	var userIDPtr, ipAddrPtr, userAgentPtr *string
	if opts.UserID != "" {
		userIDPtr = &opts.UserID
	}
	if opts.IPAddress != "" {
		ipAddrPtr = &opts.IPAddress
	}
	if opts.UserAgent != "" {
		userAgentPtr = &opts.UserAgent
	}

	_, err := a.queries.CreateStorageAccessLog(ctx, db.CreateStorageAccessLogParams{
		ID:        id,
		ObjectID:  objectID,
		UserID:    userIDPtr,
		IpAddress: ipAddrPtr,
		Action:    string(action),
		UserAgent: userAgentPtr,
		Metadata:  metadataJSON,
		CreatedAt: apptime.Format(now),
	})

	return err
}

// GetAccessLogs retrieves access logs with filters
func (a *AccessLogService) GetAccessLogs(ctx context.Context, filters AccessLogFilters) ([]StorageAccessLog, error) {
	query := `
		SELECT id, object_id, user_id, ip_address, action, user_agent, metadata, created_at
		FROM ext_cloudstorage_storage_access_logs
		WHERE 1=1
	`
	args := []interface{}{}

	if filters.ObjectID != "" {
		query += " AND object_id = ?"
		args = append(args, filters.ObjectID)
	}
	if filters.UserID != "" {
		query += " AND user_id = ?"
		args = append(args, filters.UserID)
	}
	if filters.Action != "" {
		query += " AND action = ?"
		args = append(args, filters.Action)
	}
	if filters.StartDate != nil {
		query += " AND created_at >= ?"
		args = append(args, filters.StartDate)
	}
	if filters.EndDate != nil {
		query += " AND created_at <= ?"
		args = append(args, filters.EndDate)
	}

	limit := filters.Limit
	if limit <= 0 {
		limit = 100
	}
	query += " ORDER BY created_at DESC LIMIT ?"
	args = append(args, limit)

	rows, err := a.sqlDB.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, fmt.Errorf("failed to get access logs: %w", err)
	}
	defer rows.Close()

	var logs []StorageAccessLog
	for rows.Next() {
		var log StorageAccessLog
		var actionStr string
		if err := rows.Scan(
			&log.ID, &log.ObjectID, &log.UserID, &log.IPAddress,
			&actionStr, &log.UserAgent, &log.Metadata, &log.CreatedAt,
		); err != nil {
			return nil, err
		}
		log.Action = StorageAction(actionStr)
		logs = append(logs, log)
	}

	return logs, nil
}

// GetAccessStats retrieves access statistics for an object or user
func (a *AccessLogService) GetAccessStats(ctx context.Context, filters StatsFilters) (*AccessStats, error) {
	baseQuery := `FROM ext_cloudstorage_storage_access_logs WHERE 1=1`
	args := []interface{}{}

	if filters.ObjectID != "" {
		baseQuery += " AND object_id = ?"
		args = append(args, filters.ObjectID)
	}
	if filters.UserID != "" {
		baseQuery += " AND user_id = ?"
		args = append(args, filters.UserID)
	}
	if filters.StartDate != nil {
		baseQuery += " AND created_at >= ?"
		args = append(args, filters.StartDate)
	}
	if filters.EndDate != nil {
		baseQuery += " AND created_at <= ?"
		args = append(args, filters.EndDate)
	}

	var stats AccessStats

	// Get total access count
	var totalAccess int64
	err := a.sqlDB.QueryRowContext(ctx, "SELECT COUNT(*) "+baseQuery, args...).Scan(&totalAccess)
	if err != nil {
		return nil, err
	}
	stats.TotalAccess = totalAccess

	// Get action breakdown
	rows, err := a.sqlDB.QueryContext(ctx, "SELECT action, COUNT(*) as count "+baseQuery+" GROUP BY action", args...)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	stats.ActionBreakdown = make(map[string]int64)
	for rows.Next() {
		var action string
		var count int64
		if err := rows.Scan(&action, &count); err != nil {
			return nil, err
		}
		stats.ActionBreakdown[action] = count
	}

	// Get unique users count
	err = a.sqlDB.QueryRowContext(ctx, "SELECT COUNT(DISTINCT user_id) "+baseQuery, args...).Scan(&stats.UniqueUsers)
	if err != nil {
		return nil, err
	}

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
	Duration  apptime.Duration
}

// AccessLogFilters defines filters for access log queries
type AccessLogFilters struct {
	ObjectID  string
	UserID    string
	Action    string
	StartDate *apptime.Time
	EndDate   *apptime.Time
	Limit     int
}

// StatsFilters defines filters for statistics queries
type StatsFilters struct {
	ObjectID  string
	UserID    string
	StartDate *apptime.Time
	EndDate   *apptime.Time
}

// AccessStats represents access statistics
type AccessStats struct {
	TotalAccess     int64            `json:"totalAccess"`
	UniqueUsers     int64            `json:"uniqueUsers"`
	ActionBreakdown map[string]int64 `json:"actionBreakdown"`
}

// Helper function to parse IP address
func parseIPAddress(addr string) string {
	host, _, err := net.SplitHostPort(addr)
	if err != nil {
		return addr
	}
	return host
}

// dbShareToStorageShare converts a sqlc generated share to our model
func dbShareToStorageShare(dbShare db.ExtCloudstorageStorageShare) *StorageShare {
	share := &StorageShare{
		ID:                dbShare.ID,
		ObjectID:          dbShare.ObjectID,
		SharedWithUserID:  dbShare.SharedWithUserID,
		SharedWithEmail:   dbShare.SharedWithEmail,
		PermissionLevel:   PermissionLevel(dbShare.PermissionLevel),
		InheritToChildren: dbShare.InheritToChildren == 1,
		ShareToken:        dbShare.ShareToken,
		IsPublic:          dbShare.IsPublic == 1,
		ExpiresAt:         apptime.FromTimePtr(dbShare.ExpiresAt.ToTimePtr()),
		CreatedBy:         dbShare.CreatedBy,
		CreatedAt:         apptime.NewTime(apptime.MustParse(dbShare.CreatedAt)),
		UpdatedAt:         apptime.NewTime(apptime.MustParse(dbShare.UpdatedAt)),
	}
	return share
}
