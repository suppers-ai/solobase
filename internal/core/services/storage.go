package services

import (
	"bytes"
	"context"
	"crypto/md5"
	"encoding/hex"
	"fmt"
	"io"
	"log"
	"os"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/suppers-ai/solobase/internal/config"
	"github.com/suppers-ai/solobase/internal/pkg/database"
	"github.com/suppers-ai/solobase/internal/pkg/storage"
)

// EnhancedStorageService is an alias for StorageService
type EnhancedStorageService = StorageService

// StorageOptions contains optional configuration for StorageService
type StorageOptions struct {
	AppID string // Application ID for storage isolation (defaults to "solobase")
}

type StorageService struct {
	config   config.StorageConfig
	provider storage.Provider
	db       *database.DB
	appID    string // Application ID for storage isolation
	ctx      context.Context
}

func NewStorageService(db *database.DB, cfg config.StorageConfig) *StorageService {
	// Default to "solobase" app ID
	return NewStorageServiceWithOptions(db, cfg, &StorageOptions{
		AppID: "solobase",
	})
}

// NewStorageServiceWithOptions creates a new storage service with custom options
func NewStorageServiceWithOptions(db *database.DB, cfg config.StorageConfig, opts *StorageOptions) *StorageService {
	var provider storage.Provider
	var err error
	ctx := context.Background()

	// Default options
	if opts == nil {
		opts = &StorageOptions{}
	}
	if opts.AppID == "" {
		opts.AppID = "solobase"
	}

	// Update path to use new structure
	localPath := cfg.LocalStoragePath
	if localPath == "" || localPath == "./data/storage" || localPath == "./.data/storage" || localPath == "./.data/storage/int" {
		localPath = "./.data/storage" // Base storage path - buckets will be subdirectories
	}

	// Ensure storage directory exists for local storage
	if cfg.Type != "s3" {
		if err := os.MkdirAll(localPath, 0755); err != nil {
			log.Printf("Failed to create storage directory %s: %v", localPath, err)
		}
	}

	switch cfg.Type {
	case "s3":
		storageConfig := storage.Config{
			Provider:          storage.ProviderS3,
			S3Endpoint:        cfg.S3Endpoint,
			S3AccessKeyID:     cfg.S3AccessKey,
			S3SecretAccessKey: cfg.S3SecretKey,
			S3Region:          cfg.S3Region,
			S3UseSSL:          cfg.S3UseSSL,
		}
		provider, err = storage.NewProvider(storageConfig)
		if err != nil {
			log.Printf("Failed to initialize S3 storage: %v, falling back to local", err)
			localConfig := storage.Config{
				Provider: storage.ProviderLocal,
				BasePath: localPath,
			}
			provider, _ = storage.NewProvider(localConfig)
		}
	default:
		localConfig := storage.Config{
			Provider: storage.ProviderLocal,
			BasePath: localPath,
		}
		provider, err = storage.NewProvider(localConfig)
		if err != nil {
			log.Printf("Failed to initialize local storage: %v", err)
		}
	}

	service := &StorageService{
		config:   cfg,
		provider: provider,
		db:       db,
		appID:    opts.AppID,
		ctx:      ctx,
	}

	// Initialize default buckets
	service.initializeDefaultBuckets()

	return service
}

// initializeDefaultBuckets creates default buckets if they don't exist
func (s *StorageService) initializeDefaultBuckets() {
	defaultBuckets := []struct {
		name   string
		public bool
	}{
		{"int_storage", false}, // Internal storage for user/app data
		{"ext_storage", false}, // External storage for extensions
		{"public", true},       // Public files
	}

	for _, bucket := range defaultBuckets {
		// Check if bucket already exists in database
		var existingBucket storage.StorageBucket
		if err := s.db.Where("name = ?", bucket.name).First(&existingBucket).Error; err != nil {
			// Bucket doesn't exist, create it using CreateBucket method which saves to DB
			if err := s.CreateBucket(bucket.name, bucket.public); err != nil {
				// Only log if it's not an "already exists" error
				if !strings.Contains(err.Error(), "exists") && !strings.Contains(err.Error(), "exist") {
					log.Printf("Failed to create default bucket %s: %v", bucket.name, err)
				}
			} else {
				log.Printf("Created default bucket: %s", bucket.name)
			}
		}
	}
}

// GetProviderType returns the type of storage provider being used
func (s *StorageService) GetProviderType() string {
	return s.config.Type
}

// GetAppID returns the application ID for storage isolation
func (s *StorageService) GetAppID() string {
	return s.appID
}

// GetObjectInfo retrieves information about an object
func (s *StorageService) GetObjectInfo(bucket, objectID string) (*storage.StorageObject, error) {
	var object storage.StorageObject
	log.Printf("GetObjectInfo: Looking for object with id=%s in bucket=%s", objectID, bucket)
	if err := s.db.Where("id = ? AND bucket_name = ?", objectID, bucket).First(&object).Error; err != nil {
		log.Printf("GetObjectInfo: Failed to find object: %v", err)
		return nil, err
	}
	log.Printf("GetObjectInfo: Found object with id=%s, name=%s", object.ID, object.ObjectName)
	return &object, nil
}

// GetObjectByKey retrieves an object by its storage key
func (s *StorageService) GetObjectByKey(bucket, key string) (io.ReadCloser, string, string, error) {
	if s.provider == nil {
		return nil, "", "", fmt.Errorf("storage not initialized")
	}

	// Get object metadata from database
	var object storage.StorageObject
	if err := s.db.Where("key = ? AND bucket = ?", key, bucket).First(&object).Error; err != nil {
		return nil, "", "", err
	}

	// Get the actual file from storage
	reader, err := s.provider.GetObject(s.ctx, bucket, key)
	if err != nil {
		return nil, "", "", err
	}

	// Use the object name directly
	filename := object.ObjectName

	return reader, filename, object.ContentType, nil
}

// GeneratePresignedDownloadURL generates a presigned URL for downloading (S3 only)
func (s *StorageService) GeneratePresignedDownloadURL(bucket, key string, expiry int) (string, error) {
	if s.config.Type != "s3" {
		return "", fmt.Errorf("presigned URLs are only supported for S3 storage")
	}

	// Use the provider's GeneratePresignedURL method
	return s.provider.GeneratePresignedURL(s.ctx, bucket, key, time.Duration(expiry)*time.Second)
}

// GeneratePresignedUploadURL generates a presigned URL for uploading (S3 only)
func (s *StorageService) GeneratePresignedUploadURL(bucket, key, contentType string, expiry int) (string, error) {
	if s.config.Type != "s3" {
		return "", fmt.Errorf("presigned URLs are only supported for S3 storage")
	}

	// For now, we'll use the same method as download
	// In a full implementation, we'd need to extend the storage package to support upload URLs
	return s.provider.GeneratePresignedURL(s.ctx, bucket, key, time.Duration(expiry)*time.Second)
}

func (s *StorageService) CreateBucket(name string, public bool) error {
	if s.provider == nil {
		return fmt.Errorf("storage not initialized")
	}

	// Check if bucket already exists in database
	var existingBucket storage.StorageBucket
	if err := s.db.Where("name = ?", name).First(&existingBucket).Error; err == nil {
		// Bucket already exists in database
		return fmt.Errorf("bucket %s already exists", name)
	}

	// Create bucket in storage provider
	err := s.provider.CreateBucket(s.ctx, name, storage.CreateBucketOptions{Public: public})
	if err != nil {
		// If bucket already exists on disk but not in DB, that's ok - we'll add it to DB
		if !strings.Contains(err.Error(), "exists") && !strings.Contains(err.Error(), "exist") {
			return err
		}
	}

	// Save bucket to database
	bucket := &storage.StorageBucket{
		ID:        uuid.New().String(),
		Name:      name,
		Public:    public,
		CreatedAt: time.Now(),
		UpdatedAt: time.Now(),
	}

	if err := s.db.Create(bucket).Error; err != nil {
		// If we just created the bucket on disk, try to rollback
		if err == nil {
			s.provider.DeleteBucket(s.ctx, name)
		}
		return err
	}

	return nil
}

func (s *StorageService) DeleteBucket(name string) error {
	if s.provider == nil {
		return fmt.Errorf("storage not initialized")
	}

	// Delete from storage provider
	err := s.provider.DeleteBucket(s.ctx, name)
	if err != nil {
		return err
	}

	// Delete bucket and all objects from database
	if err := s.db.Where("bucket_name = ?", name).Delete(&storage.StorageObject{}).Error; err != nil {
		return err
	}

	if err := s.db.Where("name = ?", name).Delete(&storage.StorageBucket{}).Error; err != nil {
		return err
	}

	return nil
}

func (s *StorageService) GetBuckets() ([]interface{}, error) {
	if s.provider == nil {
		return []interface{}{}, nil
	}

	// Get buckets from database
	var buckets []storage.StorageBucket
	if err := s.db.Find(&buckets).Error; err != nil {
		return nil, err
	}

	// Convert to interface slice with stats
	result := make([]interface{}, len(buckets))
	for i, bucket := range buckets {
		// Get object count and size for this bucket
		var count int64
		var totalSize int64

		s.db.Model(&storage.StorageObject{}).
			Where("bucket_name = ?", bucket.Name).
			Count(&count)

		s.db.Model(&storage.StorageObject{}).
			Where("bucket_name = ?", bucket.Name).
			Select("COALESCE(SUM(size), 0)").
			Scan(&totalSize)

		result[i] = map[string]interface{}{
			"id":         bucket.ID,
			"name":       bucket.Name,
			"public":     bucket.Public,
			"created_at": bucket.CreatedAt.Format("2006-01-02"),
			"files":      count,
			"size":       formatBytes(totalSize),
			"size_bytes": totalSize,
		}
	}

	return result, nil
}

// GetObjects returns objects in a bucket filtered by userID, appID, and parentFolderID
func (s *StorageService) GetObjects(bucket string, userID string, parentFolderID *string) ([]interface{}, error) {
	if s.provider == nil {
		return []interface{}{}, nil
	}

	log.Printf("GetObjects: bucket=%s, userID=%s, parentFolderID=%v, appID=%s", bucket, userID, parentFolderID, s.appID)

	// Build query for objects
	query := s.db.Where("bucket_name = ?", bucket)
	
	// Only filter by user_id if provided and not empty
	if userID != "" {
		query = query.Where("user_id = ?", userID)
	}

	// Filter by app ID
	if s.appID != "" {
		query = query.Where("app_id = ?", s.appID)
	} else {
		query = query.Where("app_id IS NULL")
	}

	// Filter by parent folder
	if parentFolderID != nil {
		query = query.Where("parent_folder_id = ?", *parentFolderID)
	} else {
		query = query.Where("parent_folder_id IS NULL")
	}

	var objects []storage.StorageObject

	if err := query.Find(&objects).Error; err != nil {
		return nil, err
	}

	// Return raw StorageObject data without transformation
	result := make([]interface{}, 0, len(objects))

	for i := range objects {
		obj := &objects[i]

		// Skip empty names and .keep files
		if obj.ObjectName == "" || obj.ObjectName == ".keep" {
			continue
		}

		// Determine the type based on content type
		fileType := "file"
		if obj.ContentType == "application/x-directory" {
			fileType = "folder"
		} else {
			fileType = getFileType(obj.ObjectName)
		}

		// Return the raw StorageObject fields
		result = append(result, map[string]interface{}{
			"id":               obj.ID,
			"bucket_name":      obj.BucketName,
			"object_name":      obj.ObjectName,
			"parent_folder_id": obj.ParentFolderID,
			"size":             obj.Size,
			"content_type":     obj.ContentType,
			"type":             fileType,  // Add type field for frontend
			"checksum":         obj.Checksum,
			"metadata":         obj.Metadata,
			"created_at":       obj.CreatedAt,
			"updated_at":       obj.UpdatedAt,
			"last_viewed":      obj.LastViewed,
			"user_id":          obj.UserID,
			"app_id":           obj.AppID,
		})
	}

	log.Printf("Returning %d items", len(result))
	return result, nil
}

func (s *StorageService) UploadFile(bucket, filename, userID string, reader io.Reader, size int64, mimeType string, parentFolderID *string) (interface{}, error) {
	if s.provider == nil {
		return nil, fmt.Errorf("storage not initialized")
	}

	// Read the content to calculate checksum
	var buf bytes.Buffer
	tee := io.TeeReader(reader, &buf)

	// Calculate MD5 checksum
	hash := md5.New()
	if _, err := io.Copy(hash, tee); err != nil {
		return nil, fmt.Errorf("failed to calculate checksum: %v", err)
	}
	checksum := hex.EncodeToString(hash.Sum(nil))

	// Generate a unique ID for this object
	objectID := uuid.New().String()

	// Storage key is simply bucket/objectID/filename
	// This keeps files organized and avoids collisions without complex paths
	storageKey := fmt.Sprintf("%s/%s", objectID, filename)

	// Upload to storage provider
	err := s.provider.PutObject(s.ctx, bucket, storageKey, &buf, size, storage.PutObjectOptions{
		ContentType: mimeType,
	})
	if err != nil {
		return nil, err
	}

	// Get app ID as pointer
	var appIDPtr *string
	if s.appID != "" {
		appIDPtr = &s.appID
	}

	// Save to database with simplified structure
	storageObj := &storage.StorageObject{
		ID:             objectID,
		BucketName:     bucket,
		ObjectName:     filename,
		ParentFolderID: parentFolderID,
		Size:           size,
		ContentType:    mimeType,
		Checksum:       checksum,
		UserID:         userID,
		AppID:          appIDPtr,
		CreatedAt:      time.Now(),
		UpdatedAt:      time.Now(),
	}

	if err := s.db.Create(storageObj).Error; err != nil {
		// Try to rollback storage upload
		s.provider.DeleteObject(s.ctx, bucket, storageKey)
		return nil, err
	}

	return map[string]interface{}{
		"id":               storageObj.ID,
		"size":             size,
		"content_type":     mimeType,
		"checksum":         checksum,
		"parent_folder_id": parentFolderID,
		"app_id":           appIDPtr,
		"url":              s.getPublicURL(bucket, storageKey),
	}, nil
}

// UploadFileWithParent uploads a file with a specific parent folder ID
func (s *StorageService) UploadFileWithParent(bucket, filename, parentFolderID, userID string, reader io.Reader, size int64, mimeType string) (interface{}, error) {
	// Simply delegate to UploadFile with the parent folder ID
	var parentFolderPtr *string
	if parentFolderID != "" {
		parentFolderPtr = &parentFolderID
	}
	return s.UploadFile(bucket, filename, userID, reader, size, mimeType, parentFolderPtr)
}

func (s *StorageService) UploadFileBytes(bucket, filename, userID string, content []byte, mimeType string) (interface{}, error) {
	reader := bytes.NewReader(content)
	return s.UploadFile(bucket, filename, userID, reader, int64(len(content)), mimeType, nil)
}

// getStorageKey builds the storage key for an object
// Storage key is simply objectID/filename for simplicity
func (s *StorageService) getStorageKey(obj *storage.StorageObject) string {
	return fmt.Sprintf("%s/%s", obj.ID, obj.ObjectName)
}

func (s *StorageService) GetObject(bucket, objectID string) (io.ReadCloser, string, string, error) {
	if s.provider == nil {
		return nil, "", "", fmt.Errorf("storage not initialized")
	}

	// Get object from database to get the actual key and metadata
	var obj storage.StorageObject
	if err := s.db.Where("id = ? AND bucket_name = ?", objectID, bucket).First(&obj).Error; err != nil {
		return nil, "", "", fmt.Errorf("object not found")
	}

	// Build the storage key using the simple ID-based approach
	storageKey := s.getStorageKey(&obj)

	// Get the object from storage
	reader, err := s.provider.GetObject(s.ctx, bucket, storageKey)
	if err != nil {
		return nil, "", "", err
	}

	// Use ObjectName as the filename
	filename := obj.ObjectName

	return reader, filename, obj.ContentType, nil
}

// GeneratePresignedURL generates a presigned URL for direct downloads
func (s *StorageService) GeneratePresignedURL(bucket, objectKey string, expiry time.Duration) (string, error) {
	// For now, return empty string to indicate presigned URLs are not supported
	// This will cause the system to fall back to token-based downloads
	// In future, we can implement presigned URLs for S3 here
	return "", fmt.Errorf("presigned URLs not supported by current storage provider")
}

// CreateFolderWithParent creates a folder with explicit parent folder ID
func (s *StorageService) CreateFolderWithParent(bucket, folderName, userID string, parentFolderID *string) (string, error) {
	if s.provider == nil {
		return "", fmt.Errorf("storage not initialized")
	}

	// Clean up the folder name
	folderName = strings.TrimSpace(folderName)
	if folderName == "" {
		return "", fmt.Errorf("folder name cannot be empty")
	}

	log.Printf("CreateFolderWithParent: Creating folder '%s' in bucket '%s' for user '%s', parent: %v",
		folderName, bucket, userID, parentFolderID)

	// Generate a unique ID for the folder
	folderID := uuid.New().String()

	// Storage key for folder (we don't actually store anything for folders in the provider)
	storageKey := fmt.Sprintf("%s/%s", folderID, folderName)

	// Check if folder already exists at this location
	var existingFolder storage.StorageObject
	query := s.db.Where("bucket_name = ? AND object_name = ? AND content_type = ? AND user_id = ?",
		bucket, folderName, "application/x-directory", userID)

	// Add AppID filter if present
	if s.appID != "" {
		query = query.Where("app_id = ?", s.appID)
	} else {
		query = query.Where("app_id IS NULL")
	}

	// Add parent folder filter
	if parentFolderID != nil {
		query = query.Where("parent_folder_id = ?", *parentFolderID)
	} else {
		query = query.Where("parent_folder_id IS NULL")
	}

	if err := query.First(&existingFolder).Error; err == nil {
		// Folder with same name exists in the same location
		// Let's create a new folder with a numbered suffix
		log.Printf("CreateFolderWithParent: Folder '%s' already exists, will create with suffix", folderName)

		// Find a unique name by adding a number suffix
		baseName := folderName
		counter := 1
		for {
			// Try with suffix
			newFolderName := fmt.Sprintf("%s (%d)", baseName, counter)

			// Check if this name exists
			var checkFolder storage.StorageObject
			checkQuery := s.db.Where("bucket_name = ? AND object_name = ? AND content_type = ? AND user_id = ?",
				bucket, newFolderName, "application/x-directory", userID)

			// Add AppID filter if present
			if s.appID != "" {
				checkQuery = checkQuery.Where("app_id = ?", s.appID)
			} else {
				checkQuery = checkQuery.Where("app_id IS NULL")
			}

			// Add parent folder filter
			if parentFolderID != nil {
				checkQuery = checkQuery.Where("parent_folder_id = ?", *parentFolderID)
			} else {
				checkQuery = checkQuery.Where("parent_folder_id IS NULL")
			}

			if err := checkQuery.First(&checkFolder).Error; err != nil {
				// This name doesn't exist, use it
				folderName = newFolderName
				log.Printf("CreateFolderWithParent: Will create folder with name: %s", folderName)
				break
			}

			counter++
			if counter > 100 {
				// Safety check to avoid infinite loop
				return "", fmt.Errorf("could not find unique folder name after 100 attempts")
			}
		}
	}

	// Create a placeholder file in storage
	keepFilePath := storageKey + "/.keep"
	content := []byte("")
	err := s.provider.PutObject(s.ctx, bucket, keepFilePath, bytes.NewReader(content), 0, storage.PutObjectOptions{
		ContentType: "application/x-directory",
	})
	if err != nil {
		log.Printf("CreateFolderWithParent: Failed to create .keep file: %v", err)
		return "", fmt.Errorf("failed to create folder structure: %v", err)
	}

	// Get app ID as pointer
	var appIDPtr *string
	if s.appID != "" {
		appIDPtr = &s.appID
	}

	// Create folder object in database with simplified structure
	folderObj := &storage.StorageObject{
		ID:             folderID,
		BucketName:     bucket,
		ObjectName:     folderName,
		ParentFolderID: parentFolderID,
		Size:           0,
		ContentType:    "application/x-directory",
		UserID:         userID,
		AppID:          appIDPtr,
		CreatedAt:      time.Now(),
		UpdatedAt:      time.Now(),
	}

	if err := s.db.Create(folderObj).Error; err != nil {
		// Try to rollback storage creation
		s.provider.DeleteObject(s.ctx, bucket, keepFilePath)
		return "", fmt.Errorf("failed to create folder in database: %v", err)
	}

	log.Printf("CreateFolderWithParent: Successfully created folder with ID: %s", folderObj.ID)
	return folderObj.ID, nil
}

// GetDB returns the database connection (needed for API handler)
func (s *StorageService) GetDB() *database.DB {
	return s.db
}

func (s *StorageService) DeleteObject(bucket, objectID string) error {
	if s.provider == nil {
		return fmt.Errorf("storage not initialized")
	}

	// Get object from database to get the actual key
	var obj storage.StorageObject
	if err := s.db.Where("id = ? AND bucket_name = ?", objectID, bucket).First(&obj).Error; err != nil {
		return fmt.Errorf("object not found")
	}

	// Build the storage key using the simple ID-based approach
	storageKey := s.getStorageKey(&obj)

	// Delete from storage provider
	if err := s.provider.DeleteObject(s.ctx, bucket, storageKey); err != nil {
		return err
	}

	// Delete from database
	if err := s.db.Delete(&obj).Error; err != nil {
		return err
	}

	return nil
}

func (s *StorageService) GetTotalStorageUsed() (int64, error) {
	var totalSize int64

	// Get total storage used from database
	if err := s.db.Model(&storage.StorageObject{}).
		Select("COALESCE(SUM(size), 0)").
		Scan(&totalSize).Error; err != nil {
		return 0, err
	}

	return totalSize, nil
}

// GetUserStorageUsed returns the total storage used by a specific user
func (s *StorageService) GetUserStorageUsed(userID string) (int64, error) {
	var totalSize int64

	if err := s.db.Model(&storage.StorageObject{}).
		Select("COALESCE(SUM(size), 0)").
		Where("user_id = ?", userID).
		Scan(&totalSize).Error; err != nil {
		return 0, err
	}

	return totalSize, nil
}

// GetStorageStats returns comprehensive storage statistics
func (s *StorageService) GetStorageStats(userID string) (map[string]interface{}, error) {
	stats := make(map[string]interface{})

	// Get total file count for user
	var fileCount int64
	if err := s.db.Model(&storage.StorageObject{}).
		Where("user_id = ? AND NOT is_folder", userID).
		Count(&fileCount).Error; err != nil {
		return nil, err
	}
	stats["file_count"] = fileCount

	// Get total folder count for user
	var folderCount int64
	if err := s.db.Model(&storage.StorageObject{}).
		Where("user_id = ? AND is_folder", userID).
		Count(&folderCount).Error; err != nil {
		return nil, err
	}
	stats["folder_count"] = folderCount

	// Get total storage used
	totalSize, err := s.GetUserStorageUsed(userID)
	if err != nil {
		return nil, err
	}
	stats["total_size"] = totalSize

	// Get shared files count (if public column exists)
	var sharedCount int64
	if err := s.db.Model(&storage.StorageObject{}).
		Where("user_id = ? AND public = ?", userID, true).
		Count(&sharedCount).Error; err != nil {
		// Ignore error if public column doesn't exist
		sharedCount = 0
	}
	stats["shared_count"] = sharedCount

	// Get recent uploads (last 7 days)
	var recentCount int64
	sevenDaysAgo := time.Now().AddDate(0, 0, -7)
	if err := s.db.Model(&storage.StorageObject{}).
		Where("user_id = ? AND created_at >= ?", userID, sevenDaysAgo).
		Count(&recentCount).Error; err != nil {
		recentCount = 0
	}
	stats["recent_uploads"] = recentCount

	return stats, nil
}

// GetAllUsersStorageStats returns storage statistics for all users (admin use)
func (s *StorageService) GetAllUsersStorageStats() (map[string]interface{}, error) {
	stats := make(map[string]interface{})

	// Get total storage used across all users
	totalSize, err := s.GetTotalStorageUsed()
	if err != nil {
		return nil, err
	}
	stats["total_storage_used"] = totalSize

	// Get total file count
	var totalFiles int64
	if err := s.db.Model(&storage.StorageObject{}).
		Where("NOT is_folder").
		Count(&totalFiles).Error; err != nil {
		return nil, err
	}
	stats["total_files"] = totalFiles

	// Get total folder count
	var totalFolders int64
	if err := s.db.Model(&storage.StorageObject{}).
		Where("is_folder").
		Count(&totalFolders).Error; err != nil {
		return nil, err
	}
	stats["total_folders"] = totalFolders

	// Get number of users with files
	var activeUsers int64
	if err := s.db.Model(&storage.StorageObject{}).
		Select("COUNT(DISTINCT user_id)").
		Scan(&activeUsers).Error; err != nil {
		return nil, err
	}
	stats["active_users"] = activeUsers

	return stats, nil
}

// getPublicURL constructs a public URL for an object
func (s *StorageService) getPublicURL(bucket, key string) string {
	if s.provider == nil {
		return ""
	}
	// For now, return a simple path - this should be enhanced based on provider type
	return "/" + bucket + "/" + key
}

// GetPublicURL returns the public URL for an object
func (s *StorageService) GetPublicURL(bucket, key string) string {
	return s.getPublicURL(bucket, key)
}

// RenameObject renames an object in storage
func (s *StorageService) RenameObject(bucket, objectID, newName string) error {
	// First get the object from database
	var object storage.StorageObject
	if err := s.db.Where("id = ? AND bucket_name = ?", objectID, bucket).First(&object).Error; err != nil {
		return fmt.Errorf("object not found: %v", err)
	}

	// Build old key using the simple ID-based approach
	oldKey := s.getStorageKey(&object)

	// Build new key - keep same ID but new filename
	newKey := fmt.Sprintf("%s/%s", object.ID, newName)

	// Check if new name already exists
	var existingCount int64
	s.db.Model(&storage.StorageObject{}).
		Where("bucket_name = ? AND object_key = ?", bucket, newKey).
		Count(&existingCount)

	if existingCount > 0 {
		return fmt.Errorf("an object with name '%s' already exists", newName)
	}

	// If it's a file, rename in storage backend
	if object.ContentType != "application/x-directory" {
		// Copy to new location
		reader, err := s.provider.GetObject(s.ctx, bucket, oldKey)
		if err != nil {
			return fmt.Errorf("failed to get object: %v", err)
		}
		defer reader.Close()

		// Read the content
		content, err := io.ReadAll(reader)
		if err != nil {
			return fmt.Errorf("failed to read object: %v", err)
		}

		// Upload with new name
		if err := s.provider.PutObject(s.ctx, bucket, newKey, bytes.NewReader(content), int64(len(content)), storage.PutObjectOptions{
			ContentType: object.ContentType,
		}); err != nil {
			return fmt.Errorf("failed to put renamed object: %v", err)
		}

		// Delete old object from storage
		if err := s.provider.DeleteObject(s.ctx, bucket, oldKey); err != nil {
			// Try to clean up the new object
			s.provider.DeleteObject(s.ctx, bucket, newKey)
			return fmt.Errorf("failed to delete old object: %v", err)
		}
	}

	// Update database record with new name
	object.ObjectName = newName
	if err := s.db.Save(&object).Error; err != nil {
		// If database update fails and it's a file, try to revert storage changes
		if object.ContentType != "application/x-directory" {
			// Try to restore original
			reader, _ := s.provider.GetObject(s.ctx, bucket, newKey)
			if reader != nil {
				defer reader.Close()
				content, _ := io.ReadAll(reader)
				s.provider.PutObject(s.ctx, bucket, oldKey, bytes.NewReader(content), int64(len(content)), storage.PutObjectOptions{
					ContentType: object.ContentType,
				})
				s.provider.DeleteObject(s.ctx, bucket, newKey)
			}
		}
		return fmt.Errorf("failed to update database: %v", err)
	}

	// If it's a folder, we don't need to update child paths since we use IDs
	// The children still reference the same parent folder ID

	return nil
}

// UpdateObjectMetadata updates the metadata field of a storage object
func (s *StorageService) UpdateObjectMetadata(bucket, objectID, metadata string) error {
	// Update the metadata field in the database
	result := s.db.Model(&storage.StorageObject{}).
		Where("id = ? AND bucket_name = ?", objectID, bucket).
		Update("metadata", metadata)

	if result.Error != nil {
		return fmt.Errorf("failed to update metadata: %v", result.Error)
	}

	if result.RowsAffected == 0 {
		return fmt.Errorf("object not found")
	}

	return nil
}

// EnsureUserMyFilesFolder ensures that the user has a "My Files" folder and returns its ID
func (s *StorageService) EnsureUserMyFilesFolder(userID string) (string, error) {
	if userID == "" {
		return "", fmt.Errorf("user ID is required")
	}

	// Check if "My Files" folder already exists for this user
	var existingFolder storage.StorageObject
	query := s.db.Where("bucket_name = ? AND object_name = ? AND content_type = ? AND user_id = ?",
		"int_storage", "My Files", "application/x-directory", userID)

	// Add AppID filter if present
	if s.appID != "" {
		query = query.Where("app_id = ?", s.appID)
	}

	// Check for existing folder with parent_folder_id = NULL (root level)
	query = query.Where("parent_folder_id IS NULL")

	err := query.First(&existingFolder).Error
	if err == nil {
		// Folder already exists
		log.Printf("EnsureUserMyFilesFolder: My Files folder already exists for user %s with ID %s", userID, existingFolder.ID)
		return existingFolder.ID, nil
	}

	// Create "My Files" folder
	log.Printf("EnsureUserMyFilesFolder: Creating My Files folder for user %s", userID)

	folderID := uuid.New().String()

	// Prepare AppID pointer
	var appIDPtr *string
	if s.appID != "" {
		appIDPtr = &s.appID
	}

	// Create the folder record in database
	folder := storage.StorageObject{
		ID:             folderID,
		BucketName:     "int_storage",
		ObjectName:     "My Files",
		UserID:         userID,
		ContentType:    "application/x-directory",
		Size:           0,
		ParentFolderID: nil, // Root level folder
		AppID:          appIDPtr,
	}

	if err := s.db.Create(&folder).Error; err != nil {
		return "", fmt.Errorf("failed to create My Files folder: %v", err)
	}

	log.Printf("EnsureUserMyFilesFolder: Created My Files folder for user %s with ID %s", userID, folderID)
	return folderID, nil
}

// Helper functions
func formatBytes(bytes int64) string {
	if bytes == 0 {
		return "0 B"
	}
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

func getFileType(filename string) string {
	// Get file extension
	ext := ""
	for i := len(filename) - 1; i >= 0; i-- {
		if filename[i] == '.' {
			ext = filename[i+1:]
			break
		}
	}

	switch ext {
	case "jpg", "jpeg", "png", "gif", "webp", "svg":
		return "image"
	case "pdf":
		return "pdf"
	case "mp4", "avi", "mov", "webm":
		return "video"
	case "mp3", "wav", "ogg", "m4a":
		return "audio"
	case "zip", "tar", "gz", "rar", "7z":
		return "archive"
	case "json", "js", "ts", "go", "py", "html", "css":
		return "code"
	default:
		return "file"
	}
}
