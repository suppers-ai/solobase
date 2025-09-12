package services

import (
	"fmt"
	"io"
	"path/filepath"
	"time"

	"github.com/suppers-ai/solobase/storage"
)

// ExtensionStorageService provides storage capabilities for extensions
type ExtensionStorageService struct {
	provider      storage.Provider
	storage       *storage.Storage
	extensionName string
	basePath      string
}

// NewExtensionStorageService creates a storage service for an extension
func NewExtensionStorageService(extensionName string) (*ExtensionStorageService, error) {
	// Extensions use ./.data/storage/ext/{extension_name} structure
	basePath := filepath.Join("./.data/storage/ext", extensionName)

	provider, err := storage.NewLocalProvider(basePath)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize extension storage: %w", err)
	}

	return &ExtensionStorageService{
		provider:      provider,
		storage:       storage.New(provider),
		extensionName: extensionName,
		basePath:      basePath,
	}, nil
}

// CreateBucket creates a bucket for the extension
func (s *ExtensionStorageService) CreateBucket(name string, public bool) error {
	if s.storage == nil {
		return fmt.Errorf("storage not initialized")
	}
	return s.storage.CreateBucket(name, public)
}

// DeleteBucket deletes a bucket
func (s *ExtensionStorageService) DeleteBucket(name string) error {
	if s.storage == nil {
		return fmt.Errorf("storage not initialized")
	}
	return s.storage.DeleteBucket(name)
}

// ListBuckets lists all buckets for this extension
func (s *ExtensionStorageService) ListBuckets() ([]storage.Bucket, error) {
	if s.storage == nil {
		return nil, fmt.Errorf("storage not initialized")
	}
	return s.storage.ListBuckets()
}

// PutObject stores an object in a bucket
func (s *ExtensionStorageService) PutObject(bucket, key string, reader io.Reader, size int64, contentType string) error {
	if s.storage == nil {
		return fmt.Errorf("storage not initialized")
	}
	return s.storage.PutObject(bucket, key, reader, size, contentType)
}

// GetObject retrieves an object from a bucket
func (s *ExtensionStorageService) GetObject(bucket, key string) (io.ReadCloser, error) {
	if s.storage == nil {
		return nil, fmt.Errorf("storage not initialized")
	}
	return s.storage.GetObject(bucket, key)
}

// DeleteObject deletes an object from a bucket
func (s *ExtensionStorageService) DeleteObject(bucket, key string) error {
	if s.storage == nil {
		return fmt.Errorf("storage not initialized")
	}
	return s.storage.DeleteObject(bucket, key)
}

// ListObjects lists objects in a bucket
func (s *ExtensionStorageService) ListObjects(bucket, prefix string) ([]storage.Object, error) {
	if s.storage == nil {
		return nil, fmt.Errorf("storage not initialized")
	}
	return s.storage.ListObjects(bucket, prefix)
}

// GetObjectInfo gets object metadata
func (s *ExtensionStorageService) GetObjectInfo(bucket, key string) (*storage.Object, error) {
	if s.storage == nil {
		return nil, fmt.Errorf("storage not initialized")
	}
	return s.storage.GetObjectInfo(bucket, key)
}

// GetPublicURL gets the public URL for an object
// For extensions, this returns a path relative to /storage/ext/{extension}/{bucket}/{key}
func (s *ExtensionStorageService) GetPublicURL(bucket, key string) string {
	return fmt.Sprintf("/storage/ext/%s/%s/%s", s.extensionName, bucket, key)
}

// GetSignedURL generates a signed URL for temporary access
func (s *ExtensionStorageService) GetSignedURL(bucket, key string, expiry time.Duration) (string, error) {
	if s.storage == nil {
		return "", fmt.Errorf("storage not initialized")
	}
	return s.storage.GetSignedURL(bucket, key, expiry)
}

// GetBasePath returns the base path for this extension's storage
func (s *ExtensionStorageService) GetBasePath() string {
	return s.basePath
}

// GetExtensionName returns the name of the extension
func (s *ExtensionStorageService) GetExtensionName() string {
	return s.extensionName
}
