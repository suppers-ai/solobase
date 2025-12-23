//go:build wasm

package wasm

import (
	"context"

	"github.com/suppers-ai/solobase/internal/iam/types"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type iamRepository struct{}

// Role operations

func (r *iamRepository) CreateRole(ctx context.Context, role *types.Role) error {
	return ErrNotImplemented
}

func (r *iamRepository) GetRole(ctx context.Context, id string) (*types.Role, error) {
	return nil, ErrNotImplemented
}

func (r *iamRepository) GetRoleByName(ctx context.Context, name string) (*types.Role, error) {
	return nil, ErrNotImplemented
}

func (r *iamRepository) ListRoles(ctx context.Context) ([]*types.Role, error) {
	return nil, ErrNotImplemented
}

func (r *iamRepository) ListRolesByType(ctx context.Context, roleType string) ([]*types.Role, error) {
	return nil, ErrNotImplemented
}

func (r *iamRepository) UpdateRole(ctx context.Context, role *types.Role) error {
	return ErrNotImplemented
}

func (r *iamRepository) DeleteRole(ctx context.Context, id string) error {
	return ErrNotImplemented
}

// User-Role operations

func (r *iamRepository) CreateUserRole(ctx context.Context, userRole *types.UserRole) error {
	return ErrNotImplemented
}

func (r *iamRepository) GetUserRole(ctx context.Context, userID, roleID string) (*types.UserRole, error) {
	return nil, ErrNotImplemented
}

func (r *iamRepository) ListUserRolesByUserID(ctx context.Context, userID string) ([]*types.UserRole, error) {
	return nil, ErrNotImplemented
}

func (r *iamRepository) ListUserRolesByRoleID(ctx context.Context, roleID string) ([]*types.UserRole, error) {
	return nil, ErrNotImplemented
}

func (r *iamRepository) ListUserIDsWithRole(ctx context.Context, roleID string) ([]string, error) {
	return nil, ErrNotImplemented
}

func (r *iamRepository) DeleteUserRole(ctx context.Context, userID, roleID string) error {
	return ErrNotImplemented
}

func (r *iamRepository) DeleteUserRolesByUserID(ctx context.Context, userID string) error {
	return ErrNotImplemented
}

func (r *iamRepository) DeleteUserRolesByRoleID(ctx context.Context, roleID string) error {
	return ErrNotImplemented
}

func (r *iamRepository) DeleteExpiredUserRoles(ctx context.Context) error {
	return ErrNotImplemented
}

// Policy operations

func (r *iamRepository) CreatePolicy(ctx context.Context, policy *types.IAMPolicy) error {
	return ErrNotImplemented
}

func (r *iamRepository) GetPolicy(ctx context.Context, ptype string, v0, v1, v2, v3, v4, v5 *string) (*types.IAMPolicy, error) {
	return nil, ErrNotImplemented
}

func (r *iamRepository) ListPolicies(ctx context.Context) ([]*types.IAMPolicy, error) {
	return nil, ErrNotImplemented
}

func (r *iamRepository) ListPoliciesByType(ctx context.Context, ptype string) ([]*types.IAMPolicy, error) {
	return nil, ErrNotImplemented
}

func (r *iamRepository) ListPoliciesBySubject(ctx context.Context, subject string) ([]*types.IAMPolicy, error) {
	return nil, ErrNotImplemented
}

func (r *iamRepository) ListGroupingPolicies(ctx context.Context) ([]*types.IAMPolicy, error) {
	return nil, ErrNotImplemented
}

func (r *iamRepository) ListGroupingPoliciesByUser(ctx context.Context, userID string) ([]*types.IAMPolicy, error) {
	return nil, ErrNotImplemented
}

func (r *iamRepository) DeletePolicy(ctx context.Context, id string) error {
	return ErrNotImplemented
}

func (r *iamRepository) DeletePolicyByValues(ctx context.Context, ptype string, v0, v1, v2, v3, v4, v5 *string) error {
	return ErrNotImplemented
}

func (r *iamRepository) DeletePoliciesBySubject(ctx context.Context, subject string) error {
	return ErrNotImplemented
}

func (r *iamRepository) DeleteGroupingPoliciesByUser(ctx context.Context, userID string) error {
	return ErrNotImplemented
}

// Audit log operations

func (r *iamRepository) CreateAuditLog(ctx context.Context, log *types.IAMAuditLog) error {
	return ErrNotImplemented
}

func (r *iamRepository) ListAuditLogs(ctx context.Context, opts repos.AuditLogQueryOptions) (*repos.PaginatedResult[*types.IAMAuditLog], error) {
	return nil, ErrNotImplemented
}

func (r *iamRepository) CountAuditLogs(ctx context.Context) (int64, error) {
	return 0, ErrNotImplemented
}

func (r *iamRepository) CountAuditLogsByUserID(ctx context.Context, userID string) (int64, error) {
	return 0, ErrNotImplemented
}

func (r *iamRepository) DeleteAuditLogsOlderThan(ctx context.Context, cutoff apptime.Time) error {
	return ErrNotImplemented
}

// Ensure iamRepository implements IAMRepository
var _ repos.IAMRepository = (*iamRepository)(nil)
