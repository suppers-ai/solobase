package repos

import (
	"context"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/storage"
)

// ListObjectsOptions configures object listing
type ListObjectsOptions struct {
	BucketName     string
	UserID         *string
	AppID          *string
	ParentFolderID *string
	ContentType    *string // filter by content type
	Pagination
}

// UploadToken represents an upload token
type UploadToken struct {
	ID             string
	Token          string
	Bucket         string
	ParentFolderID *string
	ObjectName     string
	UserID         *string
	MaxSize        *int64
	ContentType    *string
	BytesUploaded  *int64
	Completed      bool
	ObjectID       *string
	ExpiresAt      apptime.NullTime
	CreatedAt      apptime.Time
	CompletedAt    apptime.NullTime
	ClientIP       *string
}

// DownloadToken represents a download token
type DownloadToken struct {
	ID             string
	Token          string
	FileID         string
	Bucket         string
	ParentFolderID *string
	ObjectName     string
	UserID         *string
	FileSize       *int64
	BytesServed    *int64
	Completed      bool
	ExpiresAt      apptime.NullTime
	CreatedAt      apptime.Time
	CallbackAt     apptime.NullTime
	ClientIP       *string
}

// StorageRepository provides storage metadata operations
// NOTE: Actual file I/O still uses storage.Provider
type StorageRepository interface {
	// Buckets
	CreateBucket(ctx context.Context, bucket *storage.StorageBucket) error
	GetBucket(ctx context.Context, id string) (*storage.StorageBucket, error)
	GetBucketByName(ctx context.Context, name string) (*storage.StorageBucket, error)
	ListBuckets(ctx context.Context) ([]*storage.StorageBucket, error)
	UpdateBucket(ctx context.Context, bucket *storage.StorageBucket) error
	DeleteBucket(ctx context.Context, id string) error

	// Objects
	CreateObject(ctx context.Context, obj *storage.StorageObject) error
	GetObject(ctx context.Context, id string) (*storage.StorageObject, error)
	GetObjectByPath(ctx context.Context, bucketName, objectName string, parentFolderID *string) (*storage.StorageObject, error)
	GetObjectByChecksum(ctx context.Context, checksum string) (*storage.StorageObject, error)
	UpdateObject(ctx context.Context, obj *storage.StorageObject) error
	UpdateObjectLastViewed(ctx context.Context, id string, lastViewed apptime.Time) error
	DeleteObject(ctx context.Context, id string) error
	DeleteObjectsByBucket(ctx context.Context, bucketName string) error
	DeleteObjectsByParentFolder(ctx context.Context, parentFolderID string) error

	// Object queries
	ListObjects(ctx context.Context, opts ListObjectsOptions) ([]*storage.StorageObject, error)
	ListRecentlyViewed(ctx context.Context, userID string, limit int) ([]*storage.StorageObject, error)
	SearchObjects(ctx context.Context, userID, appID, searchPattern string, limit int) ([]*storage.StorageObject, error)
	CountObjectsByBucket(ctx context.Context, bucketName string) (int64, error)
	CountObjectsByUser(ctx context.Context, userID string) (int64, error)
	SumSizeByBucket(ctx context.Context, bucketName string) (int64, error)
	SumSizeByUser(ctx context.Context, userID string) (int64, error)
	SumTotalSize(ctx context.Context) (int64, error)

	// Upload tokens
	CreateUploadToken(ctx context.Context, token *UploadToken) error
	GetUploadToken(ctx context.Context, id string) (*UploadToken, error)
	GetUploadTokenByToken(ctx context.Context, token string) (*UploadToken, error)
	UpdateUploadTokenProgress(ctx context.Context, id string, bytesUploaded int64) error
	CompleteUploadToken(ctx context.Context, id, objectID string) error
	DeleteUploadToken(ctx context.Context, id string) error
	DeleteExpiredUploadTokens(ctx context.Context) error

	// Download tokens
	CreateDownloadToken(ctx context.Context, token *DownloadToken) error
	GetDownloadToken(ctx context.Context, id string) (*DownloadToken, error)
	GetDownloadTokenByToken(ctx context.Context, token string) (*DownloadToken, error)
	UpdateDownloadTokenProgress(ctx context.Context, id string, bytesServed int64) error
	CompleteDownloadToken(ctx context.Context, id string) error
	DeleteDownloadToken(ctx context.Context, id string) error
	DeleteExpiredDownloadTokens(ctx context.Context) error
}
