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
	DisplayName string                 `json:"display_name"`
	Description string                 `json:"description"`
	IsSystem    bool                   `json:"is_system"` // System roles cannot be deleted
	Metadata    map[string]interface{} `gorm:"serializer:json" json:"metadata"`
	CreatedAt   time.Time              `json:"created_at"`
	UpdatedAt   time.Time              `json:"updated_at"`
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
	ID        string    `gorm:"type:uuid;primaryKey" json:"id"`
	UserID    string    `gorm:"type:uuid;not null;index" json:"user_id"`
	RoleID    string    `gorm:"type:uuid;not null;index" json:"role_id"`
	GrantedBy string    `gorm:"type:uuid" json:"granted_by"`
	GrantedAt time.Time `json:"granted_at"`
	ExpiresAt *time.Time `json:"expires_at,omitempty"`
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
	ID         string                 `gorm:"type:uuid;primaryKey" json:"id"`
	UserID     string                 `gorm:"type:uuid;index" json:"user_id"`
	Action     string                 `json:"action"`
	Resource   string                 `json:"resource"`
	Result     string                 `json:"result"` // "allow" or "deny"
	Reason     string                 `json:"reason"`
	IPAddress  string                 `json:"ip_address"`
	UserAgent  string                 `json:"user_agent"`
	Metadata   map[string]interface{} `gorm:"serializer:json" json:"metadata"`
	CreatedAt  time.Time              `json:"created_at"`
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
	AllowedIPs        []string `json:"allowed_ips,omitempty"`        // IP whitelist for access control
	DisabledFeatures  []string `json:"disabled_features,omitempty"`  // List of disabled features for this role
}