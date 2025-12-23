package storage

import (
	"bytes"
	"context"
	"database/sql"
	"fmt"
	"io"

	"github.com/google/uuid"
	"github.com/suppers-ai/solobase/internal/pkg/logger"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

// Manager handles storage operations with database tracking
type Manager struct {
	provider Provider
	db       *sql.DB
	logger   logger.Logger
	config   Config
}

// NewManager creates a new storage manager
func NewManager(cfg Config, db *sql.DB, logger logger.Logger) (*Manager, error) {
	provider, err := NewProvider(cfg)
	if err != nil {
		return nil, fmt.Errorf("failed to create storage provider: %w", err)
	}

	return &Manager{
		provider: provider,
		db:       db,
		logger:   logger,
		config:   cfg,
	}, nil
}

// GetProvider returns the underlying storage provider
func (m *Manager) GetProvider() Provider {
	return m.provider
}

// CreateBucket creates a new bucket in both storage and database
func (m *Manager) CreateBucket(ctx context.Context, name string, public bool) (*StorageBucket, error) {
	// Create in storage provider first
	err := m.provider.CreateBucket(ctx, name, CreateBucketOptions{
		Public: public,
		Region: m.config.S3Region,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to create bucket in storage: %w", err)
	}

	// Create in database
	bucket := &StorageBucket{
		Name:      name,
		Public:    public,
		CreatedAt: apptime.NowTime(),
	}

	_, err = m.db.ExecContext(ctx,
		"INSERT INTO storage_buckets (name, public, created_at) VALUES (?, ?, ?)",
		bucket.Name, bucket.Public, bucket.CreatedAt)
	if err != nil {
		// Try to clean up the storage bucket
		m.provider.DeleteBucket(ctx, name)
		return nil, fmt.Errorf("failed to create bucket in database: %w", err)
	}

	m.logger.Info(ctx, "Bucket created",
		logger.String("bucket", name),
		logger.Bool("public", public),
		logger.String("provider", string(m.provider.Type())))

	return bucket, nil
}

// DeleteBucket deletes a bucket from both storage and database
func (m *Manager) DeleteBucket(ctx context.Context, name string) error {
	// Get bucket from database
	var bucket StorageBucket
	err := m.db.QueryRowContext(ctx,
		"SELECT name, public, created_at FROM storage_buckets WHERE name = ?", name).
		Scan(&bucket.Name, &bucket.Public, &bucket.CreatedAt)
	if err != nil {
		if err == sql.ErrNoRows {
			return fmt.Errorf("bucket not found")
		}
		return fmt.Errorf("failed to get bucket: %w", err)
	}

	// Delete from storage provider
	err = m.provider.DeleteBucket(ctx, name)
	if err != nil {
		m.logger.Error(ctx, "Failed to delete bucket from storage",
			logger.String("bucket", name),
			logger.Err(err))
		// Continue to delete from database anyway
	}

	// Delete objects first (cascade)
	_, _ = m.db.ExecContext(ctx, "DELETE FROM storage_objects WHERE bucket_name = ?", name)

	// Delete from database
	_, err = m.db.ExecContext(ctx, "DELETE FROM storage_buckets WHERE name = ?", name)
	if err != nil {
		return fmt.Errorf("failed to delete bucket from database: %w", err)
	}

	m.logger.Info(ctx, "Bucket deleted",
		logger.String("bucket", name))

	return nil
}

// GetBucket retrieves a bucket by name
func (m *Manager) GetBucket(ctx context.Context, name string) (*StorageBucket, error) {
	var bucket StorageBucket
	err := m.db.QueryRowContext(ctx,
		"SELECT name, public, created_at FROM storage_buckets WHERE name = ?", name).
		Scan(&bucket.Name, &bucket.Public, &bucket.CreatedAt)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, fmt.Errorf("bucket not found")
		}
		return nil, fmt.Errorf("failed to get bucket: %w", err)
	}
	return &bucket, nil
}

// ListBuckets lists all buckets with stats
func (m *Manager) ListBuckets(ctx context.Context) ([]BucketInfo, error) {
	// Get buckets from provider
	providerBuckets, err := m.provider.ListBuckets(ctx)
	if err != nil {
		m.logger.Error(ctx, "Failed to list buckets from provider", logger.Err(err))
		// Fall back to database
		return m.listBucketsFromDB(ctx)
	}

	// Sync with database
	for _, pb := range providerBuckets {
		var exists int
		err := m.db.QueryRowContext(ctx,
			"SELECT COUNT(*) FROM storage_buckets WHERE name = ?", pb.Name).Scan(&exists)
		if err == nil && exists == 0 {
			// Create missing bucket in database
			_, _ = m.db.ExecContext(ctx,
				"INSERT INTO storage_buckets (name, public, created_at) VALUES (?, ?, ?)",
				pb.Name, pb.Public, apptime.NowTime())
		}
	}

	return providerBuckets, nil
}

// listBucketsFromDB lists buckets from database as fallback
func (m *Manager) listBucketsFromDB(ctx context.Context) ([]BucketInfo, error) {
	rows, err := m.db.QueryContext(ctx,
		"SELECT name, public, created_at FROM storage_buckets ORDER BY name")
	if err != nil {
		return nil, fmt.Errorf("failed to list buckets: %w", err)
	}
	defer rows.Close()

	var bucketInfos []BucketInfo
	for rows.Next() {
		var bucket StorageBucket
		if err := rows.Scan(&bucket.Name, &bucket.Public, &bucket.CreatedAt); err != nil {
			continue
		}

		// Get file count and total size for this bucket
		var fileCount int64
		var totalSize int64

		m.db.QueryRowContext(ctx,
			"SELECT COUNT(*) FROM storage_objects WHERE bucket_name = ?", bucket.Name).
			Scan(&fileCount)

		m.db.QueryRowContext(ctx,
			"SELECT COALESCE(SUM(size), 0) FROM storage_objects WHERE bucket_name = ?", bucket.Name).
			Scan(&totalSize)

		bucketInfos = append(bucketInfos, BucketInfo{
			Name:        bucket.Name,
			Public:      bucket.Public,
			ObjectCount: fileCount,
			TotalSize:   totalSize,
			CreatedAt:   bucket.CreatedAt,
		})
	}

	return bucketInfos, nil
}

// UploadObject uploads an object to storage and tracks it in database
func (m *Manager) UploadObject(ctx context.Context, bucketName, filename string, parentFolderID *string, content []byte, mimeType string, userID *uuid.UUID, appID *string) (*StorageObject, error) {
	// Get bucket from database
	bucket, err := m.GetBucket(ctx, bucketName)
	if err != nil {
		return nil, err
	}

	// Generate unique ID for the object
	objectID := uuid.New().String()

	// Build storage key (objectID/filename)
	storageKey := fmt.Sprintf("%s/%s", objectID, filename)

	// Upload to storage provider
	reader := bytes.NewReader(content)
	err = m.provider.PutObject(ctx, bucketName, storageKey, reader, int64(len(content)), PutObjectOptions{
		ContentType: mimeType,
		Public:      bucket.Public,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to upload to storage: %w", err)
	}

	// Convert userID to string if provided
	var userIDStr string
	if userID != nil {
		userIDStr = userID.String()
	}

	now := apptime.NowTime()
	// Create new object in database
	object := &StorageObject{
		ID:             objectID,
		BucketName:     bucket.Name,
		ObjectName:     filename,
		ParentFolderID: parentFolderID,
		Size:           int64(len(content)),
		ContentType:    mimeType,
		UserID:         userIDStr,
		AppID:          appID,
		CreatedAt:      now,
		UpdatedAt:      now,
	}

	_, err = m.db.ExecContext(ctx,
		`INSERT INTO storage_objects (id, bucket_name, object_name, parent_folder_id, size, content_type, user_id, app_id, created_at, updated_at)
		 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		object.ID, object.BucketName, object.ObjectName, object.ParentFolderID,
		object.Size, object.ContentType, object.UserID, object.AppID,
		object.CreatedAt, object.UpdatedAt)
	if err != nil {
		// Try to clean up from storage
		m.provider.DeleteObject(ctx, bucketName, storageKey)
		return nil, fmt.Errorf("failed to create object in database: %w", err)
	}

	m.logger.Info(ctx, "Object uploaded",
		logger.String("object_id", objectID),
		logger.String("bucket", bucketName),
		logger.String("filename", filename),
		logger.Int64("size", object.Size))

	return object, nil
}

// GetObject retrieves an object by its unique ID
func (m *Manager) GetObject(ctx context.Context, objectID string) (*StorageObject, error) {
	var object StorageObject
	var parentFolderID sql.NullString
	var appID sql.NullString

	err := m.db.QueryRowContext(ctx,
		`SELECT id, bucket_name, object_name, parent_folder_id, size, content_type, user_id, app_id, created_at, updated_at
		 FROM storage_objects WHERE id = ?`, objectID).
		Scan(&object.ID, &object.BucketName, &object.ObjectName, &parentFolderID,
			&object.Size, &object.ContentType, &object.UserID, &appID,
			&object.CreatedAt, &object.UpdatedAt)

	if err != nil {
		if err == sql.ErrNoRows {
			return nil, fmt.Errorf("object not found")
		}
		return nil, fmt.Errorf("failed to get object: %w", err)
	}

	if parentFolderID.Valid {
		object.ParentFolderID = &parentFolderID.String
	}
	if appID.Valid {
		object.AppID = &appID.String
	}

	return &object, nil
}

// DeleteObject deletes an object by its unique ID
func (m *Manager) DeleteObject(ctx context.Context, objectID string) error {
	// Get object from database first
	object, err := m.GetObject(ctx, objectID)
	if err != nil {
		return err
	}

	// Build storage key for the provider
	storageKey := fmt.Sprintf("%s/%s", object.ID, object.ObjectName)

	// Delete from storage provider
	err = m.provider.DeleteObject(ctx, object.BucketName, storageKey)
	if err != nil {
		m.logger.Error(ctx, "Failed to delete object from storage",
			logger.String("bucket", object.BucketName),
			logger.String("key", storageKey),
			logger.Err(err))
		// Continue to delete from database anyway
	}

	// Delete from database
	_, err = m.db.ExecContext(ctx, "DELETE FROM storage_objects WHERE id = ?", objectID)
	if err != nil {
		return fmt.Errorf("failed to delete object from database: %w", err)
	}

	m.logger.Info(ctx, "Object deleted",
		logger.String("object_id", objectID),
		logger.String("bucket", object.BucketName),
		logger.String("name", object.ObjectName))

	return nil
}

// ListObjects lists objects in a bucket, optionally filtered by parent folder
func (m *Manager) ListObjects(ctx context.Context, bucketName string, parentFolderID *string, limit int) ([]StorageObject, error) {
	var query string
	var args []interface{}

	if parentFolderID != nil {
		query = `SELECT id, bucket_name, object_name, parent_folder_id, size, content_type, user_id, app_id, created_at, updated_at
				 FROM storage_objects WHERE bucket_name = ? AND parent_folder_id = ? ORDER BY object_name`
		args = []interface{}{bucketName, *parentFolderID}
	} else {
		query = `SELECT id, bucket_name, object_name, parent_folder_id, size, content_type, user_id, app_id, created_at, updated_at
				 FROM storage_objects WHERE bucket_name = ? AND parent_folder_id IS NULL ORDER BY object_name`
		args = []interface{}{bucketName}
	}

	if limit > 0 {
		query += " LIMIT ?"
		args = append(args, limit)
	}

	rows, err := m.db.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, fmt.Errorf("failed to list objects: %w", err)
	}
	defer rows.Close()

	var objects []StorageObject
	for rows.Next() {
		var obj StorageObject
		var parentFolder sql.NullString
		var appID sql.NullString

		if err := rows.Scan(&obj.ID, &obj.BucketName, &obj.ObjectName, &parentFolder,
			&obj.Size, &obj.ContentType, &obj.UserID, &appID,
			&obj.CreatedAt, &obj.UpdatedAt); err != nil {
			continue
		}

		if parentFolder.Valid {
			obj.ParentFolderID = &parentFolder.String
		}
		if appID.Valid {
			obj.AppID = &appID.String
		}

		objects = append(objects, obj)
	}

	return objects, nil
}

// GenerateSignedURL generates a signed URL for temporary access by object ID
func (m *Manager) GenerateSignedURL(ctx context.Context, objectID string, expiresIn apptime.Duration) (string, error) {
	// Get object by ID
	obj, err := m.GetObject(ctx, objectID)
	if err != nil {
		return "", err
	}

	// Build storage key
	storageKey := fmt.Sprintf("%s/%s", obj.ID, obj.ObjectName)

	// Generate presigned URL from provider
	return m.provider.GeneratePresignedURL(ctx, obj.BucketName, storageKey, expiresIn)
}

// GetFile retrieves a file's content and type by object ID
func (m *Manager) GetFile(ctx context.Context, objectID string) ([]byte, string, error) {
	// Get object by ID
	obj, err := m.GetObject(ctx, objectID)
	if err != nil {
		return nil, "", err
	}

	// Build storage key
	storageKey := fmt.Sprintf("%s/%s", obj.ID, obj.ObjectName)

	// Fetch content from provider
	reader, err := m.provider.GetObject(ctx, obj.BucketName, storageKey)
	if err != nil {
		return nil, "", fmt.Errorf("failed to get object from storage: %w", err)
	}
	defer reader.Close()

	content, err := io.ReadAll(reader)
	if err != nil {
		return nil, "", fmt.Errorf("failed to read object content: %w", err)
	}

	return content, obj.ContentType, nil
}

// UpdateFile updates a file's content by object ID
func (m *Manager) UpdateFile(ctx context.Context, objectID string, content []byte) error {
	// Get existing object
	obj, err := m.GetObject(ctx, objectID)
	if err != nil {
		return err
	}

	// Build storage key
	storageKey := fmt.Sprintf("%s/%s", obj.ID, obj.ObjectName)

	// Upload new content to provider
	reader := bytes.NewReader(content)
	err = m.provider.PutObject(ctx, obj.BucketName, storageKey, reader, int64(len(content)), PutObjectOptions{
		ContentType: obj.ContentType,
	})
	if err != nil {
		return fmt.Errorf("failed to update object in storage: %w", err)
	}

	// Update database
	now := apptime.NowTime()
	_, err = m.db.ExecContext(ctx,
		"UPDATE storage_objects SET size = ?, updated_at = ? WHERE id = ?",
		len(content), now, objectID)

	return err
}
