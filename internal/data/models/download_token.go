package models

import (
	"time"

	"github.com/google/uuid"
)

// StorageDownloadToken represents a temporary token for file downloads
type StorageDownloadToken struct {
	ID             string     `gorm:"primaryKey;type:uuid" json:"id"`
	Token          string     `gorm:"uniqueIndex;not null" json:"token"`
	FileID         string     `gorm:"not null" json:"file_id"`
	Bucket         string     `gorm:"not null" json:"bucket"`
	ParentFolderID *string    `json:"parent_folder_id,omitempty"`  // Parent folder ID (null for root)
	ObjectName     string     `gorm:"not null" json:"object_name"` // The file name
	UserID         string     `gorm:"type:uuid" json:"user_id"`
	FileSize       int64      `json:"file_size"`
	BytesServed    int64      `gorm:"default:0" json:"bytes_served"`
	Completed      bool       `gorm:"default:false" json:"completed"`
	ExpiresAt      time.Time  `gorm:"not null" json:"expires_at"`
	CreatedAt      time.Time  `json:"created_at"`
	CallbackAt     *time.Time `json:"callback_at,omitempty"`
	ClientIP       string     `json:"client_ip,omitempty"`
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
