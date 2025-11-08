package services

import (
	"context"
	"fmt"
	"io"
	"path/filepath"
	"time"

	"github.com/suppers-ai/solobase/internal/pkg/storage"
)

// ExtensionStorageService provides storage capabilities for extensions
type ExtensionStorageService struct {
	provider      storage.Provider
	extensionName string
	basePath      string
	ctx           context.Context
}

// NewExtensionStorageService creates a storage service for an extension
func NewExtensionStorageService(extensionName string) (*ExtensionStorageService, error) {
	// Extensions use ./.data/storage/ext/{extension_name} structure
	basePath := filepath.Join("./.data/storage/ext", extensionName)

	config := storage.Config{
		Provider: storage.ProviderLocal,
		BasePath: basePath,
	}

	provider, err := storage.NewProvider(config)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize extension storage: %w", err)
	}

	return &ExtensionStorageService{
		provider:      provider,
		extensionName: extensionName,
		basePath:      basePath,
		ctx:           context.Background(),
	}, nil
}

// CreateBucket creates a bucket for the extension
func (s *ExtensionStorageService) CreateBucket(name string, public bool) error {
	if s.provider == nil {
		return fmt.Errorf("storage not initialized")
	}
	return s.provider.CreateBucket(s.ctx, name, storage.CreateBucketOptions{Public: public})
}

// DeleteBucket deletes a bucket
func (s *ExtensionStorageService) DeleteBucket(name string) error {
	if s.provider == nil {
		return fmt.Errorf("storage not initialized")
	}
	return s.provider.DeleteBucket(s.ctx, name)
}

// ListBuckets lists all buckets for this extension
func (s *ExtensionStorageService) ListBuckets() ([]storage.BucketInfo, error) {
	if s.provider == nil {
		return nil, fmt.Errorf("storage not initialized")
	}
	return s.provider.ListBuckets(s.ctx)
}

// PutObject stores an object in a bucket
func (s *ExtensionStorageService) PutObject(bucket, key string, reader io.Reader, size int64, contentType string) error {
	if s.provider == nil {
		return fmt.Errorf("storage not initialized")
	}
	return s.provider.PutObject(s.ctx, bucket, key, reader, size, storage.PutObjectOptions{
		ContentType: contentType,
	})
}

// GetObject retrieves an object from a bucket
func (s *ExtensionStorageService) GetObject(bucket, key string) (io.ReadCloser, error) {
	if s.provider == nil {
		return nil, fmt.Errorf("storage not initialized")
	}
	return s.provider.GetObject(s.ctx, bucket, key)
}

// DeleteObject deletes an object from a bucket
func (s *ExtensionStorageService) DeleteObject(bucket, key string) error {
	if s.provider == nil {
		return fmt.Errorf("storage not initialized")
	}
	return s.provider.DeleteObject(s.ctx, bucket, key)
}

// ListObjects lists objects in a bucket
func (s *ExtensionStorageService) ListObjects(bucket, prefix string) ([]storage.ObjectInfo, error) {
	if s.provider == nil {
		return nil, fmt.Errorf("storage not initialized")
	}
	return s.provider.ListObjects(s.ctx, bucket, prefix, storage.ListObjectsOptions{})
}

// GetObjectInfo gets object metadata
func (s *ExtensionStorageService) GetObjectInfo(bucket, key string) (*storage.ObjectInfo, error) {
	if s.provider == nil {
		return nil, fmt.Errorf("storage not initialized")
	}
	return s.provider.GetObjectInfo(s.ctx, bucket, key)
}

// GetPublicURL gets the public URL for an object
// For extensions, this returns a path relative to /storage/ext/{extension}/{bucket}/{key}
func (s *ExtensionStorageService) GetPublicURL(bucket, key string) string {
	return fmt.Sprintf("/storage/ext/%s/%s/%s", s.extensionName, bucket, key)
}

// GetSignedURL generates a signed URL for temporary access
func (s *ExtensionStorageService) GetSignedURL(bucket, key string, expiry time.Duration) (string, error) {
	if s.provider == nil {
		return "", fmt.Errorf("storage not initialized")
	}
	return s.provider.GeneratePresignedURL(s.ctx, bucket, key, expiry)
}

// GetBasePath returns the base path for this extension's storage
func (s *ExtensionStorageService) GetBasePath() string {
	return s.basePath
}

// GetExtensionName returns the name of the extension
func (s *ExtensionStorageService) GetExtensionName() string {
	return s.extensionName
}
