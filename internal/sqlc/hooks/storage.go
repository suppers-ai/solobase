package hooks

import (
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
)

// BucketBeforeCreate prepares a bucket for creation
func BucketBeforeCreate(params *db.CreateBucketParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}
	// Timestamps are handled by database defaults (CURRENT_TIMESTAMP)
	return nil
}

// ObjectBeforeCreate prepares a storage object for creation
func ObjectBeforeCreate(params *db.CreateObjectParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}
	// Timestamps are handled by database defaults (CURRENT_TIMESTAMP)
	return nil
}

// UploadTokenBeforeCreate prepares an upload token for creation
func UploadTokenBeforeCreate(params *db.CreateUploadTokenParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}
	params.CreatedAt = apptime.NowString()
	return nil
}

// DownloadTokenBeforeCreate prepares a download token for creation
func DownloadTokenBeforeCreate(params *db.CreateDownloadTokenParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}
	params.CreatedAt = apptime.NowString()
	return nil
}
