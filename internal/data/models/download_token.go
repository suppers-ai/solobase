package models

import (
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
)

// StorageDownloadToken represents a temporary token for file downloads
type StorageDownloadToken struct {
	ID             string           `json:"id"`
	Token          string           `json:"token"`
	FileID         string           `json:"fileId"`
	Bucket         string           `json:"bucket"`
	ParentFolderID *string          `json:"parentFolderId,omitempty"` // Parent folder ID (null for root)
	ObjectName     string           `json:"objectName"`               // The file name
	UserID         string           `json:"userId"`
	FileSize       int64            `json:"fileSize"`
	BytesServed    int64            `json:"bytesServed"`
	Completed      bool             `json:"completed"`
	ExpiresAt      apptime.Time     `json:"expiresAt"`
	CreatedAt      apptime.Time     `json:"createdAt"`
	CallbackAt     apptime.NullTime `json:"callbackAt,omitempty"`
	ClientIP       string           `json:"clientIp,omitempty"`
}

// TableName sets the table name
func (StorageDownloadToken) TableName() string {
	return "storage_download_tokens"
}

// NewStorageDownloadToken creates a new download token
func NewStorageDownloadToken(fileID, bucket string, parentFolderID *string, objectName, userID string, fileSize int64, duration apptime.Duration) *StorageDownloadToken {
	return &StorageDownloadToken{
		ID:             uuid.New().String(),
		Token:          uuid.New().String(), // Simple UUID token for now
		FileID:         fileID,
		Bucket:         bucket,
		ParentFolderID: parentFolderID,
		ObjectName:     objectName,
		UserID:         userID,
		FileSize:       fileSize,
		ExpiresAt:      apptime.NowTime().Add(duration),
		CreatedAt:      apptime.NowTime(),
	}
}

// IsExpired checks if the token has expired
func (dt *StorageDownloadToken) IsExpired() bool {
	return apptime.NowTime().After(dt.ExpiresAt)
}

// IsValid checks if the token is valid for use
func (dt *StorageDownloadToken) IsValid() bool {
	return !dt.IsExpired() && !dt.Completed
}
