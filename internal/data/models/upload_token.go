package models

import (
	"time"

	"github.com/google/uuid"
)

// StorageUploadToken represents a temporary token for file uploads
type StorageUploadToken struct {
	ID             string     `gorm:"primaryKey;type:uuid" json:"id"`
	Token          string     `gorm:"uniqueIndex;not null" json:"token"`
	Bucket         string     `gorm:"not null" json:"bucket"`
	ParentFolderID *string    `json:"parentFolderId,omitempty"`  // Parent folder ID (null for root)
	ObjectName     string     `gorm:"not null" json:"objectName"` // The file name
	UserID         string     `gorm:"type:uuid" json:"userId"`
	MaxSize        int64      `json:"maxSize"`     // Maximum allowed file size
	ContentType    string     `json:"contentType"` // Expected content type
	BytesUploaded  int64      `gorm:"default:0" json:"bytesUploaded"`
	Completed      bool       `gorm:"default:false" json:"completed"`
	ObjectID       string     `json:"objectId,omitempty"` // ID of created storage object
	ExpiresAt      time.Time  `gorm:"not null" json:"expiresAt"`
	CreatedAt      time.Time  `json:"createdAt"`
	CompletedAt    *time.Time `json:"completedAt,omitempty"`
	ClientIP       string     `json:"clientIp,omitempty"`
}

// TableName sets the table name
func (StorageUploadToken) TableName() string {
	return "storage_upload_tokens"
}

// NewStorageUploadToken creates a new upload token
func NewStorageUploadToken(bucket string, parentFolderID *string, objectName, userID, contentType string, maxSize int64, duration time.Duration) *StorageUploadToken {
	return &StorageUploadToken{
		ID:             uuid.New().String(),
		Token:          uuid.New().String(),
		Bucket:         bucket,
		ParentFolderID: parentFolderID,
		ObjectName:     objectName,
		UserID:         userID,
		ContentType:    contentType,
		MaxSize:        maxSize,
		ExpiresAt:      time.Now().Add(duration),
		CreatedAt:      time.Now(),
	}
}

// IsExpired checks if the token has expired
func (ut *StorageUploadToken) IsExpired() bool {
	return time.Now().After(ut.ExpiresAt)
}

// IsValid checks if the token is valid for use
func (ut *StorageUploadToken) IsValid() bool {
	return !ut.IsExpired() && !ut.Completed
}

// CanAcceptBytes checks if the upload can accept more bytes
func (ut *StorageUploadToken) CanAcceptBytes(additionalBytes int64) bool {
	return ut.BytesUploaded+additionalBytes <= ut.MaxSize
}
