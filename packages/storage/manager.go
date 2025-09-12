package storage

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"time"

	"github.com/google/uuid"
	"github.com/suppers-ai/logger"
	"gorm.io/gorm"
)

// Manager handles storage operations with database tracking
type Manager struct {
	provider Provider
	db       *gorm.DB
	logger   logger.Logger
	config   Config
}

// NewManager creates a new storage manager
func NewManager(cfg Config, db *gorm.DB, logger logger.Logger) (*Manager, error) {
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
		Name:   name,
		Public: public,
	}

	if err := m.db.WithContext(ctx).Create(bucket).Error; err != nil {
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
	err := m.db.WithContext(ctx).Where("name = ?", name).First(&bucket).Error
	if err != nil {
		if err == gorm.ErrRecordNotFound {
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

	// Delete from database (cascade will delete objects)
	if err := m.db.WithContext(ctx).Delete(&bucket).Error; err != nil {
		return fmt.Errorf("failed to delete bucket from database: %w", err)
	}

	m.logger.Info(ctx, "Bucket deleted",
		logger.String("bucket", name))

	return nil
}

// GetBucket retrieves a bucket by name
func (m *Manager) GetBucket(ctx context.Context, name string) (*StorageBucket, error) {
	var bucket StorageBucket
	err := m.db.WithContext(ctx).Where("name = ?", name).First(&bucket).Error
	if err != nil {
		if err == gorm.ErrRecordNotFound {
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
		var bucket StorageBucket
		err := m.db.WithContext(ctx).Where("name = ?", pb.Name).First(&bucket).Error
		if err == gorm.ErrRecordNotFound {
			// Create missing bucket in database
			bucket = StorageBucket{
				Name:   pb.Name,
				Public: pb.Public,
			}
			m.db.WithContext(ctx).Create(&bucket)
		}
	}

	return providerBuckets, nil
}

// listBucketsFromDB lists buckets from database as fallback
func (m *Manager) listBucketsFromDB(ctx context.Context) ([]BucketInfo, error) {
	var buckets []StorageBucket
	err := m.db.WithContext(ctx).Order("name").Find(&buckets).Error
	if err != nil {
		return nil, fmt.Errorf("failed to list buckets: %w", err)
	}

	bucketInfos := make([]BucketInfo, len(buckets))
	for i, bucket := range buckets {
		// Get file count and total size for this bucket
		var fileCount int64
		var totalSize int64

		m.db.WithContext(ctx).Model(&StorageObject{}).
			Where("bucket_name = ?", bucket.Name).
			Count(&fileCount)

		m.db.WithContext(ctx).Model(&StorageObject{}).
			Where("bucket_name = ?", bucket.Name).
			Select("COALESCE(SUM(size), 0)").
			Scan(&totalSize)

		bucketInfos[i] = BucketInfo{
			Name:        bucket.Name,
			Public:      bucket.Public,
			ObjectCount: fileCount,
			TotalSize:   totalSize,
			CreatedAt:   bucket.CreatedAt,
		}
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
		CreatedAt:      time.Now(),
		UpdatedAt:      time.Now(),
	}

	if err := m.db.WithContext(ctx).Create(object).Error; err != nil {
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
	// Get object from database by ID
	var object StorageObject
	err := m.db.WithContext(ctx).
		Where("id = ?", objectID).
		First(&object).Error

	if err != nil {
		if err == gorm.ErrRecordNotFound {
			return nil, fmt.Errorf("object not found")
		}
		return nil, fmt.Errorf("failed to get object: %w", err)
	}

	return &object, nil
}

// DeleteObject deletes an object by its unique ID
func (m *Manager) DeleteObject(ctx context.Context, objectID string) error {
	// Get object from database first
	var object StorageObject
	err := m.db.WithContext(ctx).Where("id = ?", objectID).First(&object).Error
	if err != nil {
		if err == gorm.ErrRecordNotFound {
			return fmt.Errorf("object not found")
		}
		return fmt.Errorf("failed to get object: %w", err)
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
	result := m.db.WithContext(ctx).Delete(&object)
	if result.Error != nil {
		return fmt.Errorf("failed to delete object from database: %w", result.Error)
	}

	m.logger.Info(ctx, "Object deleted",
		logger.String("object_id", objectID),
		logger.String("bucket", object.BucketName),
		logger.String("name", object.ObjectName))

	return nil
}

// ListObjects lists objects in a bucket, optionally filtered by parent folder
func (m *Manager) ListObjects(ctx context.Context, bucketName string, parentFolderID *string, limit int) ([]StorageObject, error) {
	// Build query
	query := m.db.WithContext(ctx).Where("bucket_name = ?", bucketName)

	// Filter by parent folder
	if parentFolderID != nil {
		query = query.Where("parent_folder_id = ?", *parentFolderID)
	} else {
		// Get root items (no parent folder)
		query = query.Where("parent_folder_id IS NULL")
	}

	if limit > 0 {
		query = query.Limit(limit)
	}

	var objects []StorageObject
	err := query.Order("object_name").Find(&objects).Error
	if err != nil {
		return nil, fmt.Errorf("failed to list objects: %w", err)
	}

	return objects, nil
}

// GenerateSignedURL generates a signed URL for temporary access by object ID
func (m *Manager) GenerateSignedURL(ctx context.Context, objectID string, expiresIn time.Duration) (string, error) {
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
	obj.Size = int64(len(content))
	obj.UpdatedAt = time.Now()

	return m.db.WithContext(ctx).Save(obj).Error
}
