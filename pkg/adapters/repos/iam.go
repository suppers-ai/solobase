package repos

import (
	"context"

	"github.com/suppers-ai/solobase/internal/iam/types"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

// AuditLogQueryOptions configures audit log queries
type AuditLogQueryOptions struct {
	UserID *string
	Action *string
	Result *string
	Pagination
}

// IAMRepository provides IAM persistence operations
type IAMRepository interface {
	// Roles
	CreateRole(ctx context.Context, role *types.Role) error
	GetRole(ctx context.Context, id string) (*types.Role, error)
	GetRoleByName(ctx context.Context, name string) (*types.Role, error)
	ListRoles(ctx context.Context) ([]*types.Role, error)
	ListRolesByType(ctx context.Context, roleType string) ([]*types.Role, error)
	UpdateRole(ctx context.Context, role *types.Role) error
	DeleteRole(ctx context.Context, id string) error

	// User-Role assignments
	CreateUserRole(ctx context.Context, userRole *types.UserRole) error
	GetUserRole(ctx context.Context, userID, roleID string) (*types.UserRole, error)
	ListUserRolesByUserID(ctx context.Context, userID string) ([]*types.UserRole, error)
	ListUserRolesByRoleID(ctx context.Context, roleID string) ([]*types.UserRole, error)
	ListUserIDsWithRole(ctx context.Context, roleID string) ([]string, error)
	DeleteUserRole(ctx context.Context, userID, roleID string) error
	DeleteUserRolesByUserID(ctx context.Context, userID string) error
	DeleteUserRolesByRoleID(ctx context.Context, roleID string) error
	DeleteExpiredUserRoles(ctx context.Context) error

	// Policies (Casbin-style) - for RBAC rules
	CreatePolicy(ctx context.Context, policy *types.IAMPolicy) error
	GetPolicy(ctx context.Context, ptype string, v0, v1, v2, v3, v4, v5 *string) (*types.IAMPolicy, error)
	ListPolicies(ctx context.Context) ([]*types.IAMPolicy, error)
	ListPoliciesByType(ctx context.Context, ptype string) ([]*types.IAMPolicy, error)
	ListPoliciesBySubject(ctx context.Context, subject string) ([]*types.IAMPolicy, error)
	ListGroupingPolicies(ctx context.Context) ([]*types.IAMPolicy, error)
	ListGroupingPoliciesByUser(ctx context.Context, userID string) ([]*types.IAMPolicy, error)
	DeletePolicy(ctx context.Context, id string) error
	DeletePolicyByValues(ctx context.Context, ptype string, v0, v1, v2, v3, v4, v5 *string) error
	DeletePoliciesBySubject(ctx context.Context, subject string) error
	DeleteGroupingPoliciesByUser(ctx context.Context, userID string) error

	// Audit logs
	CreateAuditLog(ctx context.Context, log *types.IAMAuditLog) error
	ListAuditLogs(ctx context.Context, opts AuditLogQueryOptions) (*PaginatedResult[*types.IAMAuditLog], error)
	CountAuditLogs(ctx context.Context) (int64, error)
	CountAuditLogsByUserID(ctx context.Context, userID string) (int64, error)
	DeleteAuditLogsOlderThan(ctx context.Context, cutoff apptime.Time) error
}
