package services

import (
	"bytes"
	"context"
	"crypto/md5"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"log"
	"strings"

	"github.com/suppers-ai/solobase/internal/config"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/fileutil"
	"github.com/suppers-ai/solobase/internal/pkg/storage"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
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
	repo     repos.StorageRepository
	appID    string // Application ID for storage isolation
	ctx      context.Context
}

func NewStorageService(repo repos.StorageRepository, cfg config.StorageConfig) *StorageService {
	// Default to "solobase" app ID
	return NewStorageServiceWithOptions(repo, cfg, &StorageOptions{
		AppID: "solobase",
	})
}

// NewStorageServiceWithOptions creates a new storage service with custom options
func NewStorageServiceWithOptions(repo repos.StorageRepository, cfg config.StorageConfig, opts *StorageOptions) *StorageService {
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

	// Ensure storage directory exists for local storage (no-op in WASM builds)
	if cfg.Type != "s3" {
		if err := fileutil.EnsureDir(localPath); err != nil {
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
			log.Printf("Failed to initialize S3 storage: %v, falling back to noop", err)
			provider = storage.NewNoopProvider()
		}
	default:
		localConfig := storage.Config{
			Provider: storage.ProviderLocal,
			BasePath: localPath,
		}
		provider, err = storage.NewProvider(localConfig)
		if err != nil {
			log.Printf("Failed to initialize local storage: %v, using noop provider", err)
			provider = storage.NewNoopProvider()
		}
	}

	service := &StorageService{
		config:   cfg,
		provider: provider,
		repo:     repo,
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
		_, err := s.repo.GetBucketByName(s.ctx, bucket.name)
		if err != nil {
			// Bucket doesn't exist, create it
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
	log.Printf("GetObjectInfo: Looking for object with id=%s in bucket=%s", objectID, bucket)
	obj, err := s.repo.GetObject(s.ctx, objectID)
	if err != nil {
		if err == repos.ErrNotFound {
			log.Printf("GetObjectInfo: Failed to find object: not found")
			return nil, errors.New("object not found")
		}
		log.Printf("GetObjectInfo: Failed to find object: %v", err)
		return nil, err
	}
	if obj.BucketName != bucket {
		log.Printf("GetObjectInfo: Object bucket mismatch")
		return nil, errors.New("object not found in bucket")
	}
	log.Printf("GetObjectInfo: Found object with id=%s, name=%s", obj.ID, obj.ObjectName)
	return obj, nil
}

// GetObjectByKey retrieves an object by its storage key
func (s *StorageService) GetObjectByKey(bucket, key string) (io.ReadCloser, string, string, error) {
	if s.provider == nil {
		return nil, "", "", fmt.Errorf("storage not initialized")
	}

	// Get the actual file from storage
	reader, err := s.provider.GetObject(s.ctx, bucket, key)
	if err != nil {
		return nil, "", "", err
	}

	// Extract filename from key
	parts := strings.Split(key, "/")
	filename := parts[len(parts)-1]

	return reader, filename, "", nil
}

// GeneratePresignedDownloadURL generates a presigned URL for downloading (S3 only)
func (s *StorageService) GeneratePresignedDownloadURL(bucket, key string, expiry int) (string, error) {
	if s.config.Type != "s3" {
		return "", fmt.Errorf("presigned URLs are only supported for S3 storage")
	}
	return s.provider.GeneratePresignedURL(s.ctx, bucket, key, apptime.Duration(expiry)*apptime.Second)
}

// GeneratePresignedUploadURL generates a presigned URL for uploading (S3 only)
func (s *StorageService) GeneratePresignedUploadURL(bucket, key, contentType string, expiry int) (string, error) {
	if s.config.Type != "s3" {
		return "", fmt.Errorf("presigned URLs are only supported for S3 storage")
	}
	return s.provider.GeneratePresignedURL(s.ctx, bucket, key, apptime.Duration(expiry)*apptime.Second)
}

func (s *StorageService) CreateBucket(name string, public bool) error {
	if s.provider == nil {
		return fmt.Errorf("storage not initialized")
	}

	// Check if bucket already exists in database
	_, err := s.repo.GetBucketByName(s.ctx, name)
	if err == nil {
		return fmt.Errorf("bucket %s already exists", name)
	}

	// Create bucket in storage provider
	err = s.provider.CreateBucket(s.ctx, name, storage.CreateBucketOptions{Public: public})
	if err != nil {
		if !strings.Contains(err.Error(), "exists") && !strings.Contains(err.Error(), "exist") {
			return err
		}
	}

	// Save bucket to database
	now := apptime.NowTime()
	bucket := &storage.StorageBucket{
		ID:        uuid.New().String(),
		Name:      name,
		Public:    public,
		CreatedAt: now,
		UpdatedAt: now,
	}
	if err := s.repo.CreateBucket(s.ctx, bucket); err != nil {
		s.provider.DeleteBucket(s.ctx, name)
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

	// Delete all objects from database
	if err := s.repo.DeleteObjectsByBucket(s.ctx, name); err != nil {
		return err
	}

	// Get bucket by name to get ID
	bucket, err := s.repo.GetBucketByName(s.ctx, name)
	if err != nil {
		return err
	}

	// Delete bucket from database
	if err := s.repo.DeleteBucket(s.ctx, bucket.ID); err != nil {
		return err
	}

	return nil
}

func (s *StorageService) GetBuckets() ([]interface{}, error) {
	if s == nil || s.provider == nil {
		return []interface{}{}, nil
	}
	if s.repo == nil {
		return []interface{}{}, nil
	}

	// Get buckets from database
	buckets, err := s.repo.ListBuckets(s.ctx)
	if err != nil {
		// In WASM mode, repository returns "not implemented" - return empty list
		if strings.Contains(err.Error(), "not implemented") {
			return []interface{}{}, nil
		}
		return nil, err
	}

	// Convert to interface slice with stats
	result := make([]interface{}, len(buckets))
	for i, bucket := range buckets {
		// Get object count
		count, _ := s.repo.CountObjectsByBucket(s.ctx, bucket.Name)

		// Get total size
		totalSize, _ := s.repo.SumSizeByBucket(s.ctx, bucket.Name)

		// Format created_at date
		createdAtStr := bucket.CreatedAt.Format("2006-01-02")

		result[i] = map[string]interface{}{
			"id":         bucket.ID,
			"name":       bucket.Name,
			"public":     bucket.Public,
			"created_at": createdAtStr,
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

	// Use repository to list objects with filters
	var userIDPtr *string
	if userID != "" {
		userIDPtr = &userID
	}
	var appIDPtr *string
	if s.appID != "" {
		appIDPtr = &s.appID
	}

	objects, err := s.repo.ListObjects(s.ctx, repos.ListObjectsOptions{
		BucketName:     bucket,
		UserID:         userIDPtr,
		AppID:          appIDPtr,
		ParentFolderID: parentFolderID,
	})
	if err != nil {
		return nil, err
	}

	result := make([]interface{}, 0, len(objects))
	for _, obj := range objects {
		// Skip empty names and .keep files
		if obj.ObjectName == "" || obj.ObjectName == ".keep" {
			continue
		}

		fileType := "file"
		if obj.ContentType == "application/x-directory" {
			fileType = "folder"
		} else {
			fileType = getFileType(obj.ObjectName)
		}

		result = append(result, map[string]interface{}{
			"id":               obj.ID,
			"bucket_name":      obj.BucketName,
			"object_name":      obj.ObjectName,
			"parent_folder_id": obj.ParentFolderID,
			"size":             obj.Size,
			"content_type":     obj.ContentType,
			"type":             fileType,
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
	storageKey := fmt.Sprintf("%s/%s", objectID, filename)

	// Upload to storage provider
	err := s.provider.PutObject(s.ctx, bucket, storageKey, &buf, size, storage.PutObjectOptions{
		ContentType: mimeType,
	})
	if err != nil {
		return nil, err
	}

	// Save to database
	now := apptime.NowTime()
	obj := &storage.StorageObject{
		ID:             objectID,
		BucketName:     bucket,
		ObjectName:     filename,
		ParentFolderID: parentFolderID,
		Size:           size,
		ContentType:    mimeType,
		Checksum:       checksum,
		UserID:         userID,
		AppID:          strPtr(s.appID),
		CreatedAt:      now,
		UpdatedAt:      now,
	}

	if err := s.repo.CreateObject(s.ctx, obj); err != nil {
		s.provider.DeleteObject(s.ctx, bucket, storageKey)
		return nil, err
	}

	return map[string]interface{}{
		"id":               objectID,
		"size":             size,
		"content_type":     mimeType,
		"checksum":         checksum,
		"parent_folder_id": parentFolderID,
		"app_id":           strPtr(s.appID),
		"url":              s.getPublicURL(bucket, storageKey),
	}, nil
}

// UploadFileWithParent uploads a file with a specific parent folder ID
func (s *StorageService) UploadFileWithParent(bucket, filename, parentFolderID, userID string, reader io.Reader, size int64, mimeType string) (interface{}, error) {
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
func (s *StorageService) getStorageKey(objID, objName string) string {
	return fmt.Sprintf("%s/%s", objID, objName)
}

func (s *StorageService) GetObject(bucket, objectID string) (io.ReadCloser, string, string, error) {
	if s.provider == nil {
		return nil, "", "", fmt.Errorf("storage not initialized")
	}

	// Get object from database
	obj, err := s.repo.GetObject(s.ctx, objectID)
	if err != nil {
		if err == repos.ErrNotFound {
			return nil, "", "", fmt.Errorf("object not found")
		}
		return nil, "", "", err
	}
	if obj.BucketName != bucket {
		return nil, "", "", fmt.Errorf("object not found in bucket")
	}

	storageKey := s.getStorageKey(obj.ID, obj.ObjectName)

	reader, err := s.provider.GetObject(s.ctx, bucket, storageKey)
	if err != nil {
		return nil, "", "", err
	}

	return reader, obj.ObjectName, obj.ContentType, nil
}

// GeneratePresignedURL generates a presigned URL for direct downloads
func (s *StorageService) GeneratePresignedURL(bucket, objectKey string, expiry apptime.Duration) (string, error) {
	return "", fmt.Errorf("presigned URLs not supported by current storage provider")
}

// CreateFolderWithParent creates a folder with explicit parent folder ID
func (s *StorageService) CreateFolderWithParent(bucket, folderName, userID string, parentFolderID *string) (string, error) {
	if s.provider == nil {
		return "", fmt.Errorf("storage not initialized")
	}

	folderName = strings.TrimSpace(folderName)
	if folderName == "" {
		return "", fmt.Errorf("folder name cannot be empty")
	}

	log.Printf("CreateFolderWithParent: Creating folder '%s' in bucket '%s' for user '%s', parent: %v",
		folderName, bucket, userID, parentFolderID)

	folderID := uuid.New().String()
	storageKey := fmt.Sprintf("%s/%s", folderID, folderName)

	// Check if folder already exists - use GetObjectByPath
	_, err := s.repo.GetObjectByPath(s.ctx, bucket, folderName, parentFolderID)
	if err == nil {
		// Folder exists, find unique name
		baseName := folderName
		for counter := 1; counter <= 100; counter++ {
			folderName = fmt.Sprintf("%s (%d)", baseName, counter)
			_, err := s.repo.GetObjectByPath(s.ctx, bucket, folderName, parentFolderID)
			if err == repos.ErrNotFound {
				break
			}
			if counter == 100 {
				return "", fmt.Errorf("could not find unique folder name")
			}
		}
	}

	// Create placeholder file in storage
	keepFilePath := storageKey + "/.keep"
	err = s.provider.PutObject(s.ctx, bucket, keepFilePath, bytes.NewReader([]byte("")), 0, storage.PutObjectOptions{
		ContentType: "application/x-directory",
	})
	if err != nil {
		return "", fmt.Errorf("failed to create folder structure: %v", err)
	}

	now := apptime.NowTime()
	obj := &storage.StorageObject{
		ID:             folderID,
		BucketName:     bucket,
		ObjectName:     folderName,
		ParentFolderID: parentFolderID,
		Size:           0,
		ContentType:    "application/x-directory",
		UserID:         userID,
		AppID:          strPtr(s.appID),
		CreatedAt:      now,
		UpdatedAt:      now,
	}

	if err := s.repo.CreateObject(s.ctx, obj); err != nil {
		s.provider.DeleteObject(s.ctx, bucket, keepFilePath)
		return "", fmt.Errorf("failed to create folder in database: %v", err)
	}

	log.Printf("CreateFolderWithParent: Successfully created folder with ID: %s", folderID)
	return folderID, nil
}

func (s *StorageService) DeleteObject(bucket, objectID string) error {
	if s.provider == nil {
		return fmt.Errorf("storage not initialized")
	}

	obj, err := s.repo.GetObject(s.ctx, objectID)
	if err != nil {
		if err == repos.ErrNotFound {
			return fmt.Errorf("object not found")
		}
		return err
	}
	if obj.BucketName != bucket {
		return fmt.Errorf("object not found in bucket")
	}

	storageKey := s.getStorageKey(obj.ID, obj.ObjectName)

	if err := s.provider.DeleteObject(s.ctx, bucket, storageKey); err != nil {
		return err
	}

	return s.repo.DeleteObject(s.ctx, objectID)
}

func (s *StorageService) GetTotalStorageUsed() (int64, error) {
	return s.repo.SumTotalSize(s.ctx)
}

// GetUserStorageUsed returns the total storage used by a specific user
func (s *StorageService) GetUserStorageUsed(userID string) (int64, error) {
	return s.repo.SumSizeByUser(s.ctx, userID)
}

// GetRecentlyViewed returns recently viewed storage items for a user
func (s *StorageService) GetRecentlyViewed(userID string, limit int) ([]*storage.StorageObject, error) {
	return s.repo.ListRecentlyViewed(s.ctx, userID, limit)
}

// UpdateLastViewed updates the last viewed timestamp for an object
func (s *StorageService) UpdateLastViewed(objectID, userID string) error {
	now := apptime.NowTime()
	return s.repo.UpdateObjectLastViewed(s.ctx, objectID, now)
}

// SearchStorageObjects searches for storage objects by name
func (s *StorageService) SearchStorageObjects(userID, appID, query string, limit int) ([]*storage.StorageObject, error) {
	searchPattern := "%" + query + "%"
	return s.repo.SearchObjects(s.ctx, userID, appID, searchPattern, limit)
}

// GetStorageStats returns comprehensive storage statistics
func (s *StorageService) GetStorageStats(userID string) (map[string]interface{}, error) {
	stats := make(map[string]interface{})

	// Get file count (non-folders) - use CountObjectsByUser for now
	fileCount, _ := s.repo.CountObjectsByUser(s.ctx, userID)
	stats["fileCount"] = fileCount

	// Get folder count - would need a separate query
	stats["folderCount"] = int64(0)

	// Get total size
	totalSize, _ := s.GetUserStorageUsed(userID)
	stats["totalSize"] = totalSize

	// Get shared count (if public column exists - default 0)
	stats["sharedCount"] = int64(0)

	// Get recent uploads - would need a separate query with date filter
	stats["recentUploads"] = int64(0)

	return stats, nil
}

// GetAllUsersStorageStats returns storage statistics for all users (admin use)
func (s *StorageService) GetAllUsersStorageStats() (map[string]interface{}, error) {
	stats := make(map[string]interface{})

	totalSize, _ := s.GetTotalStorageUsed()
	stats["totalStorageUsed"] = totalSize

	// These would need specialized repository methods for accurate counts
	stats["totalFiles"] = int64(0)
	stats["totalFolders"] = int64(0)
	stats["activeUsers"] = int64(0)

	return stats, nil
}

func (s *StorageService) getPublicURL(bucket, key string) string {
	if s.provider == nil {
		return ""
	}
	return "/" + bucket + "/" + key
}

// GetPublicURL returns the public URL for an object
func (s *StorageService) GetPublicURL(bucket, key string) string {
	return s.getPublicURL(bucket, key)
}

// RenameObject renames an object in storage
func (s *StorageService) RenameObject(bucket, objectID, newName string) error {
	obj, err := s.repo.GetObject(s.ctx, objectID)
	if err != nil {
		return fmt.Errorf("object not found: %v", err)
	}
	if obj.BucketName != bucket {
		return fmt.Errorf("object not found in bucket")
	}

	oldKey := s.getStorageKey(obj.ID, obj.ObjectName)
	newKey := fmt.Sprintf("%s/%s", obj.ID, newName)

	// If it's a file, rename in storage backend
	if obj.ContentType != "application/x-directory" {
		reader, err := s.provider.GetObject(s.ctx, bucket, oldKey)
		if err != nil {
			return fmt.Errorf("failed to get object: %v", err)
		}
		defer reader.Close()

		content, err := io.ReadAll(reader)
		if err != nil {
			return fmt.Errorf("failed to read object: %v", err)
		}

		if err := s.provider.PutObject(s.ctx, bucket, newKey, bytes.NewReader(content), int64(len(content)), storage.PutObjectOptions{
			ContentType: obj.ContentType,
		}); err != nil {
			return fmt.Errorf("failed to put renamed object: %v", err)
		}

		if err := s.provider.DeleteObject(s.ctx, bucket, oldKey); err != nil {
			s.provider.DeleteObject(s.ctx, bucket, newKey)
			return fmt.Errorf("failed to delete old object: %v", err)
		}
	}

	// Update database
	obj.ObjectName = newName
	return s.repo.UpdateObject(s.ctx, obj)
}

// UpdateObjectMetadata updates the metadata field of a storage object
func (s *StorageService) UpdateObjectMetadata(bucket, objectID, metadata string) error {
	obj, err := s.repo.GetObject(s.ctx, objectID)
	if err != nil {
		return fmt.Errorf("object not found")
	}
	if obj.BucketName != bucket {
		return fmt.Errorf("object not found in bucket")
	}

	obj.Metadata = metadata
	return s.repo.UpdateObject(s.ctx, obj)
}

// EnsureUserMyFilesFolder ensures that the user has a "My Files" folder and returns its ID
func (s *StorageService) EnsureUserMyFilesFolder(userID string) (string, error) {
	if userID == "" {
		return "", fmt.Errorf("user ID is required")
	}

	// Check if folder exists using GetObjectByPath
	existingObj, err := s.repo.GetObjectByPath(s.ctx, "int_storage", "My Files", nil)
	if err == nil && existingObj.UserID == userID {
		log.Printf("EnsureUserMyFilesFolder: My Files folder already exists for user %s with ID %s", userID, existingObj.ID)
		return existingObj.ID, nil
	}

	// Create folder
	log.Printf("EnsureUserMyFilesFolder: Creating My Files folder for user %s", userID)
	folderID := uuid.New().String()

	now := apptime.NowTime()
	obj := &storage.StorageObject{
		ID:             folderID,
		BucketName:     "int_storage",
		ObjectName:     "My Files",
		ParentFolderID: nil,
		Size:           0,
		ContentType:    "application/x-directory",
		UserID:         userID,
		AppID:          strPtr(s.appID),
		CreatedAt:      now,
		UpdatedAt:      now,
	}

	if err := s.repo.CreateObject(s.ctx, obj); err != nil {
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

// strPtr returns a pointer to a string, or nil if empty
func strPtr(s string) *string {
	if s == "" {
		return nil
	}
	return &s
}
