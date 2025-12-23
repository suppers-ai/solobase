package iam

import (
	"github.com/suppers-ai/solobase/internal/iam/types"
)

// Re-export types for backward compatibility
// These are aliases to the types in internal/iam/types package

type (
	Role         = types.Role
	UserRole     = types.UserRole
	IAMPolicy    = types.IAMPolicy
	IAMAuditLog  = types.IAMAuditLog
	RoleMetadata = types.RoleMetadata
)
