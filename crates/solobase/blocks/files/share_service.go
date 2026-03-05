package files

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"errors"
	"fmt"

	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/solobase/core/uuid"
	"github.com/wafer-run/wafer-go/services/database"
)

var (
	ErrShareNotFound = errors.New("share not found")
	ErrShareExpired  = errors.New("share has expired")
)

// ShareService manages file sharing functionality
type ShareService struct {
	db      database.Service
	manager interface{} // Storage manager interface, can be nil
}

// NewShareService creates a new share service
func NewShareService(db database.Service, manager interface{}) *ShareService {
	return &ShareService{
		db:      db,
		manager: manager,
	}
}

// CreateShare creates a shareable link for a storage object
func (s *ShareService) CreateShare(ctx context.Context, objectID, userID string, opts ShareOptions) (*StorageShare, error) {
	// Verify object exists
	_, err := s.db.Get(ctx, "storage_objects", objectID)
	if err != nil {
		if errors.Is(err, database.ErrNotFound) {
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

	// Format expires_at
	var expiresAtVal any
	if share.ExpiresAt.Valid {
		expiresAtVal = apptime.Format(share.ExpiresAt.Time)
	}

	_, err = s.db.Create(ctx, "ext_cloudstorage_storage_shares", map[string]any{
		"id":                  share.ID,
		"object_id":           share.ObjectID,
		"shared_with_user_id": share.SharedWithUserID,
		"shared_with_email":   share.SharedWithEmail,
		"permission_level":    string(share.PermissionLevel),
		"inherit_to_children": inheritInt,
		"share_token":         share.ShareToken,
		"is_public":           publicInt,
		"expires_at":          expiresAtVal,
		"created_by":          share.CreatedBy,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to create share: %w", err)
	}

	return share, nil
}

// GetShareByToken retrieves a share by its token
func (s *ShareService) GetShareByToken(ctx context.Context, token string) (*StorageShare, error) {
	rec, err := database.GetByField(ctx, s.db, "ext_cloudstorage_storage_shares", "share_token", token)
	if err != nil {
		if errors.Is(err, database.ErrNotFound) {
			return nil, ErrShareNotFound
		}
		return nil, fmt.Errorf("share not found: %w", err)
	}

	share := recordToStorageShare(rec)

	// Check expiration
	if share.ExpiresAt.Valid && share.ExpiresAt.Time.Before(apptime.NowTime()) {
		return nil, ErrShareExpired
	}

	return &share, nil
}

// GetUserShares retrieves all shares for a user's objects
func (s *ShareService) GetUserShares(ctx context.Context, userID string) ([]StorageShare, error) {
	result, err := s.db.List(ctx, "ext_cloudstorage_storage_shares", &database.ListOptions{
		Filters: []database.Filter{
			{Field: "created_by", Operator: database.OpEqual, Value: userID},
		},
		Sort: []database.SortField{
			{Field: "created_at", Desc: true},
		},
	})
	if err != nil {
		return nil, fmt.Errorf("failed to get user shares: %w", err)
	}

	var shares []StorageShare
	for _, rec := range result.Records {
		shares = append(shares, recordToStorageShare(rec))
	}

	return shares, nil
}

// GetSharedWithMe retrieves all items shared with the current user (both direct and inherited)
func (s *ShareService) GetSharedWithMe(ctx context.Context, userID string) ([]StorageShareWithObject, error) {
	// Get user's email for email-based shares
	var userEmail string
	rec, err := s.db.Get(ctx, "auth_users", userID)
	if err == nil {
		userEmail = stringVal(rec.Data["email"])
	}

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

	records, err := s.db.QueryRaw(ctx, query, userID, userEmail, now, userID, userEmail, now)
	if err != nil {
		// Fallback to simple query if CTE fails
		return s.getSharedWithMeSimple(ctx, userID, userEmail)
	}

	var shares []StorageShareWithObject
	for _, rec := range records {
		share := recordToShareWithObject(rec)
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
		       so.object_name, so.content_type, so.size, so.created_at as object_created_at, so.metadata as object_metadata
		FROM ext_cloudstorage_storage_shares ss
		JOIN storage_objects so ON ss.object_id = so.id
		WHERE (ss.shared_with_user_id = ? OR ss.shared_with_email = ?)
		  AND (ss.expires_at IS NULL OR ss.expires_at > ?)
	`

	records, err := s.db.QueryRaw(ctx, query, userID, userEmail, apptime.NowTime())
	if err != nil {
		return nil, fmt.Errorf("failed to get shared items: %w", err)
	}

	var shares []StorageShareWithObject
	for _, rec := range records {
		share := recordToShareWithObject(rec)
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
		       so.object_name, so.content_type, so.size, so.created_at as object_created_at, so.metadata as object_metadata
		FROM ext_cloudstorage_storage_shares ss
		JOIN storage_objects so ON ss.object_id = so.id
		WHERE ss.created_by = ?
	`

	records, err := s.db.QueryRaw(ctx, query, userID)
	if err != nil {
		return nil, fmt.Errorf("failed to get shared items: %w", err)
	}

	var shares []StorageShareWithObject
	for _, rec := range records {
		share := recordToShareWithObject(rec)
		shares = append(shares, share)
	}

	return shares, nil
}

// RevokeShare revokes a share
func (s *ShareService) RevokeShare(ctx context.Context, shareID, userID string) error {
	// Verify the share exists and belongs to the user
	rec, err := s.db.Get(ctx, "ext_cloudstorage_storage_shares", shareID)
	if err != nil {
		return fmt.Errorf("share not found or unauthorized")
	}
	if stringVal(rec.Data["created_by"]) != userID {
		return fmt.Errorf("share not found or unauthorized")
	}
	return s.db.Delete(ctx, "ext_cloudstorage_storage_shares", shareID)
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
	var args []any
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

	records, err := s.db.QueryRaw(ctx, query, args...)
	if err != nil {
		// Fallback to iterative approach
		return s.checkInheritedPermissionsIterative(ctx, objectID, userID, userEmail)
	}

	if len(records) > 0 {
		share := recordToStorageShare(records[0])
		return &share, nil
	}

	return nil, nil
}

// checkInheritedPermissionsIterative is the fallback implementation
func (s *ShareService) checkInheritedPermissionsIterative(ctx context.Context, objectID string, userID string, userEmail string) (*StorageShare, error) {
	// Get the object to find its parent
	rec, err := s.db.Get(ctx, "storage_objects", objectID)
	if err != nil {
		if errors.Is(err, database.ErrNotFound) {
			return nil, ErrObjectNotFound
		}
		return nil, err
	}

	parentFolderID := stringVal(rec.Data["parent_folder_id"])

	// If object has no parent, no inheritance to check
	if parentFolderID == "" {
		return nil, nil
	}

	// Recursively check parent folders for shares
	currentParentID := parentFolderID
	maxDepth := 20
	depth := 0

	var bestShare *StorageShare
	var bestPermissionLevel PermissionLevel = PermissionView

	for currentParentID != "" && depth < maxDepth {
		// Build query for shares on this parent folder
		query := `
			SELECT id, object_id, shared_with_user_id, shared_with_email, permission_level,
			       inherit_to_children, share_token, is_public, expires_at, created_by, created_at, updated_at
			FROM ext_cloudstorage_storage_shares
			WHERE object_id = ? AND inherit_to_children = 1
		`
		args := []any{currentParentID}

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

		records, err := s.db.QueryRaw(ctx, query, args...)
		if err != nil {
			return nil, fmt.Errorf("failed to check parent shares: %w", err)
		}

		for _, rec := range records {
			share := recordToStorageShare(rec)

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
					return bestShare, nil
				}
			}
		}

		// Get the parent folder to continue traversing up
		parentRec, err := s.db.Get(ctx, "storage_objects", currentParentID)
		if err != nil {
			break
		}

		currentParentID = stringVal(parentRec.Data["parent_folder_id"])
		depth++
	}

	return bestShare, nil
}
