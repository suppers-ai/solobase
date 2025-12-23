//go:build wasm

// Package wasm provides a storage adapter that uses WIT-imported storage interface.
// The host runtime provides the actual storage implementation.
package wasm

import (
	"bytes"
	"context"
	"fmt"
	"io"

	"go.bytecodealliance.org/cm"

	"github.com/suppers-ai/solobase/builds/wasm/gen/solobase/core/storage"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	pkgstorage "github.com/suppers-ai/solobase/internal/pkg/storage"
)

// Provider implements storage.Provider using WIT storage imports
type Provider struct {
	name string
}

// Ensure Provider implements storage.Provider
var _ pkgstorage.Provider = (*Provider)(nil)

// New creates a new WASM storage provider
func New() *Provider {
	return &Provider{
		name: "wasm",
	}
}

// Name returns the provider name
func (p *Provider) Name() string {
	return p.name
}

// Type returns the provider type
func (p *Provider) Type() pkgstorage.ProviderType {
	return "wasm"
}

// CreateBucket creates a new storage bucket
func (p *Provider) CreateBucket(ctx context.Context, name string, opts pkgstorage.CreateBucketOptions) error {
	result := storage.CreateBucket(name)
	if result.IsErr() {
		err := result.Err()
		return fmt.Errorf("create bucket failed: %s: %s", err.Code, err.Message)
	}
	return nil
}

// DeleteBucket deletes a storage bucket
func (p *Provider) DeleteBucket(ctx context.Context, name string) error {
	// WIT interface doesn't support delete bucket
	return fmt.Errorf("DeleteBucket not supported in WASM")
}

// BucketExists checks if a bucket exists
func (p *Provider) BucketExists(ctx context.Context, name string) (bool, error) {
	result := storage.BucketExists(name)
	if result.IsErr() {
		err := result.Err()
		return false, fmt.Errorf("bucket exists check failed: %s: %s", err.Code, err.Message)
	}
	return *result.OK(), nil
}

// ListBuckets lists all buckets
func (p *Provider) ListBuckets(ctx context.Context) ([]pkgstorage.BucketInfo, error) {
	// WIT interface doesn't support list buckets
	return nil, fmt.Errorf("ListBuckets not supported in WASM")
}

// PutObject uploads an object to storage
func (p *Provider) PutObject(ctx context.Context, bucket, key string, reader io.Reader, size int64, opts pkgstorage.PutObjectOptions) error {
	// Read all data from reader
	data, err := io.ReadAll(reader)
	if err != nil {
		return fmt.Errorf("failed to read data: %w", err)
	}

	// Convert content type to Option
	var contentType cm.Option[string]
	if opts.ContentType != "" {
		contentType = cm.Some(opts.ContentType)
	} else {
		contentType = cm.None[string]()
	}

	result := storage.PutObject(bucket, key, cm.ToList(data), contentType)
	if result.IsErr() {
		storageErr := result.Err()
		return fmt.Errorf("put object failed: %s: %s", storageErr.Code, storageErr.Message)
	}
	return nil
}

// GetObject downloads an object from storage
func (p *Provider) GetObject(ctx context.Context, bucket, key string) (io.ReadCloser, error) {
	result := storage.GetObject(bucket, key)
	if result.IsErr() {
		err := result.Err()
		return nil, fmt.Errorf("get object failed: %s: %s", err.Code, err.Message)
	}
	data := result.OK().Slice()
	return io.NopCloser(bytes.NewReader(data)), nil
}

// GetObjectInfo gets object metadata without downloading
func (p *Provider) GetObjectInfo(ctx context.Context, bucket, key string) (*pkgstorage.ObjectInfo, error) {
	result := storage.HeadObject(bucket, key)
	if result.IsErr() {
		err := result.Err()
		return nil, fmt.Errorf("head object failed: %s: %s", err.Code, err.Message)
	}
	info := result.OK()

	objectInfo := &pkgstorage.ObjectInfo{
		Key:  info.Key,
		Size: int64(info.Size),
	}

	// Handle optional fields
	if ct := info.ContentType.Some(); ct != nil {
		objectInfo.ContentType = *ct
	}
	if etag := info.Etag.Some(); etag != nil {
		objectInfo.ETag = *etag
	}
	if lm := info.LastModified.Some(); lm != nil {
		objectInfo.LastModified = apptime.Unix(int64(*lm), 0)
	}

	return objectInfo, nil
}

// DeleteObject deletes an object from storage
func (p *Provider) DeleteObject(ctx context.Context, bucket, key string) error {
	result := storage.DeleteObject(bucket, key)
	if result.IsErr() {
		err := result.Err()
		return fmt.Errorf("delete object failed: %s: %s", err.Code, err.Message)
	}
	return nil
}

// ListObjects lists objects in a bucket with optional prefix
func (p *Provider) ListObjects(ctx context.Context, bucket, prefix string, opts pkgstorage.ListObjectsOptions) ([]pkgstorage.ObjectInfo, error) {
	// Convert prefix to Option
	var prefixOpt cm.Option[string]
	if prefix != "" {
		prefixOpt = cm.Some(prefix)
	} else {
		prefixOpt = cm.None[string]()
	}

	result := storage.ListObjects(bucket, prefixOpt)
	if result.IsErr() {
		err := result.Err()
		return nil, fmt.Errorf("list objects failed: %s: %s", err.Code, err.Message)
	}

	witObjects := result.OK().Slice()
	objects := make([]pkgstorage.ObjectInfo, len(witObjects))
	for i, obj := range witObjects {
		objects[i] = pkgstorage.ObjectInfo{
			Key:  obj.Key,
			Size: int64(obj.Size),
		}
		if ct := obj.ContentType.Some(); ct != nil {
			objects[i].ContentType = *ct
		}
		if etag := obj.Etag.Some(); etag != nil {
			objects[i].ETag = *etag
		}
		if lm := obj.LastModified.Some(); lm != nil {
			objects[i].LastModified = apptime.Unix(int64(*lm), 0)
		}
	}

	return objects, nil
}

// GeneratePresignedURL generates a presigned URL for object access
func (p *Provider) GeneratePresignedURL(ctx context.Context, bucket, key string, expires apptime.Duration) (string, error) {
	// WIT interface doesn't support presigned URLs
	return "", fmt.Errorf("GeneratePresignedURL not supported in WASM")
}
