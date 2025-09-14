package storage

import (
	"context"
	"crypto/md5"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// LocalProvider implements storage using the local filesystem
type LocalProvider struct {
	basePath string
	baseURL  string
}

// NewLocalProvider creates a new local storage provider
func NewLocalProvider(cfg Config) (*LocalProvider, error) {
	basePath := cfg.BasePath
	if basePath == "" {
		// Default to a storage directory in the .data folder
		basePath = "./.data/storage"
	}

	// Ensure base path exists
	if err := os.MkdirAll(basePath, 0755); err != nil {
		return nil, fmt.Errorf("failed to create base path: %w", err)
	}

	return &LocalProvider{
		basePath: basePath,
		baseURL:  cfg.BaseURL,
	}, nil
}

// Name returns the provider name
func (l *LocalProvider) Name() string {
	return "Local Filesystem"
}

// Type returns the provider type
func (l *LocalProvider) Type() ProviderType {
	return ProviderLocal
}

// CreateBucket creates a new bucket (directory)
func (l *LocalProvider) CreateBucket(ctx context.Context, name string, opts CreateBucketOptions) error {
	bucketPath := filepath.Join(l.basePath, name)

	// Check if bucket already exists
	if _, err := os.Stat(bucketPath); err == nil {
		return fmt.Errorf("bucket %s already exists", name)
	}

	// Create the bucket directory
	if err := os.MkdirAll(bucketPath, 0755); err != nil {
		return fmt.Errorf("failed to create bucket: %w", err)
	}

	// Store bucket metadata
	metadata := map[string]interface{}{
		"created_at": time.Now(),
		"public":     opts.Public,
	}

	if err := l.writeBucketMetadata(name, metadata); err != nil {
		// Clean up the directory if metadata write fails
		os.RemoveAll(bucketPath)
		return err
	}

	return nil
}

// DeleteBucket deletes a bucket and all its contents
func (l *LocalProvider) DeleteBucket(ctx context.Context, name string) error {
	bucketPath := filepath.Join(l.basePath, name)

	// Check if bucket exists
	if _, err := os.Stat(bucketPath); os.IsNotExist(err) {
		return fmt.Errorf("bucket %s does not exist", name)
	}

	// Remove the bucket and all its contents
	if err := os.RemoveAll(bucketPath); err != nil {
		return fmt.Errorf("failed to delete bucket: %w", err)
	}

	// Remove metadata
	metadataPath := filepath.Join(l.basePath, ".metadata", name+".json")
	os.Remove(metadataPath)

	return nil
}

// BucketExists checks if a bucket exists
func (l *LocalProvider) BucketExists(ctx context.Context, name string) (bool, error) {
	bucketPath := filepath.Join(l.basePath, name)
	_, err := os.Stat(bucketPath)
	if os.IsNotExist(err) {
		return false, nil
	}
	if err != nil {
		return false, err
	}
	return true, nil
}

// ListBuckets lists all buckets
func (l *LocalProvider) ListBuckets(ctx context.Context) ([]BucketInfo, error) {
	entries, err := os.ReadDir(l.basePath)
	if err != nil {
		return nil, fmt.Errorf("failed to list buckets: %w", err)
	}

	var buckets []BucketInfo
	for _, entry := range entries {
		if entry.IsDir() && !strings.HasPrefix(entry.Name(), ".") {
			info, err := entry.Info()
			if err != nil {
				continue
			}

			// Get bucket stats
			objectCount, totalSize := l.getBucketStats(entry.Name())

			// Load metadata if available
			metadata := l.readBucketMetadata(entry.Name())
			public := false
			if pub, ok := metadata["public"].(bool); ok {
				public = pub
			}

			buckets = append(buckets, BucketInfo{
				Name:        entry.Name(),
				CreatedAt:   info.ModTime(),
				Public:      public,
				ObjectCount: objectCount,
				TotalSize:   totalSize,
			})
		}
	}

	return buckets, nil
}

// PutObject stores an object in a bucket
func (l *LocalProvider) PutObject(ctx context.Context, bucket, key string, reader io.Reader, size int64, opts PutObjectOptions) error {
	// Validate bucket exists
	if exists, err := l.BucketExists(ctx, bucket); err != nil {
		return err
	} else if !exists {
		return fmt.Errorf("bucket %s does not exist", bucket)
	}

	// Clean and validate the key
	key = cleanKey(key)
	objectPath := filepath.Join(l.basePath, bucket, key)

	// Create parent directories if needed
	if err := os.MkdirAll(filepath.Dir(objectPath), 0755); err != nil {
		return fmt.Errorf("failed to create directories: %w", err)
	}

	// Create the file
	file, err := os.Create(objectPath)
	if err != nil {
		return fmt.Errorf("failed to create file: %w", err)
	}
	defer file.Close()

	// Copy the content
	written, err := io.Copy(file, reader)
	if err != nil {
		os.Remove(objectPath)
		return fmt.Errorf("failed to write file: %w", err)
	}

	// Verify size if provided
	if size > 0 && written != size {
		os.Remove(objectPath)
		return fmt.Errorf("size mismatch: expected %d, got %d", size, written)
	}

	// Store object metadata
	if len(opts.Metadata) > 0 || opts.ContentType != "" {
		metadata := map[string]interface{}{
			"content_type": opts.ContentType,
			"metadata":     opts.Metadata,
			"uploaded_at":  time.Now(),
		}
		l.writeObjectMetadata(bucket, key, metadata)
	}

	return nil
}

// GetObject retrieves an object from a bucket
func (l *LocalProvider) GetObject(ctx context.Context, bucket, key string) (io.ReadCloser, error) {
	key = cleanKey(key)
	objectPath := filepath.Join(l.basePath, bucket, key)

	file, err := os.Open(objectPath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, fmt.Errorf("object not found")
		}
		return nil, fmt.Errorf("failed to open file: %w", err)
	}

	return file, nil
}

// GetObjectInfo retrieves information about an object
func (l *LocalProvider) GetObjectInfo(ctx context.Context, bucket, key string) (*ObjectInfo, error) {
	key = cleanKey(key)
	objectPath := filepath.Join(l.basePath, bucket, key)

	stat, err := os.Stat(objectPath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, fmt.Errorf("object not found")
		}
		return nil, fmt.Errorf("failed to stat file: %w", err)
	}

	// Load metadata if available
	metadata := l.readObjectMetadata(bucket, key)
	contentType := ""
	if ct, ok := metadata["content_type"].(string); ok {
		contentType = ct
	}

	// Generate ETag (using MD5 of the path for simplicity)
	h := md5.New()
	h.Write([]byte(objectPath))
	etag := hex.EncodeToString(h.Sum(nil))

	return &ObjectInfo{
		Key:          key,
		Size:         stat.Size(),
		ETag:         etag,
		ContentType:  contentType,
		LastModified: stat.ModTime(),
		IsDir:        stat.IsDir(),
	}, nil
}

// DeleteObject deletes an object from a bucket
func (l *LocalProvider) DeleteObject(ctx context.Context, bucket, key string) error {
	key = cleanKey(key)
	objectPath := filepath.Join(l.basePath, bucket, key)

	if err := os.Remove(objectPath); err != nil {
		if os.IsNotExist(err) {
			return fmt.Errorf("object not found")
		}
		return fmt.Errorf("failed to delete file: %w", err)
	}

	// Remove metadata
	l.deleteObjectMetadata(bucket, key)

	return nil
}

// ListObjects lists objects in a bucket with the given prefix
func (l *LocalProvider) ListObjects(ctx context.Context, bucket, prefix string, opts ListObjectsOptions) ([]ObjectInfo, error) {
	bucketPath := filepath.Join(l.basePath, bucket)

	// Check if bucket exists
	if _, err := os.Stat(bucketPath); os.IsNotExist(err) {
		return nil, fmt.Errorf("bucket %s does not exist", bucket)
	}

	var objects []ObjectInfo
	walkPath := bucketPath
	if prefix != "" {
		walkPath = filepath.Join(bucketPath, prefix)
	}

	err := filepath.Walk(walkPath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil // Skip errors
		}

		// Skip hidden files and directories
		if strings.HasPrefix(info.Name(), ".") {
			if info.IsDir() {
				return filepath.SkipDir
			}
			return nil
		}

		// Get relative path from bucket
		relPath, err := filepath.Rel(bucketPath, path)
		if err != nil {
			return nil
		}

		// Skip the bucket directory itself
		if relPath == "." {
			return nil
		}

		// Convert to forward slashes for consistency
		key := filepath.ToSlash(relPath)

		// Handle non-recursive listing
		if !opts.Recursive && info.IsDir() && path != walkPath {
			return filepath.SkipDir
		}

		// Skip directories unless we want them
		if info.IsDir() && !opts.Recursive {
			return nil
		}

		// Generate ETag
		h := md5.New()
		h.Write([]byte(path))
		etag := hex.EncodeToString(h.Sum(nil))

		// Load metadata if available
		metadata := l.readObjectMetadata(bucket, key)
		contentType := ""
		if ct, ok := metadata["content_type"].(string); ok {
			contentType = ct
		}

		objects = append(objects, ObjectInfo{
			Key:          key,
			Size:         info.Size(),
			ETag:         etag,
			ContentType:  contentType,
			LastModified: info.ModTime(),
			IsDir:        info.IsDir(),
		})

		// Check max keys limit
		if opts.MaxKeys > 0 && len(objects) >= opts.MaxKeys {
			return io.EOF
		}

		return nil
	})

	if err != nil && !errors.Is(err, io.EOF) {
		return nil, fmt.Errorf("failed to list objects: %w", err)
	}

	return objects, nil
}

// GeneratePresignedURL generates a presigned URL for temporary access
func (l *LocalProvider) GeneratePresignedURL(ctx context.Context, bucket, key string, expires time.Duration) (string, error) {
	// For local storage, we'll generate a simple URL with expiration
	// In production, this would need proper signing and validation
	expireTime := time.Now().Add(expires).Unix()

	if l.baseURL == "" {
		return "", fmt.Errorf("base URL not configured for presigned URLs")
	}

	// Clean the key
	key = cleanKey(key)

	// Generate a simple signed URL (in production, use proper signing)
	url := fmt.Sprintf("%s/storage/%s/%s?expires=%d", l.baseURL, bucket, key, expireTime)

	return url, nil
}

// Helper functions

func (l *LocalProvider) getBucketStats(bucket string) (int64, int64) {
	var count int64
	var size int64

	bucketPath := filepath.Join(l.basePath, bucket)
	filepath.Walk(bucketPath, func(path string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() {
			return nil
		}
		count++
		size += info.Size()
		return nil
	})

	return count, size
}

func (l *LocalProvider) writeBucketMetadata(bucket string, metadata map[string]interface{}) error {
	// For now, we'll skip metadata storage for simplicity
	// In production, you'd want to store this in a .metadata directory
	return nil
}

func (l *LocalProvider) readBucketMetadata(bucket string) map[string]interface{} {
	// For now, return empty metadata
	return make(map[string]interface{})
}

func (l *LocalProvider) writeObjectMetadata(bucket, key string, metadata map[string]interface{}) error {
	// For now, we'll skip metadata storage for simplicity
	return nil
}

func (l *LocalProvider) readObjectMetadata(bucket, key string) map[string]interface{} {
	// For now, return empty metadata
	return make(map[string]interface{})
}

func (l *LocalProvider) deleteObjectMetadata(bucket, key string) {
	// For now, this is a no-op
}

// cleanKey cleans and validates an object key
func cleanKey(key string) string {
	// Remove leading slash
	key = strings.TrimPrefix(key, "/")
	// Clean the path
	key = filepath.Clean(key)
	// Convert to forward slashes
	key = filepath.ToSlash(key)
	return key
}
