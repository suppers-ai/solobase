//go:build !wasm

package sqlite

import (
	"context"
	"database/sql"
	"encoding/json"

	"github.com/suppers-ai/solobase/internal/iam/types"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type iamRepository struct {
	sqlDB   *sql.DB
	queries *db.Queries
}

// NewIAMRepository creates a new SQLite IAM repository
func NewIAMRepository(sqlDB *sql.DB, queries *db.Queries) repos.IAMRepository {
	return &iamRepository{
		sqlDB:   sqlDB,
		queries: queries,
	}
}

// Role operations

func (r *iamRepository) CreateRole(ctx context.Context, role *types.Role) error {
	role.PrepareForCreate()
	metadata, _ := json.Marshal(role.Metadata)
	metadataStr := string(metadata)

	return r.queries.CreateRole(ctx, db.CreateRoleParams{
		ID:          role.ID,
		Name:        role.Name,
		DisplayName: strPtr(role.DisplayName),
		Description: strPtr(role.Description),
		Type:        strPtr(role.Type),
		Metadata:    &metadataStr,
	})
}

func (r *iamRepository) GetRole(ctx context.Context, id string) (*types.Role, error) {
	dbRole, err := r.queries.GetRoleByID(ctx, id)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBRoleToModel(dbRole), nil
}

func (r *iamRepository) GetRoleByName(ctx context.Context, name string) (*types.Role, error) {
	dbRole, err := r.queries.GetRoleByName(ctx, name)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBRoleToModel(dbRole), nil
}

func (r *iamRepository) ListRoles(ctx context.Context) ([]*types.Role, error) {
	dbRoles, err := r.queries.ListRoles(ctx)
	if err != nil {
		return nil, err
	}
	roles := make([]*types.Role, len(dbRoles))
	for i, role := range dbRoles {
		roles[i] = convertDBRoleToModel(role)
	}
	return roles, nil
}

func (r *iamRepository) ListRolesByType(ctx context.Context, roleType string) ([]*types.Role, error) {
	dbRoles, err := r.queries.ListRolesByType(ctx, &roleType)
	if err != nil {
		return nil, err
	}
	roles := make([]*types.Role, len(dbRoles))
	for i, role := range dbRoles {
		roles[i] = convertDBRoleToModel(role)
	}
	return roles, nil
}

func (r *iamRepository) UpdateRole(ctx context.Context, role *types.Role) error {
	role.UpdatedAt = apptime.NowTime()
	metadata, _ := json.Marshal(role.Metadata)
	metadataStr := string(metadata)

	return r.queries.UpdateRole(ctx, db.UpdateRoleParams{
		ID:          role.ID,
		DisplayName: strPtr(role.DisplayName),
		Description: strPtr(role.Description),
		Metadata:    &metadataStr,
		UpdatedAt:   apptime.Format(role.UpdatedAt),
	})
}

func (r *iamRepository) DeleteRole(ctx context.Context, id string) error {
	return r.queries.DeleteRole(ctx, id)
}

// User-Role operations

func (r *iamRepository) CreateUserRole(ctx context.Context, userRole *types.UserRole) error {
	userRole.PrepareForCreate()
	return r.queries.CreateUserRole(ctx, db.CreateUserRoleParams{
		ID:        userRole.ID,
		UserID:    userRole.UserID,
		RoleID:    userRole.RoleID,
		GrantedBy: strPtr(userRole.GrantedBy),
		ExpiresAt: userRole.ExpiresAt,
	})
}

func (r *iamRepository) GetUserRole(ctx context.Context, userID, roleID string) (*types.UserRole, error) {
	dbUserRole, err := r.queries.GetUserRole(ctx, db.GetUserRoleParams{
		UserID: userID,
		RoleID: roleID,
	})
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBUserRoleToModel(dbUserRole), nil
}

func (r *iamRepository) ListUserRolesByUserID(ctx context.Context, userID string) ([]*types.UserRole, error) {
	dbUserRoles, err := r.queries.ListUserRolesByUserID(ctx, userID)
	if err != nil {
		return nil, err
	}
	userRoles := make([]*types.UserRole, len(dbUserRoles))
	for i, ur := range dbUserRoles {
		userRoles[i] = convertDBUserRoleRowToModel(ur)
	}
	return userRoles, nil
}

func (r *iamRepository) ListUserRolesByRoleID(ctx context.Context, roleID string) ([]*types.UserRole, error) {
	dbUserRoles, err := r.queries.ListUserRolesByRoleID(ctx, roleID)
	if err != nil {
		return nil, err
	}
	userRoles := make([]*types.UserRole, len(dbUserRoles))
	for i, ur := range dbUserRoles {
		userRoles[i] = convertDBUserRoleToModel(ur)
	}
	return userRoles, nil
}

func (r *iamRepository) ListUserIDsWithRole(ctx context.Context, roleID string) ([]string, error) {
	return r.queries.ListUserIDsWithRole(ctx, roleID)
}

func (r *iamRepository) DeleteUserRole(ctx context.Context, userID, roleID string) error {
	return r.queries.DeleteUserRole(ctx, db.DeleteUserRoleParams{
		UserID: userID,
		RoleID: roleID,
	})
}

func (r *iamRepository) DeleteUserRolesByUserID(ctx context.Context, userID string) error {
	return r.queries.DeleteUserRolesByUserID(ctx, userID)
}

func (r *iamRepository) DeleteUserRolesByRoleID(ctx context.Context, roleID string) error {
	return r.queries.DeleteUserRolesByRoleID(ctx, roleID)
}

func (r *iamRepository) DeleteExpiredUserRoles(ctx context.Context) error {
	return r.queries.DeleteExpiredUserRoles(ctx, apptime.NewNullTimeNow())
}

// Policy operations

func (r *iamRepository) CreatePolicy(ctx context.Context, policy *types.IAMPolicy) error {
	policy.PrepareForCreate()
	return r.queries.CreatePolicy(ctx, db.CreatePolicyParams{
		ID:    policy.ID,
		Ptype: policy.Ptype,
		V0:    policy.V0,
		V1:    policy.V1,
		V2:    policy.V2,
		V3:    policy.V3,
		V4:    policy.V4,
		V5:    policy.V5,
	})
}

func (r *iamRepository) GetPolicy(ctx context.Context, ptype string, v0, v1, v2, v3, v4, v5 *string) (*types.IAMPolicy, error) {
	// Note: GetPolicy query only uses v0-v3, v4/v5 are ignored
	dbPolicy, err := r.queries.GetPolicy(ctx, db.GetPolicyParams{
		Ptype: ptype,
		V0:    v0,
		V1:    v1,
		V2:    v2,
		V3:    v3,
	})
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBPolicyToModel(dbPolicy), nil
}

func (r *iamRepository) ListPolicies(ctx context.Context) ([]*types.IAMPolicy, error) {
	dbPolicies, err := r.queries.ListPolicies(ctx)
	if err != nil {
		return nil, err
	}
	policies := make([]*types.IAMPolicy, len(dbPolicies))
	for i, p := range dbPolicies {
		policies[i] = convertDBPolicyToModel(p)
	}
	return policies, nil
}

func (r *iamRepository) ListPoliciesByType(ctx context.Context, ptype string) ([]*types.IAMPolicy, error) {
	dbPolicies, err := r.queries.ListPoliciesByType(ctx, ptype)
	if err != nil {
		return nil, err
	}
	policies := make([]*types.IAMPolicy, len(dbPolicies))
	for i, p := range dbPolicies {
		policies[i] = convertDBPolicyToModel(p)
	}
	return policies, nil
}

func (r *iamRepository) ListPoliciesBySubject(ctx context.Context, subject string) ([]*types.IAMPolicy, error) {
	dbPolicies, err := r.queries.ListPoliciesBySubject(ctx, &subject)
	if err != nil {
		return nil, err
	}
	policies := make([]*types.IAMPolicy, len(dbPolicies))
	for i, p := range dbPolicies {
		policies[i] = convertDBPolicyToModel(p)
	}
	return policies, nil
}

func (r *iamRepository) ListGroupingPolicies(ctx context.Context) ([]*types.IAMPolicy, error) {
	dbPolicies, err := r.queries.ListGroupingPolicies(ctx)
	if err != nil {
		return nil, err
	}
	policies := make([]*types.IAMPolicy, len(dbPolicies))
	for i, p := range dbPolicies {
		policies[i] = convertDBPolicyToModel(p)
	}
	return policies, nil
}

func (r *iamRepository) ListGroupingPoliciesByUser(ctx context.Context, userID string) ([]*types.IAMPolicy, error) {
	dbPolicies, err := r.queries.ListGroupingPoliciesByUser(ctx, &userID)
	if err != nil {
		return nil, err
	}
	policies := make([]*types.IAMPolicy, len(dbPolicies))
	for i, p := range dbPolicies {
		policies[i] = convertDBPolicyToModel(p)
	}
	return policies, nil
}

func (r *iamRepository) DeletePolicy(ctx context.Context, id string) error {
	return r.queries.DeletePolicy(ctx, id)
}

func (r *iamRepository) DeletePolicyByValues(ctx context.Context, ptype string, v0, v1, v2, v3, v4, v5 *string) error {
	// Note: DeletePolicyByValues query only uses v0-v3, v4/v5 are ignored
	return r.queries.DeletePolicyByValues(ctx, db.DeletePolicyByValuesParams{
		Ptype: ptype,
		V0:    v0,
		V1:    v1,
		V2:    v2,
		V3:    v3,
	})
}

func (r *iamRepository) DeletePoliciesBySubject(ctx context.Context, subject string) error {
	return r.queries.DeletePoliciesBySubject(ctx, &subject)
}

func (r *iamRepository) DeleteGroupingPoliciesByUser(ctx context.Context, userID string) error {
	return r.queries.DeleteGroupingPoliciesByUser(ctx, &userID)
}

// Audit log operations

func (r *iamRepository) CreateAuditLog(ctx context.Context, log *types.IAMAuditLog) error {
	log.PrepareForCreate()
	metadata, _ := json.Marshal(log.Metadata)
	metadataStr := string(metadata)

	return r.queries.CreateAuditLog(ctx, db.CreateAuditLogParams{
		ID:        log.ID,
		UserID:    strPtr(log.UserID),
		Action:    strPtr(log.Action),
		Resource:  strPtr(log.Resource),
		Result:    strPtr(log.Result),
		Reason:    strPtr(log.Reason),
		IpAddress: strPtr(log.IPAddress),
		UserAgent: strPtr(log.UserAgent),
		Metadata:  &metadataStr,
	})
}

func (r *iamRepository) ListAuditLogs(ctx context.Context, opts repos.AuditLogQueryOptions) (*repos.PaginatedResult[*types.IAMAuditLog], error) {
	dbLogs, err := r.queries.ListAuditLogs(ctx, db.ListAuditLogsParams{
		Limit:  int64(opts.Limit),
		Offset: int64(opts.Offset),
	})
	if err != nil {
		return nil, err
	}

	count, err := r.queries.CountAuditLogs(ctx)
	if err != nil {
		return nil, err
	}

	logs := make([]*types.IAMAuditLog, len(dbLogs))
	for i, l := range dbLogs {
		logs[i] = convertDBAuditLogToModel(l)
	}

	return &repos.PaginatedResult[*types.IAMAuditLog]{
		Items: logs,
		Total: count,
	}, nil
}

func (r *iamRepository) CountAuditLogs(ctx context.Context) (int64, error) {
	return r.queries.CountAuditLogs(ctx)
}

func (r *iamRepository) CountAuditLogsByUserID(ctx context.Context, userID string) (int64, error) {
	return r.queries.CountAuditLogsByUserID(ctx, &userID)
}

func (r *iamRepository) DeleteAuditLogsOlderThan(ctx context.Context, cutoff apptime.Time) error {
	return r.queries.DeleteAuditLogsOlderThan(ctx, apptime.Format(cutoff))
}

// Conversion helpers

func convertDBRoleToModel(dbRole db.IamRole) *types.Role {
	var metadata map[string]interface{}
	if dbRole.Metadata != nil {
		json.Unmarshal([]byte(*dbRole.Metadata), &metadata)
	}

	var displayName, description, roleType string
	if dbRole.DisplayName != nil {
		displayName = *dbRole.DisplayName
	}
	if dbRole.Description != nil {
		description = *dbRole.Description
	}
	if dbRole.Type != nil {
		roleType = *dbRole.Type
	}

	return &types.Role{
		ID:          dbRole.ID,
		Name:        dbRole.Name,
		DisplayName: displayName,
		Description: description,
		Type:        roleType,
		Metadata:    metadata,
		CreatedAt:   apptime.MustParse(dbRole.CreatedAt),
		UpdatedAt:   apptime.MustParse(dbRole.UpdatedAt),
	}
}

func convertDBUserRoleToModel(dbUserRole db.IamUserRole) *types.UserRole {
	var grantedBy string
	if dbUserRole.GrantedBy != nil {
		grantedBy = *dbUserRole.GrantedBy
	}

	return &types.UserRole{
		ID:        dbUserRole.ID,
		UserID:    dbUserRole.UserID,
		RoleID:    dbUserRole.RoleID,
		GrantedBy: grantedBy,
		GrantedAt: dbUserRole.GrantedAt,
		ExpiresAt: dbUserRole.ExpiresAt,
	}
}

func convertDBUserRoleRowToModel(dbUserRole db.ListUserRolesByUserIDRow) *types.UserRole {
	var grantedBy string
	if dbUserRole.GrantedBy != nil {
		grantedBy = *dbUserRole.GrantedBy
	}

	return &types.UserRole{
		ID:        dbUserRole.ID,
		UserID:    dbUserRole.UserID,
		RoleID:    dbUserRole.RoleID,
		GrantedBy: grantedBy,
		GrantedAt: dbUserRole.GrantedAt,
		ExpiresAt: dbUserRole.ExpiresAt,
	}
}

func convertDBPolicyToModel(dbPolicy db.IamPolicy) *types.IAMPolicy {
	return &types.IAMPolicy{
		ID:        dbPolicy.ID,
		Ptype:     dbPolicy.Ptype,
		V0:        dbPolicy.V0,
		V1:        dbPolicy.V1,
		V2:        dbPolicy.V2,
		V3:        dbPolicy.V3,
		V4:        dbPolicy.V4,
		V5:        dbPolicy.V5,
		CreatedAt: apptime.MustParse(dbPolicy.CreatedAt),
	}
}

func convertDBAuditLogToModel(dbLog db.IamAuditLog) *types.IAMAuditLog {
	var metadata map[string]interface{}
	if dbLog.Metadata != nil {
		json.Unmarshal([]byte(*dbLog.Metadata), &metadata)
	}

	var userID, action, resource, result, reason, ipAddress, userAgent string
	if dbLog.UserID != nil {
		userID = *dbLog.UserID
	}
	if dbLog.Action != nil {
		action = *dbLog.Action
	}
	if dbLog.Resource != nil {
		resource = *dbLog.Resource
	}
	if dbLog.Result != nil {
		result = *dbLog.Result
	}
	if dbLog.Reason != nil {
		reason = *dbLog.Reason
	}
	if dbLog.IpAddress != nil {
		ipAddress = *dbLog.IpAddress
	}
	if dbLog.UserAgent != nil {
		userAgent = *dbLog.UserAgent
	}

	return &types.IAMAuditLog{
		ID:        dbLog.ID,
		UserID:    userID,
		Action:    action,
		Resource:  resource,
		Result:    result,
		Reason:    reason,
		IPAddress: ipAddress,
		UserAgent: userAgent,
		Metadata:  metadata,
		CreatedAt: apptime.MustParse(dbLog.CreatedAt),
	}
}

// Ensure iamRepository implements IAMRepository
var _ repos.IAMRepository = (*iamRepository)(nil)
