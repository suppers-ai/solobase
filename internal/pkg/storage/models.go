package storage

import (
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

// StorageBucket represents a storage bucket in the database
type StorageBucket struct {
	ID        string       `json:"id"`
	Name      string       `json:"name"`
	Public    bool         `json:"public"`
	CreatedAt apptime.Time `json:"createdAt"`
	UpdatedAt apptime.Time `json:"updatedAt"`
}

// TableName specifies the table name
func (StorageBucket) TableName() string {
	return "storage_buckets"
}

// StorageObject represents a stored file/object in the database
type StorageObject struct {
	ID             string           `json:"id"`
	BucketName     string           `json:"bucketName"`
	ObjectName     string           `json:"objectName"`               // Just the name (file.txt or foldername)
	ParentFolderID *string          `json:"parentFolderId,omitempty"` // ID of parent folder, null for root items
	Size           int64            `json:"size"`
	ContentType    string           `json:"contentType"`         // "application/x-directory" for folders
	Checksum       string           `json:"checksum,omitempty"`  // MD5 or SHA256 hash
	Metadata       string           `json:"metadata,omitempty"`  // JSON string
	CreatedAt      apptime.Time     `json:"createdAt"`
	UpdatedAt      apptime.Time     `json:"updatedAt"`
	LastViewed     apptime.NullTime `json:"lastViewed,omitempty"` // Track when the item was last viewed
	UserID         string           `json:"userId,omitempty"`
	AppID          *string          `json:"appId,omitempty"` // Application ID, null for admin uploads
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
