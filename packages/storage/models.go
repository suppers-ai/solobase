package storage

import (
	"time"
)

// StorageBucket represents a storage bucket in the database
type StorageBucket struct {
	ID        string    `gorm:"primaryKey" json:"id"`
	Name      string    `gorm:"uniqueIndex;not null" json:"name"`
	Public    bool      `gorm:"default:false" json:"public"`
	CreatedAt time.Time `json:"created_at"`
	UpdatedAt time.Time `json:"updated_at"`
}

// TableName specifies the table name
func (StorageBucket) TableName() string {
	return "storage_buckets"
}

// StorageObject represents a stored file/object in the database
type StorageObject struct {
	ID             string     `gorm:"primaryKey" json:"id"`
	BucketName     string     `gorm:"not null;index" json:"bucket_name"`
	ObjectName     string     `gorm:"not null;index" json:"object_name"`       // Just the name (file.txt or foldername)
	ParentFolderID *string    `gorm:"index" json:"parent_folder_id,omitempty"` // ID of parent folder, null for root items
	Size           int64      `json:"size"`
	ContentType    string     `json:"content_type"`                        // "application/x-directory" for folders
	Checksum       string     `gorm:"index" json:"checksum,omitempty"`     // MD5 or SHA256 hash
	Metadata       string     `gorm:"type:text" json:"metadata,omitempty"` // JSON string
	CreatedAt      time.Time  `json:"created_at"`
	UpdatedAt      time.Time  `json:"updated_at"`
	LastViewed     *time.Time `gorm:"index" json:"last_viewed,omitempty"` // Track when the item was last viewed
	UserID         string     `gorm:"index" json:"user_id,omitempty"`
	AppID          *string    `gorm:"index" json:"app_id,omitempty"` // Application ID, null for admin uploads
}

// TableName specifies the table name
func (StorageObject) TableName() string {
	return "storage_objects"
}

// IsFolder returns true if this object is a folder
func (s *StorageObject) IsFolder() bool {
	return s.ContentType == "application/x-directory"
}

// IsFile returns true if this object is a file
func (s *StorageObject) IsFile() bool {
	return s.ContentType != "application/x-directory"
}
