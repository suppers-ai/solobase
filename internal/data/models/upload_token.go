package models

import (
	"time"

	"github.com/google/uuid"
)

// UploadToken represents a temporary token for file uploads
type UploadToken struct {
	ID             string     `gorm:"primaryKey;type:uuid" json:"id"`
	Token          string     `gorm:"uniqueIndex;not null" json:"token"`
	Bucket         string     `gorm:"not null" json:"bucket"`
	ParentFolderID *string    `json:"parent_folder_id,omitempty"`  // Parent folder ID (null for root)
	ObjectName     string     `gorm:"not null" json:"object_name"` // The file name
	UserID         string     `gorm:"type:uuid" json:"user_id"`
	MaxSize        int64      `json:"max_size"`     // Maximum allowed file size
	ContentType    string     `json:"content_type"` // Expected content type
	BytesUploaded  int64      `gorm:"default:0" json:"bytes_uploaded"`
	Completed      bool       `gorm:"default:false" json:"completed"`
	ObjectID       string     `json:"object_id,omitempty"` // ID of created storage object
	ExpiresAt      time.Time  `gorm:"not null" json:"expires_at"`
	CreatedAt      time.Time  `json:"created_at"`
	CompletedAt    *time.Time `json:"completed_at,omitempty"`
	ClientIP       string     `json:"client_ip,omitempty"`
}

// TableName sets the table name
func (UploadToken) TableName() string {
	return "upload_tokens"
}

// NewUploadToken creates a new upload token
func NewUploadToken(bucket string, parentFolderID *string, objectName, userID, contentType string, maxSize int64, duration time.Duration) *UploadToken {
	return &UploadToken{
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
func (ut *UploadToken) IsExpired() bool {
	return time.Now().After(ut.ExpiresAt)
}

// IsValid checks if the token is valid for use
func (ut *UploadToken) IsValid() bool {
	return !ut.IsExpired() && !ut.Completed
}

// CanAcceptBytes checks if the upload can accept more bytes
func (ut *UploadToken) CanAcceptBytes(additionalBytes int64) bool {
	return ut.BytesUploaded+additionalBytes <= ut.MaxSize
}
