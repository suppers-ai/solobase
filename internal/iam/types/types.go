// Package types contains IAM model definitions
// This package is separate from internal/iam to avoid import cycles
// with pkg/adapters/repos
package types

import (
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
)

// Role represents a role in the IAM system
type Role struct {
	ID          string                 `json:"id"`
	Name        string                 `json:"name"`
	DisplayName string                 `json:"displayName"`
	Description string                 `json:"description"`
	Type        string                 `json:"type"` // "system" for protected roles, "custom" for user-created roles
	Metadata    map[string]interface{} `json:"metadata"`
	CreatedAt   apptime.Time           `json:"createdAt"`
	UpdatedAt   apptime.Time           `json:"updatedAt"`
}

func (r *Role) TableName() string {
	return "iam_roles"
}

// PrepareForCreate prepares the role for database insertion
func (r *Role) PrepareForCreate() {
	if r.ID == "" {
		r.ID = uuid.New().String()
	}
	now := apptime.NowTime()
	r.CreatedAt = now
	r.UpdatedAt = now
}

// UserRole represents the many-to-many relationship between users and roles
type UserRole struct {
	ID        string           `json:"id"`
	UserID    string           `json:"userId"`
	RoleID    string           `json:"roleId"`
	GrantedBy string           `json:"grantedBy"`
	GrantedAt apptime.Time     `json:"grantedAt"`
	ExpiresAt apptime.NullTime `json:"expiresAt,omitempty"`
}

func (ur *UserRole) TableName() string {
	return "iam_user_roles"
}

// PrepareForCreate prepares the user role for database insertion
func (ur *UserRole) PrepareForCreate() {
	if ur.ID == "" {
		ur.ID = uuid.New().String()
	}
	if ur.GrantedAt.IsZero() {
		ur.GrantedAt = apptime.NowTime()
	}
}

// IAMPolicy represents a policy in the IAM system (for RBAC rules)
type IAMPolicy struct {
	ID        string       `json:"id"`
	Ptype     string       `json:"ptype"`
	V0        *string      `json:"v0"`
	V1        *string      `json:"v1"`
	V2        *string      `json:"v2"`
	V3        *string      `json:"v3"`
	V4        *string      `json:"v4"`
	V5        *string      `json:"v5"`
	CreatedAt apptime.Time `json:"createdAt"`
}

func (p *IAMPolicy) TableName() string {
	return "iam_policies"
}

// PrepareForCreate prepares the policy for database insertion
func (p *IAMPolicy) PrepareForCreate() {
	if p.ID == "" {
		p.ID = uuid.New().String()
	}
	if p.CreatedAt.IsZero() {
		p.CreatedAt = apptime.NowTime()
	}
}

// IAMAuditLog represents an audit log entry for IAM actions
type IAMAuditLog struct {
	ID        string                 `json:"id"`
	UserID    string                 `json:"userId"`
	Action    string                 `json:"action"`
	Resource  string                 `json:"resource"`
	Result    string                 `json:"result"` // "allow" or "deny"
	Reason    string                 `json:"reason"`
	IPAddress string                 `json:"ipAddress"`
	UserAgent string                 `json:"userAgent"`
	Metadata  map[string]interface{} `json:"metadata"`
	CreatedAt apptime.Time           `json:"createdAt"`
}

// PrepareForCreate prepares the audit log for database insertion
func (al *IAMAuditLog) PrepareForCreate() {
	if al.ID == "" {
		al.ID = uuid.New().String()
	}
	if al.CreatedAt.IsZero() {
		al.CreatedAt = apptime.NowTime()
	}
}

// RoleMetadata contains role-specific access control configuration
// Note: Resource quotas (storage, bandwidth, upload size) are managed by the CloudStorage extension
// Note: Rate limiting and session management are handled by their respective middleware
type RoleMetadata struct {
	// Access control settings
	AllowedIPs       []string `json:"allowedIps,omitempty"`       // IP whitelist for access control
	DisabledFeatures []string `json:"disabledFeatures,omitempty"` // List of disabled features for this role
}
