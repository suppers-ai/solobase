package cloudstorage

import (
	"time"
	"github.com/google/uuid"
	"gorm.io/gorm"
)

// RoleQuota defines storage quotas and limits for a specific IAM role
type RoleQuota struct {
	ID                string    `gorm:"type:uuid;primaryKey" json:"id"`
	RoleID            string    `gorm:"type:uuid;uniqueIndex;not null" json:"role_id"`
	RoleName          string    `gorm:"not null;index" json:"role_name"` // Indexed for faster lookups
	MaxStorageBytes   int64     `gorm:"type:bigint;not null;default:5368709120" json:"max_storage_bytes"`    // 5GB default
	MaxBandwidthBytes int64     `gorm:"type:bigint;not null;default:10737418240" json:"max_bandwidth_bytes"` // 10GB default  
	MaxUploadSize     int64     `gorm:"type:bigint;not null;default:104857600" json:"max_upload_size"`       // 100MB default
	MaxFilesCount     int64     `gorm:"type:bigint;not null;default:1000" json:"max_files_count"`            // 1000 files default
	AllowedExtensions string    `gorm:"type:text" json:"allowed_extensions"`                                 // Comma-separated list
	BlockedExtensions string    `gorm:"type:text" json:"blocked_extensions"`                                 // Comma-separated list
	CreatedAt         time.Time `gorm:"autoCreateTime" json:"created_at"`
	UpdatedAt         time.Time `gorm:"autoUpdateTime" json:"updated_at"`
}

// TableName specifies the table name with extension prefix
func (RoleQuota) TableName() string {
	return "ext_cloudstorage_role_quotas"
}

// BeforeCreate hook to generate UUID
func (r *RoleQuota) BeforeCreate(tx *gorm.DB) error {
	if r.ID == "" {
		r.ID = uuid.New().String()
	}
	return nil
}

// UserQuotaOverride allows specific users to have custom quotas different from their role
type UserQuotaOverride struct {
	ID                string     `gorm:"type:uuid;primaryKey" json:"id"`
	UserID            string     `gorm:"type:uuid;uniqueIndex;not null" json:"user_id"` // Unique index for fast lookups
	MaxStorageBytes   *int64     `gorm:"type:bigint" json:"max_storage_bytes,omitempty"`
	MaxBandwidthBytes *int64     `gorm:"type:bigint" json:"max_bandwidth_bytes,omitempty"`
	MaxUploadSize     *int64     `gorm:"type:bigint" json:"max_upload_size,omitempty"`
	MaxFilesCount     *int64     `gorm:"type:bigint" json:"max_files_count,omitempty"`
	AllowedExtensions *string    `gorm:"type:text" json:"allowed_extensions,omitempty"`
	BlockedExtensions *string    `gorm:"type:text" json:"blocked_extensions,omitempty"`
	Reason            string     `gorm:"type:text" json:"reason"` // Why this override was created
	ExpiresAt         *time.Time `gorm:"type:timestamptz;index" json:"expires_at,omitempty"` // Indexed for expiry queries
	CreatedBy         string     `gorm:"type:uuid;not null;index" json:"created_by"` // Indexed for admin queries
	CreatedAt         time.Time  `gorm:"autoCreateTime" json:"created_at"`
	UpdatedAt         time.Time  `gorm:"autoUpdateTime" json:"updated_at"`
}

// TableName specifies the table name with extension prefix
func (UserQuotaOverride) TableName() string {
	return "ext_cloudstorage_user_quota_overrides"
}

// BeforeCreate hook to generate UUID
func (u *UserQuotaOverride) BeforeCreate(tx *gorm.DB) error {
	if u.ID == "" {
		u.ID = uuid.New().String()
	}
	return nil
}