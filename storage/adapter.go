// Package storage provides adapters to use the github.com/suppers-ai/solobase/internal/pkg/storage package
package storage

import (
	"context"
	"io"
	"time"

	pkgstorage "github.com/suppers-ai/solobase/internal/pkg/storage"
)

// Object represents a stored object (adapter for package storage)
type Object struct {
	Key          string    `json:"key"`
	Size         int64     `json:"size"`
	ContentType  string    `json:"content_type"`
	LastModified time.Time `json:"last_modified"`
	ETag         string    `json:"etag,omitempty"`
	IsDirectory  bool      `json:"is_directory"`
}

// Bucket represents a storage bucket (adapter for package storage)
type Bucket struct {
	Name        string    `json:"name"`
	CreatedAt   time.Time `json:"created_at"`
	Public      bool      `json:"public"`
	ObjectCount int64     `json:"object_count"`
	TotalSize   int64     `json:"total_size"`
}

// Provider defines the storage provider interface (adapter for package storage)
type Provider interface {
	// Bucket operations
	CreateBucket(name string, public bool) error
	DeleteBucket(name string) error
	ListBuckets() ([]Bucket, error)
	BucketExists(name string) (bool, error)

	// Object operations
	PutObject(bucket, key string, reader io.Reader, size int64, contentType string) error
	GetObject(bucket, key string) (io.ReadCloser, error)
	DeleteObject(bucket, key string) error
	ListObjects(bucket, prefix string) ([]Object, error)
	ObjectExists(bucket, key string) (bool, error)
	GetObjectInfo(bucket, key string) (*Object, error)

	// URL generation
	GetPublicURL(bucket, key string) string
	GetSignedURL(bucket, key string, expiry time.Duration) (string, error)
}

// Storage wraps a storage provider
type Storage struct {
	provider Provider
}

// New creates a new storage instance
func New(provider Provider) *Storage {
	return &Storage{provider: provider}
}

// CreateBucket creates a new bucket
func (s *Storage) CreateBucket(name string, public bool) error {
	return s.provider.CreateBucket(name, public)
}

// DeleteBucket deletes a bucket
func (s *Storage) DeleteBucket(name string) error {
	return s.provider.DeleteBucket(name)
}

// ListBuckets lists all buckets
func (s *Storage) ListBuckets() ([]Bucket, error) {
	return s.provider.ListBuckets()
}

// PutObject stores an object
func (s *Storage) PutObject(bucket, key string, reader io.Reader, size int64, contentType string) error {
	return s.provider.PutObject(bucket, key, reader, size, contentType)
}

// GetObject retrieves an object
func (s *Storage) GetObject(bucket, key string) (io.ReadCloser, error) {
	return s.provider.GetObject(bucket, key)
}

// DeleteObject deletes an object
func (s *Storage) DeleteObject(bucket, key string) error {
	return s.provider.DeleteObject(bucket, key)
}

// ListObjects lists objects in a bucket
func (s *Storage) ListObjects(bucket, prefix string) ([]Object, error) {
	return s.provider.ListObjects(bucket, prefix)
}

// GetObjectInfo gets object metadata
func (s *Storage) GetObjectInfo(bucket, key string) (*Object, error) {
	return s.provider.GetObjectInfo(bucket, key)
}

// GetPublicURL gets the public URL for an object
func (s *Storage) GetPublicURL(bucket, key string) string {
	return s.provider.GetPublicURL(bucket, key)
}

// GetSignedURL generates a signed URL
func (s *Storage) GetSignedURL(bucket, key string, expiry time.Duration) (string, error) {
	return s.provider.GetSignedURL(bucket, key, expiry)
}

// providerAdapter adapts the package storage Provider to our interface
type providerAdapter struct {
	provider pkgstorage.Provider
	ctx      context.Context
}

// NewS3Provider creates an S3 storage provider using the package
func NewS3Provider(endpoint, accessKey, secretKey, region string, useSSL bool) (Provider, error) {
	cfg := pkgstorage.Config{
		Provider:          pkgstorage.ProviderS3,
		S3Endpoint:        endpoint,
		S3AccessKeyID:     accessKey,
		S3SecretAccessKey: secretKey,
		S3Region:          region,
		S3UseSSL:          useSSL,
	}

	provider, err := pkgstorage.NewProvider(cfg)
	if err != nil {
		return nil, err
	}

	return &providerAdapter{
		provider: provider,
		ctx:      context.Background(),
	}, nil
}

// NewLocalProvider creates a local storage provider using the package
func NewLocalProvider(basePath string) (Provider, error) {
	cfg := pkgstorage.Config{
		Provider: pkgstorage.ProviderLocal,
		BasePath: basePath,
	}

	provider, err := pkgstorage.NewProvider(cfg)
	if err != nil {
		return nil, err
	}

	return &providerAdapter{
		provider: provider,
		ctx:      context.Background(),
	}, nil
}

// Implement Provider interface methods
func (p *providerAdapter) CreateBucket(name string, public bool) error {
	return p.provider.CreateBucket(p.ctx, name, pkgstorage.CreateBucketOptions{
		Public: public,
	})
}

func (p *providerAdapter) DeleteBucket(name string) error {
	return p.provider.DeleteBucket(p.ctx, name)
}

func (p *providerAdapter) ListBuckets() ([]Bucket, error) {
	buckets, err := p.provider.ListBuckets(p.ctx)
	if err != nil {
		return nil, err
	}

	result := make([]Bucket, len(buckets))
	for i, b := range buckets {
		result[i] = Bucket{
			Name:        b.Name,
			CreatedAt:   b.CreatedAt,
			Public:      b.Public,
			ObjectCount: b.ObjectCount,
			TotalSize:   b.TotalSize,
		}
	}
	return result, nil
}

func (p *providerAdapter) BucketExists(name string) (bool, error) {
	return p.provider.BucketExists(p.ctx, name)
}

func (p *providerAdapter) PutObject(bucket, key string, reader io.Reader, size int64, contentType string) error {
	return p.provider.PutObject(p.ctx, bucket, key, reader, size, pkgstorage.PutObjectOptions{
		ContentType: contentType,
	})
}

func (p *providerAdapter) GetObject(bucket, key string) (io.ReadCloser, error) {
	return p.provider.GetObject(p.ctx, bucket, key)
}

func (p *providerAdapter) DeleteObject(bucket, key string) error {
	return p.provider.DeleteObject(p.ctx, bucket, key)
}

func (p *providerAdapter) ListObjects(bucket, prefix string) ([]Object, error) {
	objects, err := p.provider.ListObjects(p.ctx, bucket, prefix, pkgstorage.ListObjectsOptions{})
	if err != nil {
		return nil, err
	}

	result := make([]Object, len(objects))
	for i, o := range objects {
		result[i] = Object{
			Key:          o.Key,
			Size:         o.Size,
			ContentType:  o.ContentType,
			LastModified: o.LastModified,
			ETag:         o.ETag,
			IsDirectory:  o.IsDir,
		}
	}
	return result, nil
}

func (p *providerAdapter) ObjectExists(bucket, key string) (bool, error) {
	_, err := p.provider.GetObjectInfo(p.ctx, bucket, key)
	if err != nil {
		return false, nil
	}
	return true, nil
}

func (p *providerAdapter) GetObjectInfo(bucket, key string) (*Object, error) {
	info, err := p.provider.GetObjectInfo(p.ctx, bucket, key)
	if err != nil {
		return nil, err
	}

	return &Object{
		Key:          info.Key,
		Size:         info.Size,
		ContentType:  info.ContentType,
		LastModified: info.LastModified,
		ETag:         info.ETag,
		IsDirectory:  info.IsDir,
	}, nil
}

func (p *providerAdapter) GetPublicURL(bucket, key string) string {
	// Package storage doesn't have GetPublicURL, we'll construct it
	// This is a simplified implementation
	return "/" + bucket + "/" + key
}

func (p *providerAdapter) GetSignedURL(bucket, key string, expiry time.Duration) (string, error) {
	return p.provider.GeneratePresignedURL(p.ctx, bucket, key, expiry)
}
