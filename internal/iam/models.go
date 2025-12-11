package iam

import (
	"time"

	"github.com/google/uuid"
	"gorm.io/gorm"
)

// Role represents a role in the IAM system
type Role struct {
	ID          string                 `gorm:"type:uuid;primaryKey" json:"id"`
	Name        string                 `gorm:"uniqueIndex;not null" json:"name"`
	DisplayName string                 `json:"displayName"`
	Description string                 `json:"description"`
	Type        string                 `json:"type"` // "system" for protected roles, "custom" for user-created roles
	Metadata    map[string]interface{} `gorm:"serializer:json" json:"metadata"`
	CreatedAt   time.Time              `json:"createdAt"`
	UpdatedAt   time.Time              `json:"updatedAt"`
}

func (r *Role) TableName() string {
	return "iam_roles"
}

func (r *Role) BeforeCreate(tx *gorm.DB) error {
	if r.ID == "" {
		r.ID = uuid.New().String()
	}
	return nil
}

// UserRole represents the many-to-many relationship between users and roles
type UserRole struct {
	ID        string     `gorm:"type:uuid;primaryKey" json:"id"`
	UserID    string     `gorm:"type:uuid;not null;index:idx_user_role,unique" json:"userId"`
	RoleID    string     `gorm:"type:uuid;not null;index:idx_user_role,unique" json:"roleId"`
	GrantedBy string     `gorm:"type:uuid" json:"grantedBy"`
	GrantedAt time.Time  `json:"grantedAt"`
	ExpiresAt *time.Time `json:"expiresAt,omitempty"`
}

func (ur *UserRole) TableName() string {
	return "iam_user_roles"
}

func (ur *UserRole) BeforeCreate(tx *gorm.DB) error {
	if ur.ID == "" {
		ur.ID = uuid.New().String()
	}
	if ur.GrantedAt.IsZero() {
		ur.GrantedAt = time.Now()
	}
	return nil
}

// IAMAuditLog represents an audit log entry for IAM actions
type IAMAuditLog struct {
	ID        string                 `gorm:"type:uuid;primaryKey" json:"id"`
	UserID    string                 `gorm:"type:uuid;index" json:"userId"`
	Action    string                 `json:"action"`
	Resource  string                 `json:"resource"`
	Result    string                 `json:"result"` // "allow" or "deny"
	Reason    string                 `json:"reason"`
	IPAddress string                 `json:"ipAddress"`
	UserAgent string                 `json:"userAgent"`
	Metadata  map[string]interface{} `gorm:"serializer:json" json:"metadata"`
	CreatedAt time.Time              `json:"createdAt"`
}

func (al *IAMAuditLog) BeforeCreate(tx *gorm.DB) error {
	if al.ID == "" {
		al.ID = uuid.New().String()
	}
	return nil
}

// RoleMetadata contains role-specific access control configuration
// Note: Resource quotas (storage, bandwidth, upload size) are managed by the CloudStorage extension
// Note: Rate limiting and session management are handled by their respective middleware
type RoleMetadata struct {
	// Access control settings
	AllowedIPs       []string `json:"allowedIps,omitempty"`       // IP whitelist for access control
	DisabledFeatures []string `json:"disabledFeatures,omitempty"` // List of disabled features for this role
}
