package interfaces

import (
	"context"
	"io"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

// Storage defines the interface for object storage operations.
// Implementations:
//   - Standard: Local filesystem, AWS S3 (SDK), GCS (SDK)
//   - WASM: HTTP-based S3/GCS (via Spin outbound HTTP)
type Storage interface {
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
	Type() StorageType
}

// StorageType represents the type of storage provider
type StorageType string

const (
	StorageLocal StorageType = "local"
	StorageS3    StorageType = "s3"
	StorageGCS   StorageType = "gcs"
	StorageAzure StorageType = "azure"
	StorageHTTP  StorageType = "http" // For WASM - HTTP-based S3/GCS
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

// StorageConfig contains configuration for storage providers
type StorageConfig struct {
	// Common settings
	Provider      StorageType
	DefaultBucket string
	BasePath      string // For local storage
	BaseURL       string // For URL generation

	// S3 settings (also works for MinIO, GCS S3 compatibility)
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
	GCSCredentialsJSON string // For WASM - inline credentials

	// Azure settings
	AzureAccountName string
	AzureAccountKey  string
	AzureContainer   string
}
