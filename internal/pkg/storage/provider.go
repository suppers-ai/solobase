package storage

import (
	"context"
	"io"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

// Provider defines the interface for storage backends
type Provider interface {
	// Bucket operations
	CreateBucket(ctx context.Context, name string, opts CreateBucketOptions) error
	DeleteBucket(ctx context.Context, name string) error
	BucketExists(ctx context.Context, name string) (bool, error)
	ListBuckets(ctx context.Context) ([]BucketInfo, error)

	// Object operations
	PutObject(ctx context.Context, bucket, key string, reader io.Reader, size int64, opts PutObjectOptions) error
	GetObject(ctx context.Context, bucket, key string) (io.ReadCloser, error)
	GetObjectInfo(ctx context.Context, bucket, key string) (*ObjectInfo, error)
	DeleteObject(ctx context.Context, bucket, key string) error
	ListObjects(ctx context.Context, bucket, prefix string, opts ListObjectsOptions) ([]ObjectInfo, error)

	// URL operations
	GeneratePresignedURL(ctx context.Context, bucket, key string, expires apptime.Duration) (string, error)

	// Provider info
	Name() string
	Type() ProviderType
}

// ProviderType represents the type of storage provider
type ProviderType string

const (
	ProviderLocal ProviderType = "local"
	ProviderS3    ProviderType = "s3"
	ProviderGCS   ProviderType = "gcs"
	ProviderAzure ProviderType = "azure"
)

// CreateBucketOptions contains options for creating a bucket
type CreateBucketOptions struct {
	Public           bool
	Region           string
	Versioning       bool
	FileSizeLimit    int64
	AllowedMimeTypes []string
}

// PutObjectOptions contains options for uploading an object
type PutObjectOptions struct {
	ContentType     string
	ContentEncoding string
	Metadata        map[string]string
	CacheControl    string
	Public          bool
}

// ListObjectsOptions contains options for listing objects
type ListObjectsOptions struct {
	MaxKeys   int
	Delimiter string
	Marker    string
	Recursive bool
}

// BucketInfo contains information about a bucket
type BucketInfo struct {
	Name        string
	CreatedAt   apptime.Time
	Public      bool
	Region      string
	ObjectCount int64
	TotalSize   int64
}

// ObjectInfo contains information about an object
type ObjectInfo struct {
	Key          string
	Size         int64
	ETag         string
	ContentType  string
	LastModified apptime.Time
	Metadata     map[string]string
	IsDir        bool
}

// Config contains configuration for storage providers
type Config struct {
	// Common settings
	Provider      ProviderType
	DefaultBucket string
	BasePath      string // For local storage
	BaseURL       string // For URL generation

	// S3 settings
	S3Endpoint        string
	S3Region          string
	S3AccessKeyID     string
	S3SecretAccessKey string
	S3BucketPrefix    string
	S3UseSSL          bool
	S3PathStyle       bool // For MinIO compatibility

	// GCS settings
	GCSProjectID       string
	GCSCredentialsPath string

	// Azure settings
	AzureAccountName string
	AzureAccountKey  string
	AzureContainer   string
}

// NewProvider is implemented in provider_standard.go and provider_wasm.go
