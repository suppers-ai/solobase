package storage

import (
	"context"
	"errors"
	"io"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

// NoopProvider implements storage without actual storage (for WASM environments)
type NoopProvider struct{}

// NewNoopProvider creates a new no-op storage provider
func NewNoopProvider() *NoopProvider {
	return &NoopProvider{}
}

// Name returns the provider name
func (n *NoopProvider) Name() string {
	return "No-op Storage"
}

// Type returns the provider type
func (n *NoopProvider) Type() ProviderType {
	return ProviderLocal // Treat as local, but non-functional
}

// CreateBucket is a no-op
func (n *NoopProvider) CreateBucket(ctx context.Context, name string, opts CreateBucketOptions) error {
	return nil // Silently succeed
}

// DeleteBucket is a no-op
func (n *NoopProvider) DeleteBucket(ctx context.Context, name string) error {
	return nil
}

// BucketExists always returns true to prevent errors
func (n *NoopProvider) BucketExists(ctx context.Context, name string) (bool, error) {
	return true, nil
}

// ListBuckets returns empty list
func (n *NoopProvider) ListBuckets(ctx context.Context) ([]BucketInfo, error) {
	return []BucketInfo{}, nil
}

// PutObject is a no-op
func (n *NoopProvider) PutObject(ctx context.Context, bucket, key string, reader io.Reader, size int64, opts PutObjectOptions) error {
	// Drain the reader
	io.Copy(io.Discard, reader)
	return nil
}

// GetObject returns not found
func (n *NoopProvider) GetObject(ctx context.Context, bucket, key string) (io.ReadCloser, error) {
	return nil, errors.New("storage not available")
}

// GetObjectInfo returns not found
func (n *NoopProvider) GetObjectInfo(ctx context.Context, bucket, key string) (*ObjectInfo, error) {
	return nil, errors.New("storage not available")
}

// DeleteObject is a no-op
func (n *NoopProvider) DeleteObject(ctx context.Context, bucket, key string) error {
	return nil
}

// ListObjects returns empty list
func (n *NoopProvider) ListObjects(ctx context.Context, bucket, prefix string, opts ListObjectsOptions) ([]ObjectInfo, error) {
	return []ObjectInfo{}, nil
}

// GeneratePresignedURL returns an error
func (n *NoopProvider) GeneratePresignedURL(ctx context.Context, bucket, key string, expires apptime.Duration) (string, error) {
	return "", errors.New("storage not available")
}
