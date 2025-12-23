package cloudstorage

import (
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
)

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

// TableName specifies the table name with extension prefix
func (RoleQuota) TableName() string {
	return "ext_cloudstorage_role_quotas"
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
	// Set defaults
	if r.MaxStorageBytes == 0 {
		r.MaxStorageBytes = 5368709120 // 5GB default
	}
	if r.MaxBandwidthBytes == 0 {
		r.MaxBandwidthBytes = 10737418240 // 10GB default
	}
	if r.MaxUploadSize == 0 {
		r.MaxUploadSize = 104857600 // 100MB default
	}
	if r.MaxFilesCount == 0 {
		r.MaxFilesCount = 1000 // 1000 files default
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

// TableName specifies the table name with extension prefix
func (UserQuotaOverride) TableName() string {
	return "ext_cloudstorage_user_quota_overrides"
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
