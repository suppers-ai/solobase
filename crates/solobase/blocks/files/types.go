package files

import (
	"errors"

	"github.com/suppers-ai/solobase/core/apptime"
)

// Common errors returned by storage operations
var (
	ErrNotFound            = errors.New("not found")
	ErrDuplicate           = errors.New("duplicate entry")
	ErrConstraintViolation = errors.New("constraint violation")
)

// ListObjectsOptions configures object listing
type ListObjectsOptions struct {
	BucketName     string
	UserID         *string
	AppID          *string
	ParentFolderID *string
	ContentType    *string
	Limit          int
	Offset         int
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

// BucketStats contains aggregated statistics for a bucket
type BucketStats struct {
	BucketName  string
	ObjectCount int64
	TotalSize   int64
}
