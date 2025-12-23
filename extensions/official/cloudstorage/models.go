package cloudstorage

import (
	"database/sql/driver"
	"encoding/json"
	"errors"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
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

// ErrInvalidShareData indicates invalid share configuration
var ErrInvalidShareData = errors.New("invalid share data: must specify user ID, email, or token, but not both user ID and email")

// StorageShare represents a shared storage object with granular permissions
type StorageShare struct {
	ID                string           `json:"id"`
	ObjectID          string           `json:"objectId"`
	SharedWithUserID  *string          `json:"sharedWithUserId,omitempty"`
	SharedWithEmail   *string          `json:"sharedWithEmail,omitempty"`
	PermissionLevel   PermissionLevel  `json:"permissionLevel"`
	InheritToChildren bool             `json:"inheritToChildren"`
	ShareToken        *string          `json:"shareToken,omitempty"`
	IsPublic          bool             `json:"isPublic"`
	ExpiresAt         apptime.NullTime `json:"expiresAt,omitempty"`
	CreatedBy         string           `json:"createdBy"`
	CreatedAt         apptime.Time     `json:"createdAt"`
	UpdatedAt         apptime.Time     `json:"updatedAt"`
}

// TableName specifies the table name with extension prefix
func (StorageShare) TableName() string {
	return "ext_cloudstorage_storage_shares"
}

// PrepareForCreate prepares the share for insertion and validates constraints
func (s *StorageShare) PrepareForCreate() error {
	if s.ID == "" {
		s.ID = uuid.New().String()
	}
	now := apptime.NowTime()
	if s.CreatedAt.IsZero() {
		s.CreatedAt = now
	}
	s.UpdatedAt = now
	if s.PermissionLevel == "" {
		s.PermissionLevel = PermissionView
	}
	// Ensure at least one sharing method is specified
	if s.SharedWithUserID == nil && s.SharedWithEmail == nil && s.ShareToken == nil {
		return ErrInvalidShareData
	}
	// Ensure not both user_id and email are set
	if s.SharedWithUserID != nil && s.SharedWithEmail != nil {
		return ErrInvalidShareData
	}
	return nil
}

// StorageAccessLog tracks all access to storage objects
type StorageAccessLog struct {
	ID        string          `json:"id"`
	ObjectID  string          `json:"objectId"`
	UserID    *string         `json:"userId,omitempty"`
	IPAddress *string         `json:"ipAddress,omitempty"`
	Action    StorageAction   `json:"action"`
	UserAgent *string         `json:"userAgent,omitempty"`
	Metadata  json.RawMessage `json:"metadata"`
	CreatedAt apptime.Time    `json:"createdAt"`
}

// TableName specifies the table name with extension prefix
func (StorageAccessLog) TableName() string {
	return "ext_cloudstorage_storage_access_logs"
}

// PrepareForCreate prepares the access log for insertion
func (s *StorageAccessLog) PrepareForCreate() {
	if s.ID == "" {
		s.ID = uuid.New().String()
	}
	if s.CreatedAt.IsZero() {
		s.CreatedAt = apptime.NowTime()
	}
	if s.Metadata == nil {
		s.Metadata = json.RawMessage("{}")
	}
}

// StorageQuota defines storage and bandwidth limits for users
type StorageQuota struct {
	ID                string           `json:"id"`
	UserID            string           `json:"userId"`
	MaxStorageBytes   int64            `json:"maxStorageBytes"`
	MaxBandwidthBytes int64            `json:"maxBandwidthBytes"`
	StorageUsed       int64            `json:"storageUsed"`
	BandwidthUsed     int64            `json:"bandwidthUsed"`
	ResetBandwidthAt  apptime.NullTime `json:"resetBandwidthAt,omitempty"`
	CreatedAt         apptime.Time     `json:"createdAt"`
	UpdatedAt         apptime.Time     `json:"updatedAt"`
}

// TableName specifies the table name with extension prefix
func (StorageQuota) TableName() string {
	return "ext_cloudstorage_storage_quotas"
}

// PrepareForCreate prepares the quota for insertion with defaults
func (s *StorageQuota) PrepareForCreate() {
	if s.ID == "" {
		s.ID = uuid.New().String()
	}
	now := apptime.NowTime()
	if s.CreatedAt.IsZero() {
		s.CreatedAt = now
	}
	s.UpdatedAt = now
	// Set defaults
	if s.MaxStorageBytes == 0 {
		s.MaxStorageBytes = 5368709120 // 5GB default
	}
	if s.MaxBandwidthBytes == 0 {
		s.MaxBandwidthBytes = 10737418240 // 10GB default
	}
}

// StorageShareWithObject combines share data with object information
type StorageShareWithObject struct {
	StorageShare
	ObjectName      string          `json:"objectName"`
	ContentType     string          `json:"contentType"`
	Size            int64           `json:"size"`
	ObjectCreatedAt apptime.Time    `json:"objectCreatedAt"`
	ObjectMetadata  json.RawMessage `json:"objectMetadata"`
}
