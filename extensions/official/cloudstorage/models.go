package cloudstorage

import (
	"database/sql/driver"
	"github.com/google/uuid"
	"gorm.io/datatypes"
	"gorm.io/gorm"
	"time"
)

// PermissionLevel represents the level of permission for a share
type PermissionLevel string

const (
	PermissionView  PermissionLevel = "view"
	PermissionEdit  PermissionLevel = "edit"
	PermissionAdmin PermissionLevel = "admin"
)

func (p *PermissionLevel) Scan(value interface{}) error {
	*p = PermissionLevel(value.(string))
	return nil
}

func (p PermissionLevel) Value() (driver.Value, error) {
	return string(p), nil
}

// StorageAction represents the type of action taken on a storage object
type StorageAction string

const (
	ActionView     StorageAction = "view"
	ActionDownload StorageAction = "download"
	ActionUpload   StorageAction = "upload"
	ActionDelete   StorageAction = "delete"
	ActionShare    StorageAction = "share"
	ActionEdit     StorageAction = "edit"
)

func (a *StorageAction) Scan(value interface{}) error {
	*a = StorageAction(value.(string))
	return nil
}

func (a StorageAction) Value() (driver.Value, error) {
	return string(a), nil
}

// StorageShare represents a shared storage object with granular permissions
type StorageShare struct {
	ID                string          `gorm:"type:uuid;primaryKey" json:"id"`
	ObjectID          string          `gorm:"type:uuid;not null;index" json:"objectId"`
	SharedWithUserID  *string         `gorm:"type:uuid;index" json:"sharedWithUserId,omitempty"`
	SharedWithEmail   *string         `gorm:"type:text" json:"sharedWithEmail,omitempty"`
	PermissionLevel   PermissionLevel `gorm:"type:text;not null;default:'view'" json:"permissionLevel"`
	InheritToChildren bool            `gorm:"default:true;not null" json:"inheritToChildren"`
	ShareToken        *string         `gorm:"type:text;uniqueIndex" json:"shareToken,omitempty"`
	IsPublic          bool            `gorm:"default:false;not null" json:"isPublic"`
	ExpiresAt         *time.Time      `gorm:"type:timestamptz" json:"expiresAt,omitempty"`
	CreatedBy         string          `gorm:"type:uuid;not null" json:"createdBy"`
	CreatedAt         time.Time       `gorm:"autoCreateTime" json:"createdAt"`
	UpdatedAt         time.Time       `gorm:"autoUpdateTime" json:"updatedAt"`
}

// TableName specifies the table name with extension prefix
func (StorageShare) TableName() string {
	return "ext_cloudstorage_storage_shares"
}

// BeforeCreate hook to validate share constraints and generate UUID
func (s *StorageShare) BeforeCreate(tx *gorm.DB) error {
	// Generate UUID if not set
	if s.ID == "" {
		s.ID = uuid.New().String()
	}
	// Ensure at least one sharing method is specified
	if s.SharedWithUserID == nil && s.SharedWithEmail == nil && s.ShareToken == nil {
		return gorm.ErrInvalidData
	}
	// Ensure not both user_id and email are set
	if s.SharedWithUserID != nil && s.SharedWithEmail != nil {
		return gorm.ErrInvalidData
	}
	return nil
}

// StorageAccessLog tracks all access to storage objects
type StorageAccessLog struct {
	ID        string         `gorm:"type:uuid;primaryKey" json:"id"`
	ObjectID  string         `gorm:"type:uuid;not null;index" json:"objectId"`
	UserID    *string        `gorm:"type:uuid;index" json:"userId,omitempty"`
	IPAddress *string        `gorm:"type:inet" json:"ipAddress,omitempty"`
	Action    StorageAction  `gorm:"type:text;not null" json:"action"`
	UserAgent *string        `gorm:"type:text" json:"userAgent,omitempty"`
	Metadata  datatypes.JSON `gorm:"type:jsonb;default:'{}'" json:"metadata"`
	CreatedAt time.Time      `gorm:"autoCreateTime" json:"createdAt"` // Use GORM's auto create time
}

// TableName specifies the table name with extension prefix
func (StorageAccessLog) TableName() string {
	return "ext_cloudstorage_storage_access_logs"
}

// BeforeCreate hook to generate UUID
func (s *StorageAccessLog) BeforeCreate(tx *gorm.DB) error {
	if s.ID == "" {
		s.ID = uuid.New().String()
	}
	return nil
}

// StorageQuota defines storage and bandwidth limits for users
type StorageQuota struct {
	ID                string     `gorm:"type:uuid;primaryKey" json:"id"`
	UserID            string     `gorm:"type:uuid;not null;uniqueIndex" json:"userId"`
	MaxStorageBytes   int64      `gorm:"type:bigint;not null;default:5368709120" json:"maxStorageBytes"`    // 5GB default
	MaxBandwidthBytes int64      `gorm:"type:bigint;not null;default:10737418240" json:"maxBandwidthBytes"` // 10GB default
	StorageUsed       int64      `gorm:"type:bigint;not null;default:0" json:"storageUsed"`
	BandwidthUsed     int64      `gorm:"type:bigint;not null;default:0" json:"bandwidthUsed"`
	ResetBandwidthAt  *time.Time `gorm:"type:timestamptz" json:"resetBandwidthAt,omitempty"`
	CreatedAt         time.Time  `gorm:"autoCreateTime" json:"createdAt"`
	UpdatedAt         time.Time  `gorm:"autoUpdateTime" json:"updatedAt"`
}

// TableName specifies the table name with extension prefix
func (StorageQuota) TableName() string {
	return "ext_cloudstorage_storage_quotas"
}

// BeforeCreate hook to generate UUID
func (s *StorageQuota) BeforeCreate(tx *gorm.DB) error {
	if s.ID == "" {
		s.ID = uuid.New().String()
	}
	return nil
}

// StorageShareWithObject combines share data with object information
type StorageShareWithObject struct {
	StorageShare
	ObjectName      string         `json:"objectName"`
	ContentType     string         `json:"contentType"`
	Size            int64          `json:"size"`
	ObjectCreatedAt time.Time      `json:"objectCreatedAt"`
	ObjectMetadata  datatypes.JSON `json:"objectMetadata"`
}
