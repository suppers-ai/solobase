package files

import (
	"context"

	adapterstorage "github.com/suppers-ai/solobase/adapters/storage"
	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/solobase/core/uuid"
	"github.com/suppers-ai/waffle-go/services/database"
)

// storageRepository provides all storage metadata database operations
// using database.Service (no raw SQL).
type storageRepository struct {
	db database.Service
}

// newStorageRepository creates a new storage repository backed by database.Service.
func newStorageRepository(db database.Service) *storageRepository {
	return &storageRepository{db: db}
}

// ==================== Bucket operations ====================

func (r *storageRepository) CreateBucket(ctx context.Context, bucket *adapterstorage.StorageBucket) error {
	if bucket.ID == "" {
		bucket.ID = uuid.NewString()
	}
	now := apptime.NowString()

	publicVal := int64(0)
	if bucket.Public {
		publicVal = 1
	}

	_, err := r.db.Create(ctx, "storage_buckets", map[string]any{
		"id":         bucket.ID,
		"name":       bucket.Name,
		"public":     publicVal,
		"created_at": now,
		"updated_at": now,
	})
	return err
}

func (r *storageRepository) GetBucket(ctx context.Context, id string) (*adapterstorage.StorageBucket, error) {
	rec, err := r.db.Get(ctx, "storage_buckets", id)
	if err != nil {
		if err == database.ErrNotFound {
			return nil, ErrNotFound
		}
		return nil, err
	}
	return recordToBucket(rec), nil
}

func (r *storageRepository) GetBucketByName(ctx context.Context, name string) (*adapterstorage.StorageBucket, error) {
	rec, err := database.GetByField(ctx, r.db, "storage_buckets", "name", name)
	if err != nil {
		if err == database.ErrNotFound {
			return nil, ErrNotFound
		}
		return nil, err
	}
	return recordToBucket(rec), nil
}

func (r *storageRepository) ListBuckets(ctx context.Context) ([]*adapterstorage.StorageBucket, error) {
	result, err := r.db.List(ctx, "storage_buckets", &database.ListOptions{
		Sort:  []database.SortField{{Field: "name"}},
		Limit: 10000,
	})
	if err != nil {
		return nil, err
	}
	buckets := make([]*adapterstorage.StorageBucket, len(result.Records))
	for i, rec := range result.Records {
		buckets[i] = recordToBucket(rec)
	}
	return buckets, nil
}

func (r *storageRepository) ListBucketsWithStats(ctx context.Context) ([]*adapterstorage.StorageBucket, map[string]*BucketStats, error) {
	// Use QueryRaw for the JOIN query
	records, err := r.db.QueryRaw(ctx, `
		SELECT
			b.id, b.name, b.public, b.created_at, b.updated_at,
			COALESCE(COUNT(o.id), 0) as object_count,
			COALESCE(SUM(o.size), 0) as total_size
		FROM storage_buckets b
		LEFT JOIN storage_objects o ON b.name = o.bucket_name AND o.is_folder = 0
		GROUP BY b.id, b.name, b.public, b.created_at, b.updated_at
		ORDER BY b.name
	`)
	if err != nil {
		return nil, nil, err
	}

	var buckets []*adapterstorage.StorageBucket
	stats := make(map[string]*BucketStats)

	for _, rec := range records {
		bucket := &adapterstorage.StorageBucket{
			ID:   stringVal(rec.Data["id"]),
			Name: stringVal(rec.Data["name"]),
		}
		bucket.Public = toInt64Val(rec.Data["public"]) == 1
		bucket.CreatedAt = apptime.MustParse(stringVal(rec.Data["created_at"]))
		bucket.UpdatedAt = apptime.MustParse(stringVal(rec.Data["updated_at"]))

		buckets = append(buckets, bucket)
		stats[bucket.Name] = &BucketStats{
			BucketName:  bucket.Name,
			ObjectCount: toInt64Val(rec.Data["object_count"]),
			TotalSize:   toInt64Val(rec.Data["total_size"]),
		}
	}

	return buckets, stats, nil
}

func (r *storageRepository) UpdateBucket(ctx context.Context, bucket *adapterstorage.StorageBucket) error {
	publicVal := int64(0)
	if bucket.Public {
		publicVal = 1
	}

	_, err := r.db.Update(ctx, "storage_buckets", bucket.ID, map[string]any{
		"name":   bucket.Name,
		"public": publicVal,
	})
	return err
}

func (r *storageRepository) DeleteBucket(ctx context.Context, id string) error {
	return r.db.Delete(ctx, "storage_buckets", id)
}

// ==================== Object operations ====================

func (r *storageRepository) CreateObject(ctx context.Context, obj *adapterstorage.StorageObject) error {
	if obj.ID == "" {
		obj.ID = uuid.NewString()
	}
	now := apptime.NowString()

	data := map[string]any{
		"id":          obj.ID,
		"bucket_name": obj.BucketName,
		"object_name": obj.ObjectName,
		"size":        obj.Size,
		"content_type": obj.ContentType,
		"metadata":    obj.Metadata,
		"app_id":      obj.AppID,
		"created_at":  now,
		"updated_at":  now,
	}

	if obj.ParentFolderID != nil {
		data["parent_folder_id"] = *obj.ParentFolderID
	}
	if obj.Checksum != "" {
		data["checksum"] = obj.Checksum
	}
	if obj.UserID != "" {
		data["user_id"] = obj.UserID
	}

	_, err := r.db.Create(ctx, "storage_objects", data)
	return err
}

func (r *storageRepository) GetObject(ctx context.Context, id string) (*adapterstorage.StorageObject, error) {
	rec, err := r.db.Get(ctx, "storage_objects", id)
	if err != nil {
		if err == database.ErrNotFound {
			return nil, ErrNotFound
		}
		return nil, err
	}
	return recordToObject(rec), nil
}

func (r *storageRepository) GetObjectByPath(ctx context.Context, bucketName, objectName string, parentFolderID *string) (*adapterstorage.StorageObject, error) {
	filters := []database.Filter{
		{Field: "bucket_name", Operator: database.OpEqual, Value: bucketName},
		{Field: "object_name", Operator: database.OpEqual, Value: objectName},
	}

	if parentFolderID == nil {
		filters = append(filters, database.Filter{Field: "parent_folder_id", Operator: database.OpIsNull})
	} else {
		filters = append(filters, database.Filter{Field: "parent_folder_id", Operator: database.OpEqual, Value: *parentFolderID})
	}

	result, err := r.db.List(ctx, "storage_objects", &database.ListOptions{
		Filters: filters,
		Limit:   1,
	})
	if err != nil {
		return nil, err
	}
	if len(result.Records) == 0 {
		return nil, ErrNotFound
	}
	return recordToObject(result.Records[0]), nil
}

func (r *storageRepository) GetObjectByChecksum(ctx context.Context, checksum string) (*adapterstorage.StorageObject, error) {
	rec, err := database.GetByField(ctx, r.db, "storage_objects", "checksum", checksum)
	if err != nil {
		if err == database.ErrNotFound {
			return nil, ErrNotFound
		}
		return nil, err
	}
	return recordToObject(rec), nil
}

func (r *storageRepository) UpdateObject(ctx context.Context, obj *adapterstorage.StorageObject) error {
	data := map[string]any{
		"object_name":  obj.ObjectName,
		"size":         obj.Size,
		"content_type": obj.ContentType,
		"metadata":     obj.Metadata,
	}

	if obj.ParentFolderID != nil {
		data["parent_folder_id"] = *obj.ParentFolderID
	}
	if obj.Checksum != "" {
		data["checksum"] = obj.Checksum
	}

	_, err := r.db.Update(ctx, "storage_objects", obj.ID, data)
	return err
}

func (r *storageRepository) UpdateObjectLastViewed(ctx context.Context, id string, lastViewed apptime.Time) error {
	_, err := r.db.Update(ctx, "storage_objects", id, map[string]any{
		"last_viewed": apptime.Format(lastViewed),
	})
	return err
}

func (r *storageRepository) DeleteObject(ctx context.Context, id string) error {
	return r.db.Delete(ctx, "storage_objects", id)
}

func (r *storageRepository) DeleteObjectsByBucket(ctx context.Context, bucketName string) error {
	return database.DeleteByField(ctx, r.db, "storage_objects", "bucket_name", bucketName)
}

func (r *storageRepository) DeleteObjectsByParentFolder(ctx context.Context, parentFolderID string) error {
	return database.DeleteByField(ctx, r.db, "storage_objects", "parent_folder_id", parentFolderID)
}

// ==================== Object queries ====================

func (r *storageRepository) ListObjects(ctx context.Context, opts ListObjectsOptions) ([]*adapterstorage.StorageObject, error) {
	filters := []database.Filter{
		{Field: "bucket_name", Operator: database.OpEqual, Value: opts.BucketName},
	}

	if opts.UserID != nil {
		filters = append(filters, database.Filter{Field: "user_id", Operator: database.OpEqual, Value: *opts.UserID})
	}
	if opts.AppID != nil {
		filters = append(filters, database.Filter{Field: "app_id", Operator: database.OpEqual, Value: *opts.AppID})
	}
	if opts.ParentFolderID != nil {
		filters = append(filters, database.Filter{Field: "parent_folder_id", Operator: database.OpEqual, Value: *opts.ParentFolderID})
	} else {
		filters = append(filters, database.Filter{Field: "parent_folder_id", Operator: database.OpIsNull})
	}
	if opts.ContentType != nil {
		filters = append(filters, database.Filter{Field: "content_type", Operator: database.OpEqual, Value: *opts.ContentType})
	}

	listOpts := &database.ListOptions{
		Filters: filters,
		Sort:    []database.SortField{{Field: "updated_at", Desc: true}},
		Limit:   10000,
	}
	if opts.Limit > 0 {
		listOpts.Limit = opts.Limit
	}
	if opts.Offset > 0 {
		listOpts.Offset = opts.Offset
	}

	result, err := r.db.List(ctx, "storage_objects", listOpts)
	if err != nil {
		return nil, err
	}
	return recordsToObjects(result.Records), nil
}

func (r *storageRepository) ListRecentlyViewed(ctx context.Context, userID string, limit int) ([]*adapterstorage.StorageObject, error) {
	result, err := r.db.List(ctx, "storage_objects", &database.ListOptions{
		Filters: []database.Filter{
			{Field: "user_id", Operator: database.OpEqual, Value: userID},
			{Field: "last_viewed", Operator: database.OpIsNotNull},
		},
		Sort:  []database.SortField{{Field: "last_viewed", Desc: true}},
		Limit: limit,
	})
	if err != nil {
		return nil, err
	}
	return recordsToObjects(result.Records), nil
}

func (r *storageRepository) SearchObjects(ctx context.Context, userID, appID, searchPattern string, limit int) ([]*adapterstorage.StorageObject, error) {
	result, err := r.db.List(ctx, "storage_objects", &database.ListOptions{
		Filters: []database.Filter{
			{Field: "user_id", Operator: database.OpEqual, Value: userID},
			{Field: "app_id", Operator: database.OpEqual, Value: appID},
			{Field: "object_name", Operator: database.OpLike, Value: searchPattern},
		},
		Sort:  []database.SortField{{Field: "updated_at", Desc: true}},
		Limit: limit,
	})
	if err != nil {
		return nil, err
	}
	return recordsToObjects(result.Records), nil
}

func (r *storageRepository) CountObjectsByBucket(ctx context.Context, bucketName string) (int64, error) {
	count, err := r.db.Count(ctx, "storage_objects", []database.Filter{
		{Field: "bucket_name", Operator: database.OpEqual, Value: bucketName},
	})
	return int64(count), err
}

func (r *storageRepository) CountObjectsByUser(ctx context.Context, userID string) (int64, error) {
	count, err := r.db.Count(ctx, "storage_objects", []database.Filter{
		{Field: "user_id", Operator: database.OpEqual, Value: userID},
	})
	return int64(count), err
}

func (r *storageRepository) SumSizeByBucket(ctx context.Context, bucketName string) (int64, error) {
	sum, err := r.db.Sum(ctx, "storage_objects", "size", []database.Filter{
		{Field: "bucket_name", Operator: database.OpEqual, Value: bucketName},
	})
	return int64(sum), err
}

func (r *storageRepository) SumSizeByUser(ctx context.Context, userID string) (int64, error) {
	sum, err := r.db.Sum(ctx, "storage_objects", "size", []database.Filter{
		{Field: "user_id", Operator: database.OpEqual, Value: userID},
	})
	return int64(sum), err
}

func (r *storageRepository) SumTotalSize(ctx context.Context) (int64, error) {
	sum, err := r.db.Sum(ctx, "storage_objects", "size", nil)
	return int64(sum), err
}

// ==================== Upload token operations ====================

func (r *storageRepository) CreateUploadToken(ctx context.Context, token *UploadToken) error {
	if token.ID == "" {
		token.ID = uuid.NewString()
	}
	now := apptime.NowString()

	data := map[string]any{
		"id":          token.ID,
		"token":       token.Token,
		"bucket":      token.Bucket,
		"object_name": token.ObjectName,
		"created_at":  now,
	}
	setOptional(data, "parent_folder_id", token.ParentFolderID)
	setOptional(data, "user_id", token.UserID)
	setOptional(data, "max_size", token.MaxSize)
	setOptional(data, "content_type", token.ContentType)
	setOptional(data, "client_ip", token.ClientIP)
	if token.ExpiresAt.Valid {
		data["expires_at"] = apptime.Format(token.ExpiresAt.Time)
	}

	_, err := r.db.Create(ctx, "storage_upload_tokens", data)
	return err
}

func (r *storageRepository) GetUploadToken(ctx context.Context, id string) (*UploadToken, error) {
	rec, err := r.db.Get(ctx, "storage_upload_tokens", id)
	if err != nil {
		if err == database.ErrNotFound {
			return nil, ErrNotFound
		}
		return nil, err
	}
	return recordToUploadToken(rec), nil
}

func (r *storageRepository) GetUploadTokenByToken(ctx context.Context, token string) (*UploadToken, error) {
	rec, err := database.GetByField(ctx, r.db, "storage_upload_tokens", "token", token)
	if err != nil {
		if err == database.ErrNotFound {
			return nil, ErrNotFound
		}
		return nil, err
	}
	return recordToUploadToken(rec), nil
}

func (r *storageRepository) UpdateUploadTokenProgress(ctx context.Context, id string, bytesUploaded int64) error {
	_, err := r.db.Update(ctx, "storage_upload_tokens", id, map[string]any{
		"bytes_uploaded": bytesUploaded,
	})
	return err
}

func (r *storageRepository) CompleteUploadToken(ctx context.Context, id, objectID string) error {
	now := apptime.NowString()
	_, err := r.db.Update(ctx, "storage_upload_tokens", id, map[string]any{
		"completed":    1,
		"object_id":    objectID,
		"completed_at": now,
	})
	return err
}

func (r *storageRepository) DeleteUploadToken(ctx context.Context, id string) error {
	return r.db.Delete(ctx, "storage_upload_tokens", id)
}

func (r *storageRepository) DeleteExpiredUploadTokens(ctx context.Context) error {
	now := apptime.NowString()
	return database.DeleteByFilters(ctx, r.db, "storage_upload_tokens", []database.Filter{
		{Field: "expires_at", Operator: database.OpLessThan, Value: now},
	})
}

// ==================== Download token operations ====================

func (r *storageRepository) CreateDownloadToken(ctx context.Context, token *DownloadToken) error {
	if token.ID == "" {
		token.ID = uuid.NewString()
	}
	now := apptime.NowString()

	data := map[string]any{
		"id":          token.ID,
		"token":       token.Token,
		"file_id":     token.FileID,
		"bucket":      token.Bucket,
		"object_name": token.ObjectName,
		"created_at":  now,
	}
	setOptional(data, "parent_folder_id", token.ParentFolderID)
	setOptional(data, "user_id", token.UserID)
	setOptional(data, "file_size", token.FileSize)
	setOptional(data, "client_ip", token.ClientIP)
	if token.ExpiresAt.Valid {
		data["expires_at"] = apptime.Format(token.ExpiresAt.Time)
	}

	_, err := r.db.Create(ctx, "storage_download_tokens", data)
	return err
}

func (r *storageRepository) GetDownloadToken(ctx context.Context, id string) (*DownloadToken, error) {
	rec, err := r.db.Get(ctx, "storage_download_tokens", id)
	if err != nil {
		if err == database.ErrNotFound {
			return nil, ErrNotFound
		}
		return nil, err
	}
	return recordToDownloadToken(rec), nil
}

func (r *storageRepository) GetDownloadTokenByToken(ctx context.Context, token string) (*DownloadToken, error) {
	rec, err := database.GetByField(ctx, r.db, "storage_download_tokens", "token", token)
	if err != nil {
		if err == database.ErrNotFound {
			return nil, ErrNotFound
		}
		return nil, err
	}
	return recordToDownloadToken(rec), nil
}

func (r *storageRepository) UpdateDownloadTokenProgress(ctx context.Context, id string, bytesServed int64) error {
	_, err := r.db.Update(ctx, "storage_download_tokens", id, map[string]any{
		"bytes_served": bytesServed,
	})
	return err
}

func (r *storageRepository) CompleteDownloadToken(ctx context.Context, id string) error {
	now := apptime.NowString()
	_, err := r.db.Update(ctx, "storage_download_tokens", id, map[string]any{
		"completed":   1,
		"callback_at": now,
	})
	return err
}

func (r *storageRepository) DeleteDownloadToken(ctx context.Context, id string) error {
	return r.db.Delete(ctx, "storage_download_tokens", id)
}

func (r *storageRepository) DeleteExpiredDownloadTokens(ctx context.Context) error {
	now := apptime.NowString()
	return database.DeleteByFilters(ctx, r.db, "storage_download_tokens", []database.Filter{
		{Field: "expires_at", Operator: database.OpLessThan, Value: now},
	})
}

// ==================== Record conversion helpers ====================

func recordToBucket(rec *database.Record) *adapterstorage.StorageBucket {
	d := rec.Data
	bucket := &adapterstorage.StorageBucket{
		ID:   stringVal(d["id"]),
		Name: stringVal(d["name"]),
	}
	bucket.Public = toInt64Val(d["public"]) == 1
	if s := stringVal(d["created_at"]); s != "" {
		bucket.CreatedAt = apptime.MustParse(s)
	}
	if s := stringVal(d["updated_at"]); s != "" {
		bucket.UpdatedAt = apptime.MustParse(s)
	}
	return bucket
}

func recordToObject(rec *database.Record) *adapterstorage.StorageObject {
	d := rec.Data
	obj := &adapterstorage.StorageObject{
		ID:          stringVal(d["id"]),
		BucketName:  stringVal(d["bucket_name"]),
		ObjectName:  stringVal(d["object_name"]),
		Size:        toInt64Val(d["size"]),
		ContentType: stringVal(d["content_type"]),
		Checksum:    stringVal(d["checksum"]),
		Metadata:    stringVal(d["metadata"]),
		UserID:      stringVal(d["user_id"]),
	}

	if s := stringVal(d["parent_folder_id"]); s != "" {
		obj.ParentFolderID = &s
	}
	if s := stringVal(d["app_id"]); s != "" {
		obj.AppID = &s
	}
	if s := stringVal(d["created_at"]); s != "" {
		obj.CreatedAt = apptime.MustParse(s)
	}
	if s := stringVal(d["updated_at"]); s != "" {
		obj.UpdatedAt = apptime.MustParse(s)
	}
	if s := stringVal(d["last_viewed"]); s != "" {
		t := apptime.MustParse(s)
		obj.LastViewed = apptime.NewNullTime(t)
	}
	return obj
}

func recordsToObjects(records []*database.Record) []*adapterstorage.StorageObject {
	objects := make([]*adapterstorage.StorageObject, len(records))
	for i, rec := range records {
		objects[i] = recordToObject(rec)
	}
	return objects
}

func recordToUploadToken(rec *database.Record) *UploadToken {
	d := rec.Data
	token := &UploadToken{
		ID:         stringVal(d["id"]),
		Token:      stringVal(d["token"]),
		Bucket:     stringVal(d["bucket"]),
		ObjectName: stringVal(d["object_name"]),
	}

	if s := stringVal(d["parent_folder_id"]); s != "" {
		token.ParentFolderID = &s
	}
	if s := stringVal(d["user_id"]); s != "" {
		token.UserID = &s
	}
	if v := toInt64Val(d["max_size"]); v != 0 {
		token.MaxSize = &v
	}
	if s := stringVal(d["content_type"]); s != "" {
		token.ContentType = &s
	}
	if v := toInt64Val(d["bytes_uploaded"]); v != 0 {
		token.BytesUploaded = &v
	}
	token.Completed = toInt64Val(d["completed"]) == 1
	if s := stringVal(d["object_id"]); s != "" {
		token.ObjectID = &s
	}
	if s := stringVal(d["expires_at"]); s != "" {
		t := apptime.MustParse(s)
		token.ExpiresAt = apptime.NewNullTime(t)
	}
	if s := stringVal(d["created_at"]); s != "" {
		token.CreatedAt = apptime.MustParse(s)
	}
	if s := stringVal(d["completed_at"]); s != "" {
		t := apptime.MustParse(s)
		token.CompletedAt = apptime.NewNullTime(t)
	}
	if s := stringVal(d["client_ip"]); s != "" {
		token.ClientIP = &s
	}
	return token
}

func recordToDownloadToken(rec *database.Record) *DownloadToken {
	d := rec.Data
	token := &DownloadToken{
		ID:         stringVal(d["id"]),
		Token:      stringVal(d["token"]),
		FileID:     stringVal(d["file_id"]),
		Bucket:     stringVal(d["bucket"]),
		ObjectName: stringVal(d["object_name"]),
	}

	if s := stringVal(d["parent_folder_id"]); s != "" {
		token.ParentFolderID = &s
	}
	if s := stringVal(d["user_id"]); s != "" {
		token.UserID = &s
	}
	if v := toInt64Val(d["file_size"]); v != 0 {
		token.FileSize = &v
	}
	if v := toInt64Val(d["bytes_served"]); v != 0 {
		token.BytesServed = &v
	}
	token.Completed = toInt64Val(d["completed"]) == 1
	if s := stringVal(d["expires_at"]); s != "" {
		t := apptime.MustParse(s)
		token.ExpiresAt = apptime.NewNullTime(t)
	}
	if s := stringVal(d["created_at"]); s != "" {
		token.CreatedAt = apptime.MustParse(s)
	}
	if s := stringVal(d["callback_at"]); s != "" {
		t := apptime.MustParse(s)
		token.CallbackAt = apptime.NewNullTime(t)
	}
	if s := stringVal(d["client_ip"]); s != "" {
		token.ClientIP = &s
	}
	return token
}
