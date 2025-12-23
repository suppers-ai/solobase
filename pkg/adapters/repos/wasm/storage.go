//go:build wasm

package wasm

import (
	"context"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/storage"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type storageRepository struct{}

// Bucket operations

func (r *storageRepository) CreateBucket(ctx context.Context, bucket *storage.StorageBucket) error {
	return ErrNotImplemented
}

func (r *storageRepository) GetBucket(ctx context.Context, id string) (*storage.StorageBucket, error) {
	return nil, ErrNotImplemented
}

func (r *storageRepository) GetBucketByName(ctx context.Context, name string) (*storage.StorageBucket, error) {
	return nil, ErrNotImplemented
}

func (r *storageRepository) ListBuckets(ctx context.Context) ([]*storage.StorageBucket, error) {
	return nil, ErrNotImplemented
}

func (r *storageRepository) UpdateBucket(ctx context.Context, bucket *storage.StorageBucket) error {
	return ErrNotImplemented
}

func (r *storageRepository) DeleteBucket(ctx context.Context, id string) error {
	return ErrNotImplemented
}

// Object operations

func (r *storageRepository) CreateObject(ctx context.Context, obj *storage.StorageObject) error {
	return ErrNotImplemented
}

func (r *storageRepository) GetObject(ctx context.Context, id string) (*storage.StorageObject, error) {
	return nil, ErrNotImplemented
}

func (r *storageRepository) GetObjectByPath(ctx context.Context, bucketName, objectName string, parentFolderID *string) (*storage.StorageObject, error) {
	return nil, ErrNotImplemented
}

func (r *storageRepository) GetObjectByChecksum(ctx context.Context, checksum string) (*storage.StorageObject, error) {
	return nil, ErrNotImplemented
}

func (r *storageRepository) UpdateObject(ctx context.Context, obj *storage.StorageObject) error {
	return ErrNotImplemented
}

func (r *storageRepository) UpdateObjectLastViewed(ctx context.Context, id string, lastViewed apptime.Time) error {
	return ErrNotImplemented
}

func (r *storageRepository) DeleteObject(ctx context.Context, id string) error {
	return ErrNotImplemented
}

func (r *storageRepository) DeleteObjectsByBucket(ctx context.Context, bucketName string) error {
	return ErrNotImplemented
}

func (r *storageRepository) DeleteObjectsByParentFolder(ctx context.Context, parentFolderID string) error {
	return ErrNotImplemented
}

// Object queries

func (r *storageRepository) ListObjects(ctx context.Context, opts repos.ListObjectsOptions) ([]*storage.StorageObject, error) {
	return nil, ErrNotImplemented
}

func (r *storageRepository) ListRecentlyViewed(ctx context.Context, userID string, limit int) ([]*storage.StorageObject, error) {
	return nil, ErrNotImplemented
}

func (r *storageRepository) SearchObjects(ctx context.Context, userID, appID, searchPattern string, limit int) ([]*storage.StorageObject, error) {
	return nil, ErrNotImplemented
}

func (r *storageRepository) CountObjectsByBucket(ctx context.Context, bucketName string) (int64, error) {
	return 0, ErrNotImplemented
}

func (r *storageRepository) CountObjectsByUser(ctx context.Context, userID string) (int64, error) {
	return 0, ErrNotImplemented
}

func (r *storageRepository) SumSizeByBucket(ctx context.Context, bucketName string) (int64, error) {
	return 0, ErrNotImplemented
}

func (r *storageRepository) SumSizeByUser(ctx context.Context, userID string) (int64, error) {
	return 0, ErrNotImplemented
}

func (r *storageRepository) SumTotalSize(ctx context.Context) (int64, error) {
	return 0, ErrNotImplemented
}

// Upload token operations

func (r *storageRepository) CreateUploadToken(ctx context.Context, token *repos.UploadToken) error {
	return ErrNotImplemented
}

func (r *storageRepository) GetUploadToken(ctx context.Context, id string) (*repos.UploadToken, error) {
	return nil, ErrNotImplemented
}

func (r *storageRepository) GetUploadTokenByToken(ctx context.Context, token string) (*repos.UploadToken, error) {
	return nil, ErrNotImplemented
}

func (r *storageRepository) UpdateUploadTokenProgress(ctx context.Context, id string, bytesUploaded int64) error {
	return ErrNotImplemented
}

func (r *storageRepository) CompleteUploadToken(ctx context.Context, id, objectID string) error {
	return ErrNotImplemented
}

func (r *storageRepository) DeleteUploadToken(ctx context.Context, id string) error {
	return ErrNotImplemented
}

func (r *storageRepository) DeleteExpiredUploadTokens(ctx context.Context) error {
	return ErrNotImplemented
}

// Download token operations

func (r *storageRepository) CreateDownloadToken(ctx context.Context, token *repos.DownloadToken) error {
	return ErrNotImplemented
}

func (r *storageRepository) GetDownloadToken(ctx context.Context, id string) (*repos.DownloadToken, error) {
	return nil, ErrNotImplemented
}

func (r *storageRepository) GetDownloadTokenByToken(ctx context.Context, token string) (*repos.DownloadToken, error) {
	return nil, ErrNotImplemented
}

func (r *storageRepository) UpdateDownloadTokenProgress(ctx context.Context, id string, bytesServed int64) error {
	return ErrNotImplemented
}

func (r *storageRepository) CompleteDownloadToken(ctx context.Context, id string) error {
	return ErrNotImplemented
}

func (r *storageRepository) DeleteDownloadToken(ctx context.Context, id string) error {
	return ErrNotImplemented
}

func (r *storageRepository) DeleteExpiredDownloadTokens(ctx context.Context) error {
	return ErrNotImplemented
}

// Ensure storageRepository implements StorageRepository
var _ repos.StorageRepository = (*storageRepository)(nil)
