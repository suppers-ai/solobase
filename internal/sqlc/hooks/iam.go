package hooks

import (
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
)

// RoleBeforeCreate prepares a role for creation
// Note: created_at and updated_at use SQLite DEFAULT CURRENT_TIMESTAMP
func RoleBeforeCreate(params *db.CreateRoleParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}
	return nil
}

// UserRoleBeforeCreate prepares a user role assignment for creation
// Note: granted_at uses SQLite DEFAULT CURRENT_TIMESTAMP
func UserRoleBeforeCreate(params *db.CreateUserRoleParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}
	return nil
}

// PolicyBeforeCreate prepares a policy for creation
// Note: created_at uses SQLite DEFAULT CURRENT_TIMESTAMP
func PolicyBeforeCreate(params *db.CreatePolicyParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}
	return nil
}

// AuditLogBeforeCreate prepares an audit log entry for creation
// Note: created_at uses SQLite DEFAULT CURRENT_TIMESTAMP
func AuditLogBeforeCreate(params *db.CreateAuditLogParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}
	return nil
}
