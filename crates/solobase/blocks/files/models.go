package files

import (
	"encoding/json"
	"errors"

	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/solobase/core/uuid"
)

// PermissionLevel represents the level of permission for a share
type PermissionLevel string

const (
	PermissionView  PermissionLevel = "view"
	PermissionEdit  PermissionLevel = "edit"
	PermissionAdmin PermissionLevel = "admin"
)

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
	if s.SharedWithUserID == nil && s.SharedWithEmail == nil && s.ShareToken == nil {
		return ErrInvalidShareData
	}
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
	ObjectName      string           `json:"objectName"`
	ContentType     string           `json:"contentType"`
	Size            int64            `json:"size"`
	ObjectCreatedAt apptime.Time     `json:"objectCreatedAt"`
	ObjectMetadata  *json.RawMessage `json:"objectMetadata,omitempty"`
}

// storageObject is a local representation of a storage_objects row.
type storageObject struct {
	ID             string
	BucketName     string
	ObjectName     string
	ParentFolderID *string
	Size           int64
	ContentType    string
	Checksum       string
	Metadata       string
	CreatedAt      apptime.Time
	UpdatedAt      apptime.Time
	UserID         string
	AppID          *string
}

// RoleQuota defines storage quotas and limits for a specific IAM role
type RoleQuota struct {
	ID                string       `json:"id"`
	RoleID            string       `json:"roleId"`
	RoleName          string       `json:"roleName"`
	MaxStorageBytes   int64        `json:"maxStorageBytes"`
	MaxBandwidthBytes int64        `json:"maxBandwidthBytes"`
	MaxUploadSize     int64        `json:"maxUploadSize"`
	MaxFilesCount     int64        `json:"maxFilesCount"`
	AllowedExtensions string       `json:"allowedExtensions"`
	BlockedExtensions string       `json:"blockedExtensions"`
	CreatedAt         apptime.Time `json:"createdAt"`
	UpdatedAt         apptime.Time `json:"updatedAt"`
}

// PrepareForCreate prepares the role quota for insertion with defaults
func (r *RoleQuota) PrepareForCreate() {
	if r.ID == "" {
		r.ID = uuid.New().String()
	}
	now := apptime.NowTime()
	if r.CreatedAt.IsZero() {
		r.CreatedAt = now
	}
	r.UpdatedAt = now
	if r.MaxStorageBytes == 0 {
		r.MaxStorageBytes = 5368709120
	}
	if r.MaxBandwidthBytes == 0 {
		r.MaxBandwidthBytes = 10737418240
	}
	if r.MaxUploadSize == 0 {
		r.MaxUploadSize = 104857600
	}
	if r.MaxFilesCount == 0 {
		r.MaxFilesCount = 1000
	}
}

// UserQuotaOverride allows specific users to have custom quotas different from their role
type UserQuotaOverride struct {
	ID                string           `json:"id"`
	UserID            string           `json:"userId"`
	MaxStorageBytes   *int64           `json:"maxStorageBytes,omitempty"`
	MaxBandwidthBytes *int64           `json:"maxBandwidthBytes,omitempty"`
	MaxUploadSize     *int64           `json:"maxUploadSize,omitempty"`
	MaxFilesCount     *int64           `json:"maxFilesCount,omitempty"`
	AllowedExtensions *string          `json:"allowedExtensions,omitempty"`
	BlockedExtensions *string          `json:"blockedExtensions,omitempty"`
	Reason            *string          `json:"reason,omitempty"`
	ExpiresAt         apptime.NullTime `json:"expiresAt,omitempty"`
	CreatedBy         string           `json:"createdBy"`
	CreatedAt         apptime.Time     `json:"createdAt"`
	UpdatedAt         apptime.Time     `json:"updatedAt"`
}

// PrepareForCreate prepares the user quota override for insertion
func (u *UserQuotaOverride) PrepareForCreate() {
	if u.ID == "" {
		u.ID = uuid.New().String()
	}
	now := apptime.NowTime()
	if u.CreatedAt.IsZero() {
		u.CreatedAt = now
	}
	u.UpdatedAt = now
}

// ShareResponse is the response for creating a share.
type ShareResponse struct {
	ID              string        `json:"id"`
	ShareURL        string        `json:"shareUrl,omitempty"`
	ShareToken      string        `json:"shareToken,omitempty"`
	ExpiresAt       *apptime.Time `json:"expiresAt,omitempty"`
	PermissionLevel string        `json:"permissionLevel"`
}

// QuotaResponse is the response for quota information.
type QuotaResponse struct {
	StorageUsed         int64         `json:"storageUsed"`
	StorageLimit        int64         `json:"storageLimit"`
	StoragePercentage   float64       `json:"storagePercentage"`
	BandwidthUsed       int64         `json:"bandwidthUsed"`
	BandwidthLimit      int64         `json:"bandwidthLimit"`
	BandwidthPercentage float64       `json:"bandwidthPercentage"`
	ResetDate           *apptime.Time `json:"resetDate,omitempty"`
}

// CloudStorageConfig holds cloud-storage-specific configuration
type CloudStorageConfig struct {
	DefaultStorageLimit   int64
	DefaultBandwidthLimit int64
	EnableSharing         bool
	EnableAccessLogs      bool
	EnableQuotas          bool
	BandwidthResetPeriod  string
}

// ShareOptions defines options for creating a share
type ShareOptions struct {
	SharedWithUserID  string
	SharedWithEmail   string
	PermissionLevel   PermissionLevel
	InheritToChildren bool
	GenerateToken     bool
	IsPublic          bool
	ExpiresAt         *apptime.Time
}

// LogOptions defines options for logging access
type LogOptions struct {
	UserID    string
	ShareID   string
	IPAddress string
	UserAgent string
	Success   *bool
	ErrorMsg  string
	BytesSize int64
	Duration  apptime.Duration
}

// AccessLogFilters defines filters for access log queries
type AccessLogFilters struct {
	ObjectID  string
	UserID    string
	Action    string
	StartDate *apptime.Time
	EndDate   *apptime.Time
	Limit     int
}

// StatsFilters defines filters for statistics queries
type StatsFilters struct {
	ObjectID  string
	UserID    string
	StartDate *apptime.Time
	EndDate   *apptime.Time
}

// AccessStats represents access statistics
type AccessStats struct {
	TotalAccess     int64            `json:"totalAccess"`
	UniqueUsers     int64            `json:"uniqueUsers"`
	ActionBreakdown map[string]int64 `json:"actionBreakdown"`
}

// EffectiveQuota represents the calculated quota for a user
type EffectiveQuota struct {
	UserID            string `json:"userId"`
	MaxStorageBytes   int64  `json:"maxStorageBytes"`
	MaxBandwidthBytes int64  `json:"maxBandwidthBytes"`
	MaxUploadSize     int64  `json:"maxUploadSize"`
	MaxFilesCount     int64  `json:"maxFilesCount"`
	AllowedExtensions string `json:"allowedExtensions"`
	BlockedExtensions string `json:"blockedExtensions"`
	StorageUsed       int64  `json:"storageUsed"`
	BandwidthUsed     int64  `json:"bandwidthUsed"`
	FilesUsed         int64  `json:"filesUsed"`
}

// QuotaStats represents quota usage statistics
type QuotaStats struct {
	StorageUsed         int64   `json:"storageUsed"`
	StorageLimit        int64   `json:"storageLimit"`
	StoragePercentage   float64 `json:"storagePercentage"`
	BandwidthUsed       int64   `json:"bandwidthUsed"`
	BandwidthLimit      int64   `json:"bandwidthLimit"`
	BandwidthPercentage float64 `json:"bandwidthPercentage"`
}
