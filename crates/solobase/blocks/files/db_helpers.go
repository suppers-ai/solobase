package files

import (
	"context"
	"fmt"

	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/waffle-go/services/database"
)

// Database helper methods for FilesBlock

func (b *FilesBlock) getStorageObjectByID(id string) (*storageObject, error) {
	if b.db == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	rec, err := b.db.Get(ctx, "storage_objects", id)
	if err != nil {
		return nil, err
	}
	return recordToStorageObject(rec), nil
}

func (b *FilesBlock) countStorageQuotas() (int64, error) {
	if b.db == nil {
		return 0, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	count, err := b.db.Count(ctx, "ext_cloudstorage_storage_quotas", nil)
	if err != nil {
		return 0, err
	}
	return int64(count), nil
}

func (b *FilesBlock) getAllStorageQuotas() ([]StorageQuota, error) {
	if b.db == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	records, err := database.ListAll(ctx, b.db, "ext_cloudstorage_storage_quotas")
	if err != nil {
		return nil, err
	}
	var quotas []StorageQuota
	for _, rec := range records {
		quotas = append(quotas, recordToStorageQuota(rec))
	}
	return quotas, nil
}

func (b *FilesBlock) saveStorageQuota(quota *StorageQuota) error {
	if b.db == nil {
		return fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	_, err := database.Upsert(ctx, b.db, "ext_cloudstorage_storage_quotas", "user_id", quota.UserID, map[string]any{
		"id":                  quota.ID,
		"user_id":             quota.UserID,
		"max_storage_bytes":   quota.MaxStorageBytes,
		"max_bandwidth_bytes": quota.MaxBandwidthBytes,
		"storage_used":        quota.StorageUsed,
		"bandwidth_used":      quota.BandwidthUsed,
		"reset_bandwidth_at":  nullTimeToAny(quota.ResetBandwidthAt),
		"updated_at":          apptime.NowString(),
	})
	return err
}

func (b *FilesBlock) countStorageObjectsByUser(userID string) (int64, error) {
	if b.db == nil {
		return 0, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	count, err := database.CountByField(ctx, b.db, "storage_objects", "user_id", userID)
	if err != nil {
		return 0, err
	}
	return int64(count), nil
}

func (b *FilesBlock) sumStorageSizeByUser(userID string) (int64, error) {
	if b.db == nil {
		return 0, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	sum, err := b.db.Sum(ctx, "storage_objects", "size", []database.Filter{
		{Field: "user_id", Operator: database.OpEqual, Value: userID},
	})
	if err != nil {
		return 0, err
	}
	return int64(sum), nil
}

func (b *FilesBlock) getUserEmail(userID string) (string, error) {
	if b.db == nil {
		return "", fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	rec, err := b.db.Get(ctx, "auth_users", userID)
	if err != nil {
		return "", err
	}
	return stringVal(rec.Data["email"]), nil
}

func (b *FilesBlock) countShares(condition string, args ...interface{}) (int64, error) {
	if b.db == nil {
		return 0, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	query := "SELECT COUNT(*) as count FROM ext_cloudstorage_storage_shares"
	if condition != "" {
		query += " WHERE " + condition
	}
	anyArgs := make([]any, len(args))
	for i, a := range args {
		anyArgs[i] = a
	}
	records, err := b.db.QueryRaw(ctx, query, anyArgs...)
	if err != nil {
		return 0, err
	}
	if len(records) == 0 {
		return 0, nil
	}
	return toInt64Val(records[0].Data["count"]), nil
}

func (b *FilesBlock) getRoleQuotaByRoleID(roleID string) (*RoleQuota, error) {
	if b.db == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	rec, err := database.GetByField(ctx, b.db, "ext_cloudstorage_role_quotas", "role_id", roleID)
	if err != nil {
		return nil, err
	}
	rq := recordToRoleQuota(rec)
	return &rq, nil
}

func (b *FilesBlock) getAllRoleQuotas() ([]RoleQuota, error) {
	if b.db == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	records, err := database.ListAll(ctx, b.db, "ext_cloudstorage_role_quotas")
	if err != nil {
		return nil, err
	}
	var quotas []RoleQuota
	for _, rec := range records {
		quotas = append(quotas, recordToRoleQuota(rec))
	}
	return quotas, nil
}

func (b *FilesBlock) getActiveUserQuotaOverrides() ([]UserQuotaOverride, error) {
	if b.db == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	now := apptime.NowString()
	records, err := b.db.QueryRaw(ctx, `
		SELECT id, user_id, max_storage_bytes, max_bandwidth_bytes, max_upload_size, max_files_count, allowed_extensions, blocked_extensions, reason, expires_at, created_by, created_at, updated_at
		FROM ext_cloudstorage_user_quota_overrides
		WHERE expires_at IS NULL OR expires_at > ?`, now)
	if err != nil {
		return nil, err
	}
	var overrides []UserQuotaOverride
	for _, rec := range records {
		overrides = append(overrides, recordToUserQuotaOverride(rec))
	}
	return overrides, nil
}

func (b *FilesBlock) createUserQuotaOverride(override *UserQuotaOverride) error {
	if b.db == nil {
		return fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
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
	_, err := b.db.Create(ctx, "ext_cloudstorage_user_quota_overrides", data)
	return err
}

func (b *FilesBlock) deleteUserQuotaOverrideByID(id string) error {
	if b.db == nil {
		return fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	return b.db.Delete(ctx, "ext_cloudstorage_user_quota_overrides", id)
}

func (b *FilesBlock) listSharesWithObjects(objectID, userID string, isAdmin bool) ([]map[string]interface{}, error) {
	if b.db == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()

	var query string
	var args []any
	if isAdmin {
		if objectID != "" {
			query = "SELECT ss.*, u.email as shared_by_email FROM ext_cloudstorage_storage_shares ss LEFT JOIN auth_users u ON ss.created_by = u.id WHERE ss.object_id = ? ORDER BY ss.created_at DESC"
			args = []any{objectID}
		} else {
			query = "SELECT ss.*, u.email as shared_by_email FROM ext_cloudstorage_storage_shares ss LEFT JOIN auth_users u ON ss.created_by = u.id ORDER BY ss.created_at DESC"
		}
	} else {
		if objectID != "" {
			query = "SELECT ss.*, u.email as shared_by_email FROM ext_cloudstorage_storage_shares ss LEFT JOIN auth_users u ON ss.created_by = u.id WHERE ss.object_id = ? AND ss.created_by = ? ORDER BY ss.created_at DESC"
			args = []any{objectID, userID}
		} else {
			query = "SELECT ss.*, u.email as shared_by_email FROM ext_cloudstorage_storage_shares ss LEFT JOIN auth_users u ON ss.created_by = u.id WHERE ss.created_by = ? ORDER BY ss.created_at DESC"
			args = []any{userID}
		}
	}

	records, err := b.db.QueryRaw(ctx, query, args...)
	if err != nil {
		return nil, err
	}

	var results []map[string]interface{}
	for _, rec := range records {
		d := rec.Data
		result := map[string]interface{}{
			"id":               rec.ID,
			"objectId":         d["object_id"],
			"sharedWithUserId": d["shared_with_user_id"],
			"sharedWithEmail":  d["shared_with_email"],
			"permissionLevel":  d["permission_level"],
			"shareToken":       d["share_token"],
			"isPublic":         d["is_public"],
			"expiresAt":        d["expires_at"],
			"createdBy":        d["created_by"],
			"createdAt":        d["created_at"],
		}
		if email, ok := d["shared_by_email"]; ok && email != nil {
			result["sharedByEmail"] = email
		}
		results = append(results, result)
	}
	return results, nil
}

func (b *FilesBlock) getStorageStats(userID string, isAdmin bool) (totalObjects int64, totalSize int64, err error) {
	if b.db == nil {
		return 0, 0, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	var query string
	var args []any
	if isAdmin {
		query = "SELECT COUNT(*) as cnt, COALESCE(SUM(size), 0) as total FROM storage_objects"
	} else {
		query = "SELECT COUNT(*) as cnt, COALESCE(SUM(size), 0) as total FROM storage_objects WHERE user_id = ?"
		args = []any{userID}
	}
	records, err := b.db.QueryRaw(ctx, query, args...)
	if err != nil {
		return 0, 0, err
	}
	if len(records) == 0 {
		return 0, 0, nil
	}
	totalObjects = toInt64Val(records[0].Data["cnt"])
	totalSize = toInt64Val(records[0].Data["total"])
	return totalObjects, totalSize, nil
}

func (b *FilesBlock) getQuotaAggregateStats() (totalUsers, totalStorageUsed, totalStorageLimit, totalBandwidthUsed, totalBandwidthLimit int64, err error) {
	if b.db == nil {
		return 0, 0, 0, 0, 0, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	records, err := b.db.QueryRaw(ctx, `
		SELECT
			COUNT(*) as total_users,
			COALESCE(SUM(storage_used), 0) as total_storage_used,
			COALESCE(SUM(max_storage_bytes), 0) as total_storage_limit,
			COALESCE(SUM(bandwidth_used), 0) as total_bandwidth_used,
			COALESCE(SUM(max_bandwidth_bytes), 0) as total_bandwidth_limit
		FROM ext_cloudstorage_storage_quotas
	`)
	if err != nil {
		return 0, 0, 0, 0, 0, err
	}
	if len(records) == 0 {
		return 0, 0, 0, 0, 0, nil
	}
	d := records[0].Data
	totalUsers = toInt64Val(d["total_users"])
	totalStorageUsed = toInt64Val(d["total_storage_used"])
	totalStorageLimit = toInt64Val(d["total_storage_limit"])
	totalBandwidthUsed = toInt64Val(d["total_bandwidth_used"])
	totalBandwidthLimit = toInt64Val(d["total_bandwidth_limit"])
	return
}

func (b *FilesBlock) countUsersNearQuotaLimit() (int64, error) {
	if b.db == nil {
		return 0, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	records, err := b.db.QueryRaw(ctx, `
		SELECT COUNT(*) as count FROM ext_cloudstorage_storage_quotas
		WHERE (max_storage_bytes > 0 AND (storage_used * 100.0 / max_storage_bytes) > 80)
		   OR (max_bandwidth_bytes > 0 AND (bandwidth_used * 100.0 / max_bandwidth_bytes) > 80)
	`)
	if err != nil {
		return 0, err
	}
	if len(records) == 0 {
		return 0, nil
	}
	return toInt64Val(records[0].Data["count"]), nil
}

func (b *FilesBlock) countSharedFolders(userID string, isAdmin bool) (int64, error) {
	if b.db == nil {
		return 0, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	var query string
	var args []any
	if isAdmin {
		query = `SELECT COUNT(*) as count FROM ext_cloudstorage_storage_shares ss
			JOIN storage_objects so ON ss.object_id = so.id
			WHERE so.content_type = 'application/x-directory'`
	} else {
		query = `SELECT COUNT(*) as count FROM ext_cloudstorage_storage_shares ss
			JOIN storage_objects so ON ss.object_id = so.id
			WHERE ss.created_by = ? AND so.content_type = 'application/x-directory'`
		args = []any{userID}
	}
	records, err := b.db.QueryRaw(ctx, query, args...)
	if err != nil {
		return 0, err
	}
	if len(records) == 0 {
		return 0, nil
	}
	return toInt64Val(records[0].Data["count"]), nil
}

func (b *FilesBlock) getShareByObjectAndUser(objectID, userID string) (*StorageShare, error) {
	if b.db == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	records, err := b.db.QueryRaw(ctx, `
		SELECT id, object_id, shared_with_user_id, shared_with_email, permission_level, inherit_to_children, share_token, is_public, expires_at, created_by, created_at, updated_at
		FROM ext_cloudstorage_storage_shares
		WHERE object_id = ? AND (is_public = 1 OR shared_with_user_id = ?)
		LIMIT 1`,
		objectID, userID)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, database.ErrNotFound
	}
	share := recordToStorageShare(records[0])
	return &share, nil
}

func (b *FilesBlock) getShareByObjectPublicOnly(objectID string) (*StorageShare, error) {
	if b.db == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	records, err := b.db.QueryRaw(ctx, `
		SELECT id, object_id, shared_with_user_id, shared_with_email, permission_level, inherit_to_children, share_token, is_public, expires_at, created_by, created_at, updated_at
		FROM ext_cloudstorage_storage_shares
		WHERE object_id = ? AND is_public = 1
		LIMIT 1`,
		objectID)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, database.ErrNotFound
	}
	share := recordToStorageShare(records[0])
	return &share, nil
}

func (b *FilesBlock) searchUsers(query string) ([]map[string]string, error) {
	if b.db == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	likeVal := "%" + query + "%"
	records, err := b.db.QueryRaw(ctx, `
		SELECT id, email FROM auth_users
		WHERE email LIKE ? OR id LIKE ?
		LIMIT 10`,
		likeVal, likeVal)
	if err != nil {
		return nil, err
	}
	var users []map[string]string
	for _, rec := range records {
		users = append(users, map[string]string{
			"id":    rec.ID,
			"email": stringVal(rec.Data["email"]),
		})
	}
	return users, nil
}

func (b *FilesBlock) upsertRoleQuota(quota *RoleQuota) error {
	if b.db == nil {
		return fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	now := apptime.NowString()
	_, err := b.db.ExecRaw(ctx, `
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
	`, quota.ID, quota.RoleID, quota.RoleName, quota.MaxStorageBytes, quota.MaxBandwidthBytes, quota.MaxUploadSize, quota.MaxFilesCount, stringPtrOrNil(quota.AllowedExtensions), stringPtrOrNil(quota.BlockedExtensions), now, now)
	return err
}

func (b *FilesBlock) createAccessLog(logEntry *StorageAccessLog) error {
	if b.db == nil {
		return fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	_, err := b.db.Create(ctx, "ext_cloudstorage_storage_access_logs", map[string]any{
		"id":         logEntry.ID,
		"object_id":  logEntry.ObjectID,
		"user_id":    logEntry.UserID,
		"ip_address": logEntry.IPAddress,
		"action":     string(logEntry.Action),
		"user_agent": logEntry.UserAgent,
		"metadata":   logEntry.Metadata,
	})
	return err
}

func (b *FilesBlock) getMyFilesFolder(userID, appID string) (*storageObject, error) {
	if b.db == nil {
		return nil, fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	records, err := b.db.QueryRaw(ctx, `
		SELECT id, bucket_name, object_name, parent_folder_id, size, content_type, checksum, metadata, created_at, updated_at, last_viewed, user_id, app_id
		FROM storage_objects
		WHERE bucket_name = 'int_storage' AND user_id = ? AND app_id = ? AND object_name = 'My Files' AND content_type = 'application/x-directory' AND parent_folder_id IS NULL`,
		userID, appID)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, database.ErrNotFound
	}
	obj := recordToStorageObject(records[0])
	return obj, nil
}

func (b *FilesBlock) createMyFilesFolder(obj *storageObject) error {
	if b.db == nil {
		return fmt.Errorf("database not initialized")
	}
	ctx := context.Background()
	_, err := b.db.Create(ctx, "storage_objects", map[string]any{
		"id":               obj.ID,
		"bucket_name":      obj.BucketName,
		"object_name":      obj.ObjectName,
		"parent_folder_id": obj.ParentFolderID,
		"size":             obj.Size,
		"content_type":     obj.ContentType,
		"checksum":         obj.Checksum,
		"metadata":         obj.Metadata,
		"user_id":          obj.UserID,
		"app_id":           obj.AppID,
	})
	return err
}

