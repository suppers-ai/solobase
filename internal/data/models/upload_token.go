package models

import (
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
)

// StorageUploadToken represents a temporary token for file uploads
type StorageUploadToken struct {
	ID             string           `json:"id"`
	Token          string           `json:"token"`
	Bucket         string           `json:"bucket"`
	ParentFolderID *string          `json:"parentFolderId,omitempty"` // Parent folder ID (null for root)
	ObjectName     string           `json:"objectName"`               // The file name
	UserID         string           `json:"userId"`
	MaxSize        int64            `json:"maxSize"`     // Maximum allowed file size
	ContentType    string           `json:"contentType"` // Expected content type
	BytesUploaded  int64            `json:"bytesUploaded"`
	Completed      bool             `json:"completed"`
	ObjectID       string           `json:"objectId,omitempty"` // ID of created storage object
	ExpiresAt      apptime.Time     `json:"expiresAt"`
	CreatedAt      apptime.Time     `json:"createdAt"`
	CompletedAt    apptime.NullTime `json:"completedAt,omitempty"`
	ClientIP       string           `json:"clientIp,omitempty"`
}

// TableName sets the table name
func (StorageUploadToken) TableName() string {
	return "storage_upload_tokens"
}

// NewStorageUploadToken creates a new upload token
func NewStorageUploadToken(bucket string, parentFolderID *string, objectName, userID, contentType string, maxSize int64, duration apptime.Duration) *StorageUploadToken {
	return &StorageUploadToken{
		ID:             uuid.New().String(),
		Token:          uuid.New().String(),
		Bucket:         bucket,
		ParentFolderID: parentFolderID,
		ObjectName:     objectName,
		UserID:         userID,
		ContentType:    contentType,
		MaxSize:        maxSize,
		ExpiresAt:      apptime.NowTime().Add(duration),
		CreatedAt:      apptime.NowTime(),
	}
}

// IsExpired checks if the token has expired
func (ut *StorageUploadToken) IsExpired() bool {
	return apptime.NowTime().After(ut.ExpiresAt)
}

// IsValid checks if the token is valid for use
func (ut *StorageUploadToken) IsValid() bool {
	return !ut.IsExpired() && !ut.Completed
}

// CanAcceptBytes checks if the upload can accept more bytes
func (ut *StorageUploadToken) CanAcceptBytes(additionalBytes int64) bool {
	return ut.BytesUploaded+additionalBytes <= ut.MaxSize
}
