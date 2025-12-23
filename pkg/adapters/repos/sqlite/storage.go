//go:build !wasm

package sqlite

import (
	"context"
	"database/sql"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/storage"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type storageRepository struct {
	sqlDB   *sql.DB
	queries *db.Queries
}

// NewStorageRepository creates a new SQLite storage repository
func NewStorageRepository(sqlDB *sql.DB, queries *db.Queries) repos.StorageRepository {
	return &storageRepository{
		sqlDB:   sqlDB,
		queries: queries,
	}
}

// Bucket operations

func (r *storageRepository) CreateBucket(ctx context.Context, bucket *storage.StorageBucket) error {
	if bucket.ID == "" {
		bucket.ID = uuid.NewString()
	}

	public := int64(0)
	if bucket.Public {
		public = 1
	}

	_, err := r.queries.CreateBucket(ctx, db.CreateBucketParams{
		ID:     bucket.ID,
		Name:   bucket.Name,
		Public: &public,
	})
	return err
}

func (r *storageRepository) GetBucket(ctx context.Context, id string) (*storage.StorageBucket, error) {
	dbBucket, err := r.queries.GetBucketByID(ctx, id)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBBucketToModel(dbBucket), nil
}

func (r *storageRepository) GetBucketByName(ctx context.Context, name string) (*storage.StorageBucket, error) {
	dbBucket, err := r.queries.GetBucketByName(ctx, name)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBBucketToModel(dbBucket), nil
}

func (r *storageRepository) ListBuckets(ctx context.Context) ([]*storage.StorageBucket, error) {
	dbBuckets, err := r.queries.ListBuckets(ctx)
	if err != nil {
		return nil, err
	}
	buckets := make([]*storage.StorageBucket, len(dbBuckets))
	for i, b := range dbBuckets {
		buckets[i] = convertDBBucketToModel(b)
	}
	return buckets, nil
}

func (r *storageRepository) UpdateBucket(ctx context.Context, bucket *storage.StorageBucket) error {
	public := int64(0)
	if bucket.Public {
		public = 1
	}

	return r.queries.UpdateBucket(ctx, db.UpdateBucketParams{
		ID:     bucket.ID,
		Name:   bucket.Name,
		Public: &public,
	})
}

func (r *storageRepository) DeleteBucket(ctx context.Context, id string) error {
	return r.queries.DeleteBucket(ctx, id)
}

// Object operations

func (r *storageRepository) CreateObject(ctx context.Context, obj *storage.StorageObject) error {
	if obj.ID == "" {
		obj.ID = uuid.NewString()
	}

	_, err := r.queries.CreateObject(ctx, db.CreateObjectParams{
		ID:             obj.ID,
		BucketName:     obj.BucketName,
		ObjectName:     obj.ObjectName,
		ParentFolderID: obj.ParentFolderID,
		Size:           &obj.Size,
		ContentType:    &obj.ContentType,
		Checksum:       strPtr(obj.Checksum),
		Metadata:       []byte(obj.Metadata),
		UserID:         strPtr(obj.UserID),
		AppID:          obj.AppID,
	})
	return err
}

func (r *storageRepository) GetObject(ctx context.Context, id string) (*storage.StorageObject, error) {
	dbObj, err := r.queries.GetObjectByID(ctx, id)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBObjectToModel(dbObj), nil
}

func (r *storageRepository) GetObjectByPath(ctx context.Context, bucketName, objectName string, parentFolderID *string) (*storage.StorageObject, error) {
	dbObj, err := r.queries.GetObjectByPath(ctx, db.GetObjectByPathParams{
		BucketName: bucketName,
		ObjectName: objectName,
	})
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBObjectToModel(dbObj), nil
}

func (r *storageRepository) GetObjectByChecksum(ctx context.Context, checksum string) (*storage.StorageObject, error) {
	dbObj, err := r.queries.GetObjectByChecksum(ctx, &checksum)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBObjectToModel(dbObj), nil
}

func (r *storageRepository) UpdateObject(ctx context.Context, obj *storage.StorageObject) error {
	return r.queries.UpdateObject(ctx, db.UpdateObjectParams{
		ID:             obj.ID,
		ObjectName:     obj.ObjectName,
		ParentFolderID: obj.ParentFolderID,
		Size:           &obj.Size,
		ContentType:    &obj.ContentType,
		Checksum:       strPtr(obj.Checksum),
		Metadata:       []byte(obj.Metadata),
	})
}

func (r *storageRepository) UpdateObjectLastViewed(ctx context.Context, id string, lastViewed apptime.Time) error {
	return r.queries.UpdateObjectLastViewed(ctx, db.UpdateObjectLastViewedParams{
		ID:         id,
		LastViewed: apptime.NewNullTime(lastViewed),
	})
}

func (r *storageRepository) DeleteObject(ctx context.Context, id string) error {
	return r.queries.DeleteObject(ctx, id)
}

func (r *storageRepository) DeleteObjectsByBucket(ctx context.Context, bucketName string) error {
	return r.queries.DeleteObjectsByBucket(ctx, bucketName)
}

func (r *storageRepository) DeleteObjectsByParentFolder(ctx context.Context, parentFolderID string) error {
	return r.queries.DeleteObjectsByParentFolder(ctx, &parentFolderID)
}

// Object queries

func (r *storageRepository) ListObjects(ctx context.Context, opts repos.ListObjectsOptions) ([]*storage.StorageObject, error) {
	// Build dynamic query with filters
	query := `SELECT id, bucket_name, object_name, parent_folder_id, size, content_type,
		checksum, metadata, created_at, updated_at, last_viewed, user_id, app_id
		FROM storage_objects WHERE bucket_name = ?`
	args := []interface{}{opts.BucketName}

	if opts.UserID != nil {
		query += " AND user_id = ?"
		args = append(args, *opts.UserID)
	}

	if opts.AppID != nil {
		query += " AND app_id = ?"
		args = append(args, *opts.AppID)
	}

	if opts.ParentFolderID != nil {
		query += " AND parent_folder_id = ?"
		args = append(args, *opts.ParentFolderID)
	} else {
		query += " AND parent_folder_id IS NULL"
	}

	if opts.ContentType != nil {
		query += " AND content_type = ?"
		args = append(args, *opts.ContentType)
	}

	query += " ORDER BY updated_at DESC"

	if opts.Limit > 0 {
		query += " LIMIT ?"
		args = append(args, opts.Limit)
	}
	if opts.Offset > 0 {
		query += " OFFSET ?"
		args = append(args, opts.Offset)
	}

	rows, err := r.sqlDB.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var objects []*storage.StorageObject
	for rows.Next() {
		obj := &storage.StorageObject{}
		var parentFolderID, checksum, metadata, userID, appID sql.NullString
		var size sql.NullInt64
		var contentType sql.NullString
		var createdAt, updatedAt string
		var lastViewed sql.NullTime

		if err := rows.Scan(&obj.ID, &obj.BucketName, &obj.ObjectName, &parentFolderID, &size, &contentType,
			&checksum, &metadata, &createdAt, &updatedAt, &lastViewed, &userID, &appID); err != nil {
			continue
		}

		if parentFolderID.Valid {
			obj.ParentFolderID = &parentFolderID.String
		}
		if size.Valid {
			obj.Size = size.Int64
		}
		if contentType.Valid {
			obj.ContentType = contentType.String
		}
		if checksum.Valid {
			obj.Checksum = checksum.String
		}
		if metadata.Valid {
			obj.Metadata = metadata.String
		}
		if userID.Valid {
			obj.UserID = userID.String
		}
		if appID.Valid {
			obj.AppID = &appID.String
		}
		obj.CreatedAt = apptime.MustParse(createdAt)
		obj.UpdatedAt = apptime.MustParse(updatedAt)
		if lastViewed.Valid {
			t := apptime.NewTime(lastViewed.Time)
			obj.LastViewed = apptime.NewNullTime(t)
		}

		objects = append(objects, obj)
	}

	return objects, nil
}

func (r *storageRepository) ListRecentlyViewed(ctx context.Context, userID string, limit int) ([]*storage.StorageObject, error) {
	query := `SELECT id, bucket_name, object_name, parent_folder_id, size, content_type,
		checksum, metadata, created_at, updated_at, last_viewed, user_id, app_id
		FROM storage_objects
		WHERE user_id = ? AND last_viewed IS NOT NULL
		ORDER BY last_viewed DESC LIMIT ?`

	rows, err := r.sqlDB.QueryContext(ctx, query, userID, limit)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var objects []*storage.StorageObject
	for rows.Next() {
		obj := &storage.StorageObject{}
		var parentFolderID, checksum, metadata, userIDVal, appID sql.NullString
		var size sql.NullInt64
		var contentType sql.NullString
		var createdAt, updatedAt string
		var lastViewed sql.NullTime

		if err := rows.Scan(&obj.ID, &obj.BucketName, &obj.ObjectName, &parentFolderID, &size, &contentType,
			&checksum, &metadata, &createdAt, &updatedAt, &lastViewed, &userIDVal, &appID); err != nil {
			continue
		}

		if parentFolderID.Valid {
			obj.ParentFolderID = &parentFolderID.String
		}
		if size.Valid {
			obj.Size = size.Int64
		}
		if contentType.Valid {
			obj.ContentType = contentType.String
		}
		if checksum.Valid {
			obj.Checksum = checksum.String
		}
		if metadata.Valid {
			obj.Metadata = metadata.String
		}
		if userIDVal.Valid {
			obj.UserID = userIDVal.String
		}
		if appID.Valid {
			obj.AppID = &appID.String
		}
		obj.CreatedAt = apptime.MustParse(createdAt)
		obj.UpdatedAt = apptime.MustParse(updatedAt)
		if lastViewed.Valid {
			t := apptime.NewTime(lastViewed.Time)
			obj.LastViewed = apptime.NewNullTime(t)
		}

		objects = append(objects, obj)
	}

	return objects, nil
}

func (r *storageRepository) SearchObjects(ctx context.Context, userID, appID, searchPattern string, limit int) ([]*storage.StorageObject, error) {
	query := `SELECT id, bucket_name, object_name, parent_folder_id, size, content_type,
		checksum, metadata, created_at, updated_at, last_viewed, user_id, app_id
		FROM storage_objects
		WHERE user_id = ? AND app_id = ? AND object_name LIKE ?
		ORDER BY updated_at DESC LIMIT ?`

	rows, err := r.sqlDB.QueryContext(ctx, query, userID, appID, searchPattern, limit)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var objects []*storage.StorageObject
	for rows.Next() {
		obj := &storage.StorageObject{}
		var parentFolderID, checksum, metadata, userIDVal, appIDVal sql.NullString
		var size sql.NullInt64
		var contentType sql.NullString
		var createdAt, updatedAt string
		var lastViewed sql.NullTime

		if err := rows.Scan(&obj.ID, &obj.BucketName, &obj.ObjectName, &parentFolderID, &size, &contentType,
			&checksum, &metadata, &createdAt, &updatedAt, &lastViewed, &userIDVal, &appIDVal); err != nil {
			continue
		}

		if parentFolderID.Valid {
			obj.ParentFolderID = &parentFolderID.String
		}
		if size.Valid {
			obj.Size = size.Int64
		}
		if contentType.Valid {
			obj.ContentType = contentType.String
		}
		if checksum.Valid {
			obj.Checksum = checksum.String
		}
		if metadata.Valid {
			obj.Metadata = metadata.String
		}
		if userIDVal.Valid {
			obj.UserID = userIDVal.String
		}
		if appIDVal.Valid {
			obj.AppID = &appIDVal.String
		}
		obj.CreatedAt = apptime.MustParse(createdAt)
		obj.UpdatedAt = apptime.MustParse(updatedAt)
		if lastViewed.Valid {
			t := apptime.NewTime(lastViewed.Time)
			obj.LastViewed = apptime.NewNullTime(t)
		}

		objects = append(objects, obj)
	}

	return objects, nil
}

func (r *storageRepository) CountObjectsByBucket(ctx context.Context, bucketName string) (int64, error) {
	return r.queries.CountObjectsByBucket(ctx, bucketName)
}

func (r *storageRepository) CountObjectsByUser(ctx context.Context, userID string) (int64, error) {
	return r.queries.CountObjectsByUser(ctx, &userID)
}

func (r *storageRepository) SumSizeByBucket(ctx context.Context, bucketName string) (int64, error) {
	size, err := r.queries.SumSizeByBucket(ctx, bucketName)
	if err != nil {
		return 0, err
	}
	// sqlc returns interface{} for COALESCE, convert to int64
	switch v := size.(type) {
	case int64:
		return v, nil
	case float64:
		return int64(v), nil
	default:
		return 0, nil
	}
}

func (r *storageRepository) SumSizeByUser(ctx context.Context, userID string) (int64, error) {
	size, err := r.queries.SumSizeByUser(ctx, &userID)
	if err != nil {
		return 0, err
	}
	// sqlc returns interface{} for COALESCE, convert to int64
	switch v := size.(type) {
	case int64:
		return v, nil
	case float64:
		return int64(v), nil
	default:
		return 0, nil
	}
}

func (r *storageRepository) SumTotalSize(ctx context.Context) (int64, error) {
	// Use raw query for total size
	var total sql.NullInt64
	err := r.sqlDB.QueryRowContext(ctx, "SELECT COALESCE(SUM(size), 0) FROM storage_objects").Scan(&total)
	if err != nil {
		return 0, err
	}
	return total.Int64, nil
}

// Upload token operations

func (r *storageRepository) CreateUploadToken(ctx context.Context, token *repos.UploadToken) error {
	now := apptime.NowString()
	if token.ID == "" {
		token.ID = uuid.NewString()
	}

	_, err := r.queries.CreateUploadToken(ctx, db.CreateUploadTokenParams{
		ID:             token.ID,
		Token:          token.Token,
		Bucket:         token.Bucket,
		ParentFolderID: token.ParentFolderID,
		ObjectName:     token.ObjectName,
		UserID:         token.UserID,
		MaxSize:        token.MaxSize,
		ContentType:    token.ContentType,
		ExpiresAt:      token.ExpiresAt,
		CreatedAt:      now,
		ClientIp:       token.ClientIP,
	})
	return err
}

func (r *storageRepository) GetUploadToken(ctx context.Context, id string) (*repos.UploadToken, error) {
	dbToken, err := r.queries.GetUploadTokenByID(ctx, id)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBUploadTokenToModel(dbToken), nil
}

func (r *storageRepository) GetUploadTokenByToken(ctx context.Context, token string) (*repos.UploadToken, error) {
	dbToken, err := r.queries.GetUploadTokenByToken(ctx, token)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBUploadTokenToModel(dbToken), nil
}

func (r *storageRepository) UpdateUploadTokenProgress(ctx context.Context, id string, bytesUploaded int64) error {
	return r.queries.UpdateUploadTokenProgress(ctx, db.UpdateUploadTokenProgressParams{
		ID:            id,
		BytesUploaded: &bytesUploaded,
	})
}

func (r *storageRepository) CompleteUploadToken(ctx context.Context, id, objectID string) error {
	return r.queries.CompleteUploadToken(ctx, db.CompleteUploadTokenParams{
		ID:          id,
		ObjectID:    &objectID,
		CompletedAt: apptime.NewNullTimeNow(),
	})
}

func (r *storageRepository) DeleteUploadToken(ctx context.Context, id string) error {
	return r.queries.DeleteUploadToken(ctx, id)
}

func (r *storageRepository) DeleteExpiredUploadTokens(ctx context.Context) error {
	return r.queries.DeleteExpiredUploadTokens(ctx, apptime.NewNullTimeNow())
}

// Download token operations

func (r *storageRepository) CreateDownloadToken(ctx context.Context, token *repos.DownloadToken) error {
	now := apptime.NowString()
	if token.ID == "" {
		token.ID = uuid.NewString()
	}

	_, err := r.queries.CreateDownloadToken(ctx, db.CreateDownloadTokenParams{
		ID:             token.ID,
		Token:          token.Token,
		FileID:         token.FileID,
		Bucket:         token.Bucket,
		ParentFolderID: token.ParentFolderID,
		ObjectName:     token.ObjectName,
		UserID:         token.UserID,
		FileSize:       token.FileSize,
		ExpiresAt:      token.ExpiresAt,
		CreatedAt:      now,
		ClientIp:       token.ClientIP,
	})
	return err
}

func (r *storageRepository) GetDownloadToken(ctx context.Context, id string) (*repos.DownloadToken, error) {
	dbToken, err := r.queries.GetDownloadTokenByID(ctx, id)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBDownloadTokenToModel(dbToken), nil
}

func (r *storageRepository) GetDownloadTokenByToken(ctx context.Context, token string) (*repos.DownloadToken, error) {
	dbToken, err := r.queries.GetDownloadTokenByToken(ctx, token)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBDownloadTokenToModel(dbToken), nil
}

func (r *storageRepository) UpdateDownloadTokenProgress(ctx context.Context, id string, bytesServed int64) error {
	return r.queries.UpdateDownloadTokenProgress(ctx, db.UpdateDownloadTokenProgressParams{
		ID:          id,
		BytesServed: &bytesServed,
	})
}

func (r *storageRepository) CompleteDownloadToken(ctx context.Context, id string) error {
	return r.queries.CompleteDownloadToken(ctx, db.CompleteDownloadTokenParams{
		ID:         id,
		CallbackAt: apptime.NewNullTimeNow(),
	})
}

func (r *storageRepository) DeleteDownloadToken(ctx context.Context, id string) error {
	return r.queries.DeleteDownloadToken(ctx, id)
}

func (r *storageRepository) DeleteExpiredDownloadTokens(ctx context.Context) error {
	return r.queries.DeleteExpiredDownloadTokens(ctx, apptime.NewNullTimeNow())
}

// Conversion helpers

func convertDBBucketToModel(dbBucket db.StorageBucket) *storage.StorageBucket {
	public := false
	if dbBucket.Public != nil && *dbBucket.Public == 1 {
		public = true
	}

	return &storage.StorageBucket{
		ID:        dbBucket.ID,
		Name:      dbBucket.Name,
		Public:    public,
		CreatedAt: apptime.MustParse(dbBucket.CreatedAt),
		UpdatedAt: apptime.MustParse(dbBucket.UpdatedAt),
	}
}

func convertDBObjectToModel(dbObj db.StorageObject) *storage.StorageObject {
	var size int64
	var contentType, checksum, metadata, userID string
	if dbObj.Size != nil {
		size = *dbObj.Size
	}
	if dbObj.ContentType != nil {
		contentType = *dbObj.ContentType
	}
	if dbObj.Checksum != nil {
		checksum = *dbObj.Checksum
	}
	if dbObj.Metadata != nil {
		metadata = string(dbObj.Metadata)
	}
	if dbObj.UserID != nil {
		userID = *dbObj.UserID
	}

	return &storage.StorageObject{
		ID:             dbObj.ID,
		BucketName:     dbObj.BucketName,
		ObjectName:     dbObj.ObjectName,
		ParentFolderID: dbObj.ParentFolderID,
		Size:           size,
		ContentType:    contentType,
		Checksum:       checksum,
		Metadata:       metadata,
		CreatedAt:      apptime.MustParse(dbObj.CreatedAt),
		UpdatedAt:      apptime.MustParse(dbObj.UpdatedAt),
		LastViewed:     dbObj.LastViewed,
		UserID:         userID,
		AppID:          dbObj.AppID,
	}
}

func convertDBUploadTokenToModel(dbToken db.StorageUploadToken) *repos.UploadToken {
	completed := false
	if dbToken.Completed != nil && *dbToken.Completed == 1 {
		completed = true
	}

	return &repos.UploadToken{
		ID:             dbToken.ID,
		Token:          dbToken.Token,
		Bucket:         dbToken.Bucket,
		ParentFolderID: dbToken.ParentFolderID,
		ObjectName:     dbToken.ObjectName,
		UserID:         dbToken.UserID,
		MaxSize:        dbToken.MaxSize,
		ContentType:    dbToken.ContentType,
		BytesUploaded:  dbToken.BytesUploaded,
		Completed:      completed,
		ObjectID:       dbToken.ObjectID,
		ExpiresAt:      dbToken.ExpiresAt,
		CreatedAt:      apptime.MustParse(dbToken.CreatedAt),
		CompletedAt:    dbToken.CompletedAt,
		ClientIP:       dbToken.ClientIp,
	}
}

func convertDBDownloadTokenToModel(dbToken db.StorageDownloadToken) *repos.DownloadToken {
	completed := false
	if dbToken.Completed != nil && *dbToken.Completed == 1 {
		completed = true
	}

	return &repos.DownloadToken{
		ID:             dbToken.ID,
		Token:          dbToken.Token,
		FileID:         dbToken.FileID,
		Bucket:         dbToken.Bucket,
		ParentFolderID: dbToken.ParentFolderID,
		ObjectName:     dbToken.ObjectName,
		UserID:         dbToken.UserID,
		FileSize:       dbToken.FileSize,
		BytesServed:    dbToken.BytesServed,
		Completed:      completed,
		ExpiresAt:      dbToken.ExpiresAt,
		CreatedAt:      apptime.MustParse(dbToken.CreatedAt),
		CallbackAt:     dbToken.CallbackAt,
		ClientIP:       dbToken.ClientIp,
	}
}

// Ensure storageRepository implements StorageRepository
var _ repos.StorageRepository = (*storageRepository)(nil)
