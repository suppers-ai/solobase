package models

import (
	"time"

	"github.com/google/uuid"
)

// StorageDownloadToken represents a temporary token for file downloads
type StorageDownloadToken struct {
	ID             string     `gorm:"primaryKey;type:uuid" json:"id"`
	Token          string     `gorm:"uniqueIndex;not null" json:"token"`
	FileID         string     `gorm:"not null" json:"fileId"`
	Bucket         string     `gorm:"not null" json:"bucket"`
	ParentFolderID *string    `json:"parentFolderId,omitempty"`  // Parent folder ID (null for root)
	ObjectName     string     `gorm:"not null" json:"objectName"` // The file name
	UserID         string     `gorm:"type:uuid" json:"userId"`
	FileSize       int64      `json:"fileSize"`
	BytesServed    int64      `gorm:"default:0" json:"bytesServed"`
	Completed      bool       `gorm:"default:false" json:"completed"`
	ExpiresAt      time.Time  `gorm:"not null" json:"expiresAt"`
	CreatedAt      time.Time  `json:"createdAt"`
	CallbackAt     *time.Time `json:"callbackAt,omitempty"`
	ClientIP       string     `json:"clientIp,omitempty"`
}

// TableName sets the table name
func (StorageDownloadToken) TableName() string {
	return "storage_download_tokens"
}

// NewStorageDownloadToken creates a new download token
func NewStorageDownloadToken(fileID, bucket string, parentFolderID *string, objectName, userID string, fileSize int64, duration time.Duration) *StorageDownloadToken {
	return &StorageDownloadToken{
		ID:             uuid.New().String(),
		Token:          uuid.New().String(), // Simple UUID token for now
		FileID:         fileID,
		Bucket:         bucket,
		ParentFolderID: parentFolderID,
		ObjectName:     objectName,
		UserID:         userID,
		FileSize:       fileSize,
		ExpiresAt:      time.Now().Add(duration),
		CreatedAt:      time.Now(),
	}
}

// IsExpired checks if the token has expired
func (dt *StorageDownloadToken) IsExpired() bool {
	return time.Now().After(dt.ExpiresAt)
}

// IsValid checks if the token is valid for use
func (dt *StorageDownloadToken) IsValid() bool {
	return !dt.IsExpired() && !dt.Completed
}
