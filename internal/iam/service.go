package iam

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"strings"

	"github.com/suppers-ai/solobase/internal/iam/types"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

// Service provides IAM functionality using repository pattern.
// This implementation works with both standard Go and TinyGo/WASM builds.
// No in-memory RBAC engine is used - all permission checks query the repository.
type Service struct {
	db   *sql.DB             // For migrations only
	repo repos.IAMRepository // For all operations
}

// NewService creates a new IAM service with repository.
func NewService(db *sql.DB, repo repos.IAMRepository) (*Service, error) {
	service := &Service{
		db:   db,
		repo: repo,
	}

	// Run migrations for IAM tables
	if err := service.Migrate(); err != nil {
		return nil, fmt.Errorf("failed to run IAM migrations: %w", err)
	}

	// Initialize default roles and policies if needed
	if err := service.initializeDefaults(); err != nil {
		return nil, fmt.Errorf("failed to initialize defaults: %w", err)
	}

	return service, nil
}

// Migrate runs database migrations for IAM tables
func (s *Service) Migrate() error {
	// Skip migrations if db is nil (WASM mode - host handles schema)
	if s.db == nil {
		return nil
	}

	// Create iam_roles table
	_, err := s.db.Exec(`
		CREATE TABLE IF NOT EXISTS iam_roles (
			id TEXT PRIMARY KEY,
			name TEXT NOT NULL UNIQUE,
			display_name TEXT,
			description TEXT,
			type TEXT DEFAULT 'custom',
			metadata TEXT,
			created_at TEXT NOT NULL,
			updated_at TEXT NOT NULL
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create iam_roles table: %w", err)
	}

	// Create iam_user_roles table
	_, err = s.db.Exec(`
		CREATE TABLE IF NOT EXISTS iam_user_roles (
			id TEXT PRIMARY KEY,
			user_id TEXT NOT NULL,
			role_id TEXT NOT NULL,
			granted_by TEXT,
			granted_at TEXT NOT NULL,
			expires_at TEXT,
			UNIQUE(user_id, role_id)
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create iam_user_roles table: %w", err)
	}

	// Create iam_audit_logs table
	_, err = s.db.Exec(`
		CREATE TABLE IF NOT EXISTS iam_audit_logs (
			id TEXT PRIMARY KEY,
			user_id TEXT NOT NULL,
			action TEXT NOT NULL,
			resource TEXT NOT NULL,
			result TEXT NOT NULL,
			reason TEXT,
			ip_address TEXT,
			user_agent TEXT,
			metadata TEXT,
			created_at TEXT NOT NULL
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create iam_audit_logs table: %w", err)
	}

	// Create iam_policies table for persistence
	_, err = s.db.Exec(`
		CREATE TABLE IF NOT EXISTS iam_policies (
			id TEXT PRIMARY KEY,
			ptype TEXT NOT NULL,
			v0 TEXT,
			v1 TEXT,
			v2 TEXT,
			v3 TEXT,
			v4 TEXT,
			v5 TEXT,
			created_at TEXT NOT NULL
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create iam_policies table: %w", err)
	}

	// Create iam_groupings table for user-role assignments persistence
	_, err = s.db.Exec(`
		CREATE TABLE IF NOT EXISTS iam_groupings (
			id TEXT PRIMARY KEY,
			user_id TEXT NOT NULL,
			role_name TEXT NOT NULL,
			created_at TEXT NOT NULL,
			UNIQUE(user_id, role_name)
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create iam_groupings table: %w", err)
	}

	return nil
}

// CheckPermission checks if a user has permission to perform an action on a resource
func (s *Service) CheckPermission(ctx context.Context, userID string, resource string, action string) (bool, error) {
	// Get user's roles
	roles, err := s.GetUserRoles(ctx, userID)
	if err != nil {
		return false, err
	}

	// Admin bypass - if user has admin role, always allow
	for _, role := range roles {
		if role == "admin" {
			s.logAccess(ctx, userID, resource, action, true)
			return true, nil
		}
	}

	if len(roles) == 0 {
		s.logAccess(ctx, userID, resource, action, false)
		return false, nil
	}

	// Check policies for each role - deny takes precedence over allow
	var hasAllow bool
	var hasDeny bool

	for _, roleName := range roles {
		policies, err := s.repo.ListPoliciesBySubject(ctx, roleName)
		if err != nil {
			continue
		}

		for _, policy := range policies {
			if policy.Ptype != "p" {
				continue
			}

			// Get policy values
			var object, policyAction, effect string
			if policy.V1 != nil {
				object = *policy.V1
			}
			if policy.V2 != nil {
				policyAction = *policy.V2
			}
			if policy.V3 != nil {
				effect = *policy.V3
			}

			// Check path match
			if !matchPath(resource, object) {
				continue
			}

			// Check action match
			if !matchAction(action, policyAction) {
				continue
			}

			// Policy matches
			if effect == "deny" {
				hasDeny = true
			}
			if effect == "allow" {
				hasAllow = true
			}
		}
	}

	// Deny takes precedence
	allowed := hasAllow && !hasDeny
	s.logAccess(ctx, userID, resource, action, allowed)
	return allowed, nil
}

// CheckPermissionWithContext checks permission with additional context
func (s *Service) CheckPermissionWithContext(ctx context.Context, userID string, resource string, action string, contextData map[string]interface{}) (bool, error) {
	// Context data is not used in this simplified implementation
	return s.CheckPermission(ctx, userID, resource, action)
}

// GetUserRoles gets all roles for a user
func (s *Service) GetUserRoles(ctx context.Context, userID string) ([]string, error) {
	groupings, err := s.repo.ListGroupingPoliciesByUser(ctx, userID)
	if err != nil {
		return nil, err
	}

	roles := make([]string, 0, len(groupings))
	for _, g := range groupings {
		if g.V1 != nil {
			roles = append(roles, *g.V1)
		}
	}
	return roles, nil
}

// AssignRole assigns a role to a user
func (s *Service) AssignRole(ctx context.Context, userID string, roleName string, grantedBy string) error {
	// Check if role exists
	role, err := s.repo.GetRoleByName(ctx, roleName)
	if err != nil {
		return fmt.Errorf("role not found: %w", err)
	}

	// Check if user already has this role (via iam_user_roles)
	_, err = s.repo.GetUserRole(ctx, userID, role.ID)
	if err == nil {
		// types.Role already assigned
		return nil
	}

	// Create grouping policy (iam_groupings table)
	groupingPolicy := &types.IAMPolicy{
		Ptype: "g",
		V0:    &userID,
		V1:    &roleName,
	}
	if err := s.repo.CreatePolicy(ctx, groupingPolicy); err != nil {
		// Ignore duplicate error (OR IGNORE in SQL)
		if !strings.Contains(err.Error(), "UNIQUE constraint") {
			return fmt.Errorf("failed to save grouping: %w", err)
		}
	}

	// Save to types.UserRole table for tracking
	userRole := &types.UserRole{
		UserID:    userID,
		RoleID:    role.ID,
		GrantedBy: grantedBy,
	}
	if err := s.repo.CreateUserRole(ctx, userRole); err != nil {
		return fmt.Errorf("failed to save user role: %w", err)
	}

	return nil
}

// RemoveRole removes a role from a user
func (s *Service) RemoveRole(ctx context.Context, userID string, roleName string) error {
	// Remove grouping policy
	if err := s.repo.DeleteGroupingPoliciesByUser(ctx, userID); err != nil {
		return fmt.Errorf("failed to remove grouping: %w", err)
	}

	// Get role ID
	role, err := s.repo.GetRoleByName(ctx, roleName)
	if err != nil {
		return fmt.Errorf("role not found: %w", err)
	}

	// Remove from types.UserRole table
	if err := s.repo.DeleteUserRole(ctx, userID, role.ID); err != nil {
		return fmt.Errorf("failed to remove user role: %w", err)
	}

	return nil
}

// GetUsersWithRoles gets all users with their assigned roles
func (s *Service) GetUsersWithRoles(ctx context.Context) ([]map[string]interface{}, error) {
	// This requires joining with auth_users table which is outside IAM repository
	// We'll query using raw SQL as this is a cross-domain query
	rows, err := s.db.Query(`
		SELECT id, email, first_name, last_name, last_login, created_at
		FROM auth_users
	`)
	if err != nil {
		return nil, fmt.Errorf("failed to get users: %w", err)
	}
	defer rows.Close()

	var result []map[string]interface{}
	for rows.Next() {
		var id, email string
		var firstName, lastName sql.NullString
		var lastLogin sql.NullString
		var createdAt string

		if err := rows.Scan(&id, &email, &firstName, &lastName, &lastLogin, &createdAt); err != nil {
			return nil, err
		}

		roles, _ := s.GetUserRoles(ctx, id)

		// Get role details
		roleDetails := make([]map[string]interface{}, 0)
		for _, roleName := range roles {
			role, err := s.repo.GetRoleByName(ctx, roleName)
			displayName := roleName
			if err == nil {
				displayName = role.DisplayName
			}
			roleDetails = append(roleDetails, map[string]interface{}{
				"name":         roleName,
				"display_name": displayName,
			})
		}

		user := map[string]interface{}{
			"id":         id,
			"email":      email,
			"first_name": firstName.String,
			"last_name":  lastName.String,
			"created_at": createdAt,
			"roles":      roleDetails,
		}
		if lastLogin.Valid {
			user["last_login"] = lastLogin.String
		}

		result = append(result, user)
	}

	return result, nil
}

// AssignRoleToUser assigns a role to a user
func (s *Service) AssignRoleToUser(ctx context.Context, userID string, roleName string) error {
	return s.AssignRole(ctx, userID, roleName, "system")
}

// RemoveRoleFromUser removes a role from a user
func (s *Service) RemoveRoleFromUser(ctx context.Context, userID string, roleName string) error {
	return s.RemoveRole(ctx, userID, roleName)
}

// CreateRole creates a new role
func (s *Service) CreateRole(ctx context.Context, role *types.Role) error {
	return s.repo.CreateRole(ctx, role)
}

// UpdateRole updates an existing role
func (s *Service) UpdateRole(ctx context.Context, roleID string, updates map[string]interface{}) error {
	// Check if role exists and is not a system role
	role, err := s.repo.GetRole(ctx, roleID)
	if err != nil {
		return fmt.Errorf("role not found: %w", err)
	}

	if role.Type == "system" {
		return fmt.Errorf("cannot update system role")
	}

	// Apply updates
	if displayName, ok := updates["display_name"].(string); ok {
		role.DisplayName = displayName
	}
	if description, ok := updates["description"].(string); ok {
		role.Description = description
	}

	return s.repo.UpdateRole(ctx, role)
}

// DeleteRole deletes a role
func (s *Service) DeleteRole(ctx context.Context, roleID string) error {
	// Check if role exists and is not a system role
	role, err := s.repo.GetRole(ctx, roleID)
	if err != nil {
		return fmt.Errorf("role not found: %w", err)
	}

	if role.Type == "system" {
		return fmt.Errorf("cannot delete system role")
	}

	// Delete all policies for this role
	if err := s.repo.DeletePoliciesBySubject(ctx, role.Name); err != nil {
		return err
	}

	// Delete all user role assignments for this role
	if err := s.repo.DeleteUserRolesByRoleID(ctx, roleID); err != nil {
		return err
	}

	// Delete role itself
	return s.repo.DeleteRole(ctx, roleID)
}

// GetRoles gets all roles
func (s *Service) GetRoles(ctx context.Context) ([]types.Role, error) {
	roles, err := s.repo.ListRoles(ctx)
	if err != nil {
		return nil, err
	}

	result := make([]types.Role, len(roles))
	for i, r := range roles {
		result[i] = *r
	}
	return result, nil
}

// GetRoleByID gets a role by ID
func (s *Service) GetRoleByID(ctx context.Context, roleID string) (*types.Role, error) {
	return s.repo.GetRole(ctx, roleID)
}

// GetRoleByName gets a role by name
func (s *Service) GetRoleByName(ctx context.Context, roleName string) (*types.Role, error) {
	return s.repo.GetRoleByName(ctx, roleName)
}

// UserHasRole checks if a user has a specific role
func (s *Service) UserHasRole(userID string, roleName string) (bool, error) {
	ctx := context.Background()
	roles, err := s.GetUserRoles(ctx, userID)
	if err != nil {
		return false, err
	}
	for _, role := range roles {
		if role == roleName {
			return true, nil
		}
	}
	return false, nil
}

// AddPolicy adds a policy
func (s *Service) AddPolicy(ctx context.Context, subject string, resource string, action string, effect string) error {
	policy := &types.IAMPolicy{
		Ptype: "p",
		V0:    &subject,
		V1:    &resource,
		V2:    &action,
		V3:    &effect,
	}
	return s.repo.CreatePolicy(ctx, policy)
}

// RemovePolicy removes a policy
func (s *Service) RemovePolicy(ctx context.Context, subject string, resource string, action string, effect string) error {
	return s.repo.DeletePolicyByValues(ctx, "p", &subject, &resource, &action, &effect, nil, nil)
}

// GetPolicies gets all policies
func (s *Service) GetPolicies(ctx context.Context) ([][]string, error) {
	policies, err := s.repo.ListPoliciesByType(ctx, "p")
	if err != nil {
		return nil, err
	}

	result := make([][]string, 0, len(policies))
	for _, p := range policies {
		if p.V0 == nil || p.V1 == nil || p.V2 == nil || p.V3 == nil {
			continue
		}
		result = append(result, []string{*p.V0, *p.V1, *p.V2, *p.V3})
	}
	return result, nil
}

// GetPoliciesForRole gets policies for a specific role
func (s *Service) GetPoliciesForRole(ctx context.Context, roleName string) ([][]string, error) {
	policies, err := s.repo.ListPoliciesBySubject(ctx, roleName)
	if err != nil {
		return nil, err
	}

	result := make([][]string, 0, len(policies))
	for _, p := range policies {
		if p.Ptype != "p" || p.V0 == nil || p.V1 == nil || p.V2 == nil || p.V3 == nil {
			continue
		}
		result = append(result, []string{*p.V0, *p.V1, *p.V2, *p.V3})
	}
	return result, nil
}

// GetRoleMetadata gets metadata for a role
func (s *Service) GetRoleMetadata(ctx context.Context, roleName string) (*types.RoleMetadata, error) {
	role, err := s.repo.GetRoleByName(ctx, roleName)
	if err != nil {
		return nil, fmt.Errorf("role not found: %w", err)
	}

	if role.Metadata == nil {
		return &types.RoleMetadata{}, nil
	}

	// Convert map to types.RoleMetadata
	data, err := json.Marshal(role.Metadata)
	if err != nil {
		return &types.RoleMetadata{}, nil
	}

	var metadata types.RoleMetadata
	if err := json.Unmarshal(data, &metadata); err != nil {
		return &types.RoleMetadata{}, nil
	}

	return &metadata, nil
}

// GetUserEffectiveMetadata gets the effective metadata for a user based on all their roles
func (s *Service) GetUserEffectiveMetadata(ctx context.Context, userID string) (*types.RoleMetadata, error) {
	roles, err := s.GetUserRoles(ctx, userID)
	if err != nil {
		return nil, err
	}

	effectiveMetadata := &types.RoleMetadata{}

	for _, roleName := range roles {
		metadata, err := s.GetRoleMetadata(ctx, roleName)
		if err != nil {
			continue
		}

		effectiveMetadata.AllowedIPs = append(effectiveMetadata.AllowedIPs, metadata.AllowedIPs...)

		if len(effectiveMetadata.DisabledFeatures) == 0 {
			effectiveMetadata.DisabledFeatures = metadata.DisabledFeatures
		}
	}

	return effectiveMetadata, nil
}

// logAccess logs access attempts
func (s *Service) logAccess(ctx context.Context, userID string, resource string, action string, allowed bool) {
	result := "deny"
	if allowed {
		result = "allow"
	}

	ipAddress := ""
	userAgent := ""
	if ctx.Value("ip_address") != nil {
		ipAddress = ctx.Value("ip_address").(string)
	}
	if ctx.Value("user_agent") != nil {
		userAgent = ctx.Value("user_agent").(string)
	}

	log := &types.IAMAuditLog{
		UserID:    userID,
		Action:    action,
		Resource:  resource,
		Result:    result,
		IPAddress: ipAddress,
		UserAgent: userAgent,
	}
	s.repo.CreateAuditLog(ctx, log)
}

// GetAuditLogs retrieves audit logs
func (s *Service) GetAuditLogs(ctx context.Context) ([]types.IAMAuditLog, error) {
	result, err := s.repo.ListAuditLogs(ctx, repos.AuditLogQueryOptions{
		Pagination: repos.Pagination{
			Limit:  50,
			Offset: 0,
		},
	})
	if err != nil {
		return nil, err
	}

	logs := make([]types.IAMAuditLog, len(result.Items))
	for i, l := range result.Items {
		logs[i] = *l
	}
	return logs, nil
}

// GetAuditLogsFiltered retrieves audit logs based on filters
func (s *Service) GetAuditLogsFiltered(ctx context.Context, limit string, filter string, logType string) ([]types.IAMAuditLog, error) {
	limitInt := 50
	if limit != "" {
		fmt.Sscanf(limit, "%d", &limitInt)
	}

	opts := repos.AuditLogQueryOptions{
		Pagination: repos.Pagination{
			Limit:  limitInt,
			Offset: 0,
		},
	}

	if logType != "" && logType != "all" {
		opts.Action = &logType
	}

	result, err := s.repo.ListAuditLogs(ctx, opts)
	if err != nil {
		return nil, err
	}

	// Apply filter in memory (search across multiple fields)
	logs := make([]types.IAMAuditLog, 0, len(result.Items))
	for _, l := range result.Items {
		if filter == "" {
			logs = append(logs, *l)
			continue
		}

		// Check if filter matches any field
		if strings.Contains(l.Action, filter) ||
			strings.Contains(l.Resource, filter) ||
			strings.Contains(l.UserID, filter) {
			logs = append(logs, *l)
		}
	}

	return logs, nil
}

// GetAuditLogsByUser retrieves audit logs for a specific user
func (s *Service) GetAuditLogsByUser(ctx context.Context, userID string) ([]types.IAMAuditLog, error) {
	result, err := s.repo.ListAuditLogs(ctx, repos.AuditLogQueryOptions{
		UserID: &userID,
		Pagination: repos.Pagination{
			Limit:  50,
			Offset: 0,
		},
	})
	if err != nil {
		return nil, err
	}

	logs := make([]types.IAMAuditLog, len(result.Items))
	for i, l := range result.Items {
		logs[i] = *l
	}
	return logs, nil
}

// initializeDefaults creates default roles and policies if they don't exist
func (s *Service) initializeDefaults() error {
	ctx := context.Background()

	// Check if any roles exist
	roles, err := s.repo.ListRoles(ctx)
	if err != nil {
		// In WASM mode, repository returns "not implemented" - skip initialization
		// The host is responsible for seeding default roles
		if strings.Contains(err.Error(), "not implemented") {
			return nil
		}
		// For other errors, we can still try to initialize
	}
	if len(roles) > 0 {
		return nil // Already initialized
	}

	// Create default roles
	defaultRoles := []types.Role{
		{
			Name:        "admin",
			DisplayName: "Administrator",
			Description: "Full system access",
			Type:        "system",
			Metadata:    map[string]interface{}{},
		},
		{
			Name:        "admin_viewer",
			DisplayName: "Admin Viewer",
			Description: "Read-only administrative access",
			Type:        "system",
			Metadata:    map[string]interface{}{},
		},
		{
			Name:        "user",
			DisplayName: "User",
			Description: "Standard user access",
			Type:        "system",
			Metadata:    map[string]interface{}{},
		},
	}

	for _, role := range defaultRoles {
		if err := s.CreateRole(ctx, &role); err != nil {
			return fmt.Errorf("failed to create default role %s: %w", role.Name, err)
		}
	}

	// Add default policies
	defaultPolicies := [][]string{
		{"admin", "*", "*", "allow"},
		{"admin_viewer", "/api/admin/*", "GET", "allow"},
		{"admin_viewer", "/api/auth/login", "POST", "allow"},
		{"admin_viewer", "/api/auth/logout", "POST", "allow"},
		{"admin_viewer", "/api/auth/me", "GET", "allow"},
		{"admin_viewer", "/api/auth/me", "PATCH", "allow"},
		{"admin_viewer", "/api/auth/change-password", "POST", "allow"},
		{"user", "/api/admin/*", "*", "deny"},
		{"user", "/api/auth/logout", "POST", "allow"},
		{"user", "/api/auth/me", "GET", "allow"},
		{"user", "/api/auth/me", "PATCH", "allow"},
		{"user", "/api/auth/change-password", "POST", "allow"},
		{"user", "/api/storage/buckets", "GET|POST", "allow"},
		{"user", "/api/storage/buckets/*", "*", "allow"},
		{"user", "/api/storage/search", "GET", "allow"},
		{"user", "/api/storage/recently-viewed", "*", "allow"},
		{"user", "/api/storage/quota", "GET", "allow"},
		{"user", "/api/storage/stats", "GET", "allow"},
		{"user", "/api/settings", "GET", "allow"},
		{"user", "/api/dashboard/stats", "GET", "allow"},
		{"user", "/api/ext/products/products", "GET", "allow"},
		{"user", "/api/ext/products/groups", "*", "allow"},
		{"user", "/api/ext/products/calculate-price", "POST", "allow"},
		{"user", "/api/ext/cloudstorage/shares", "*", "allow"},
	}

	for _, policy := range defaultPolicies {
		if err := s.AddPolicy(ctx, policy[0], policy[1], policy[2], policy[3]); err != nil {
			return fmt.Errorf("failed to add default policy: %w", err)
		}
	}

	return nil
}

// Pattern matching functions

// matchPath checks if a request path matches a policy pattern
// Supports:
// - * at end matches any suffix (e.g., "/api/admin/*" matches "/api/admin/users")
// - * alone matches everything
// - :param matches a single path segment (e.g., "/users/:id" matches "/users/123")
// - Exact match
func matchPath(requestPath, pattern string) bool {
	// Wildcard match all
	if pattern == "*" {
		return true
	}

	// Trailing wildcard
	if strings.HasSuffix(pattern, "*") {
		prefix := strings.TrimSuffix(pattern, "*")
		return strings.HasPrefix(requestPath, prefix)
	}

	// Check for path parameter patterns (:param)
	if strings.Contains(pattern, ":") {
		return matchPathWithParams(requestPath, pattern)
	}

	// Exact match
	return requestPath == pattern
}

// matchPathWithParams matches paths with :param patterns
func matchPathWithParams(requestPath, pattern string) bool {
	reqParts := strings.Split(requestPath, "/")
	patParts := strings.Split(pattern, "/")

	if len(reqParts) != len(patParts) {
		return false
	}

	for i, patPart := range patParts {
		if strings.HasPrefix(patPart, ":") {
			// Parameter segment - matches any non-empty value
			if reqParts[i] == "" {
				return false
			}
			continue
		}
		if patPart != reqParts[i] {
			return false
		}
	}

	return true
}

// strPtr is a helper to get a pointer to a string
func strPtr(s string) *string {
	if s == "" {
		return nil
	}
	return &s
}

// Backward compatibility - keep the old constructor name as alias
func NewServiceWithDB(db *sql.DB, repo repos.IAMRepository) (*Service, error) {
	return NewService(db, repo)
}
