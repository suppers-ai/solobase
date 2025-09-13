package iam

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/casbin/casbin/v2"
	"github.com/casbin/casbin/v2/model"
	gormadapter "github.com/casbin/gorm-adapter/v3"
	"gorm.io/gorm"
)

// Service provides IAM functionality
type Service struct {
	db       *gorm.DB
	enforcer *casbin.Enforcer
}

// NewService creates a new IAM service
func NewService(db *gorm.DB, modelPath string) (*Service, error) {
	// Create Casbin adapter using existing database connection with custom table name
	adapter, err := gormadapter.NewAdapterByDBWithCustomTable(db, nil, "iam_casbin_rules")
	if err != nil {
		return nil, fmt.Errorf("failed to create Casbin adapter: %w", err)
	}

	// Create enforcer with model and adapter
	enforcer, err := casbin.NewEnforcer(modelPath, adapter)
	if err != nil {
		return nil, fmt.Errorf("failed to create Casbin enforcer: %w", err)
	}

	// Enable auto-save (automatically save policy changes to database)
	enforcer.EnableAutoSave(true)

	// Load existing policies from database
	if err := enforcer.LoadPolicy(); err != nil {
		return nil, fmt.Errorf("failed to load policies: %w", err)
	}

	service := &Service{
		db:       db,
		enforcer: enforcer,
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

// NewServiceWithContent creates a new IAM service with model content as string
func NewServiceWithContent(db *gorm.DB, modelContent string) (*Service, error) {
	// Create Casbin adapter using existing database connection with custom table name
	adapter, err := gormadapter.NewAdapterByDBWithCustomTable(db, nil, "iam_casbin_rules")
	if err != nil {
		return nil, fmt.Errorf("failed to create Casbin adapter: %w", err)
	}

	// Create model from string content
	m, err := model.NewModelFromString(modelContent)
	if err != nil {
		return nil, fmt.Errorf("failed to create model from string: %w", err)
	}

	// Create enforcer with model and adapter
	enforcer, err := casbin.NewEnforcer(m, adapter)
	if err != nil {
		return nil, fmt.Errorf("failed to create Casbin enforcer: %w", err)
	}

	// Enable auto-save (automatically save policy changes to database)
	enforcer.EnableAutoSave(true)

	// Load existing policies from database
	if err := enforcer.LoadPolicy(); err != nil {
		return nil, fmt.Errorf("failed to load policies: %w", err)
	}

	service := &Service{
		db:       db,
		enforcer: enforcer,
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
	return s.db.AutoMigrate(
		&Role{},
		&UserRole{},
		&IAMAuditLog{},
	)
}

// GetEnforcer returns the Casbin enforcer
func (s *Service) GetEnforcer() *casbin.Enforcer {
	return s.enforcer
}

// CheckPermission checks if a user has permission to perform an action on a resource
func (s *Service) CheckPermission(ctx context.Context, userID string, resource string, action string) (bool, error) {
	// Get user email for Casbin (using user ID as subject)
	allowed, err := s.enforcer.Enforce(userID, resource, action, nil)
	if err != nil {
		return false, fmt.Errorf("failed to check permission: %w", err)
	}

	// Log the permission check
	s.logAccess(ctx, userID, resource, action, allowed)

	return allowed, nil
}

// CheckPermissionWithContext checks permission with additional context
func (s *Service) CheckPermissionWithContext(ctx context.Context, userID string, resource string, action string, contextData map[string]interface{}) (bool, error) {
	allowed, err := s.enforcer.Enforce(userID, resource, action, contextData)
	if err != nil {
		return false, fmt.Errorf("failed to check permission: %w", err)
	}

	s.logAccess(ctx, userID, resource, action, allowed)
	return allowed, nil
}

// AssignRole assigns a role to a user
func (s *Service) AssignRole(ctx context.Context, userID string, roleName string, grantedBy string) error {
	// Check if role exists
	var role Role
	if err := s.db.Where("name = ?", roleName).First(&role).Error; err != nil {
		return fmt.Errorf("role not found: %w", err)
	}

	// Add grouping policy in Casbin
	if _, err := s.enforcer.AddGroupingPolicy(userID, roleName); err != nil {
		return fmt.Errorf("failed to assign role: %w", err)
	}

	// Save to UserRole table for tracking
	userRole := &UserRole{
		UserID:    userID,
		RoleID:    role.ID,
		GrantedBy: grantedBy,
	}
	if err := s.db.Create(userRole).Error; err != nil {
		return fmt.Errorf("failed to save user role: %w", err)
	}

	return nil
}

// RemoveRole removes a role from a user
func (s *Service) RemoveRole(ctx context.Context, userID string, roleName string) error {
	// Remove grouping policy in Casbin
	if _, err := s.enforcer.RemoveGroupingPolicy(userID, roleName); err != nil {
		return fmt.Errorf("failed to remove role: %w", err)
	}

	// Remove from UserRole table
	var role Role
	if err := s.db.Where("name = ?", roleName).First(&role).Error; err != nil {
		return fmt.Errorf("role not found: %w", err)
	}

	if err := s.db.Where("user_id = ? AND role_id = ?", userID, role.ID).Delete(&UserRole{}).Error; err != nil {
		return fmt.Errorf("failed to remove user role: %w", err)
	}

	return nil
}

// GetUserRoles gets all roles for a user
func (s *Service) GetUserRoles(ctx context.Context, userID string) ([]string, error) {
	roles, err := s.enforcer.GetRolesForUser(userID)
	if err != nil {
		return nil, fmt.Errorf("failed to get user roles: %w", err)
	}
	return roles, nil
}

// GetUsersWithRoles gets all users with their assigned roles
func (s *Service) GetUsersWithRoles(ctx context.Context) ([]map[string]interface{}, error) {
	// Get all users from the database
	var users []struct {
		ID        string `json:"id"`
		Email     string `json:"email"`
		FirstName string `json:"first_name"`
		LastName  string `json:"last_name"`
	}
	
	if err := s.db.Table("users").Select("id, email, first_name, last_name").Find(&users).Error; err != nil {
		return nil, fmt.Errorf("failed to get users: %w", err)
	}
	
	// Get roles for each user
	result := make([]map[string]interface{}, 0, len(users))
	for _, user := range users {
		roles, _ := s.GetUserRoles(ctx, user.ID)
		
		// Get full role details
		roleDetails := make([]map[string]interface{}, 0, len(roles))
		for _, roleName := range roles {
			var role Role
			if err := s.db.Where("name = ?", roleName).First(&role).Error; err == nil {
				roleDetails = append(roleDetails, map[string]interface{}{
					"name":         role.Name,
					"display_name": role.DisplayName,
				})
			}
		}
		
		result = append(result, map[string]interface{}{
			"id":         user.ID,
			"email":      user.Email,
			"first_name": user.FirstName,
			"last_name":  user.LastName,
			"roles":      roleDetails,
		})
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
func (s *Service) CreateRole(ctx context.Context, role *Role) error {
	if err := s.db.Create(role).Error; err != nil {
		return fmt.Errorf("failed to create role: %w", err)
	}

	// If role has metadata with quotas, add them as policies
	if role.Metadata != nil {
		if err := s.addRoleMetadataPolicies(role.Name, role.Metadata); err != nil {
			return fmt.Errorf("failed to add role metadata policies: %w", err)
		}
	}

	return nil
}

// UpdateRole updates an existing role
func (s *Service) UpdateRole(ctx context.Context, roleID string, updates map[string]interface{}) error {
	var role Role
	if err := s.db.Where("id = ?", roleID).First(&role).Error; err != nil {
		return fmt.Errorf("role not found: %w", err)
	}

	if role.IsSystem {
		return fmt.Errorf("cannot update system role")
	}

	if err := s.db.Model(&role).Updates(updates).Error; err != nil {
		return fmt.Errorf("failed to update role: %w", err)
	}

	// Update metadata policies if metadata changed
	if metadata, ok := updates["metadata"].(map[string]interface{}); ok {
		if err := s.addRoleMetadataPolicies(role.Name, metadata); err != nil {
			return fmt.Errorf("failed to update role metadata policies: %w", err)
		}
	}

	return nil
}

// DeleteRole deletes a role
func (s *Service) DeleteRole(ctx context.Context, roleID string) error {
	var role Role
	if err := s.db.Where("id = ?", roleID).First(&role).Error; err != nil {
		return fmt.Errorf("role not found: %w", err)
	}

	if role.IsSystem {
		return fmt.Errorf("cannot delete system role")
	}

	// Remove all policies for this role
	if _, err := s.enforcer.RemoveFilteredPolicy(0, role.Name); err != nil {
		return fmt.Errorf("failed to remove role policies: %w", err)
	}

	// Remove all user assignments for this role
	if _, err := s.enforcer.RemoveFilteredGroupingPolicy(1, role.Name); err != nil {
		return fmt.Errorf("failed to remove role assignments: %w", err)
	}

	// Delete from database
	if err := s.db.Delete(&role).Error; err != nil {
		return fmt.Errorf("failed to delete role: %w", err)
	}

	return nil
}

// GetRoles gets all roles
func (s *Service) GetRoles(ctx context.Context) ([]Role, error) {
	var roles []Role
	if err := s.db.Find(&roles).Error; err != nil {
		return nil, fmt.Errorf("failed to get roles: %w", err)
	}
	return roles, nil
}

// UserHasRole checks if a user has a specific role
func (s *Service) UserHasRole(userID string, roleName string) (bool, error) {
	roles, err := s.GetUserRoles(context.Background(), userID)
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
	_, err := s.enforcer.AddPolicy(subject, resource, action, effect)
	if err != nil {
		return fmt.Errorf("failed to add policy: %w", err)
	}
	return nil
}

// RemovePolicy removes a policy
func (s *Service) RemovePolicy(ctx context.Context, subject string, resource string, action string, effect string) error {
	_, err := s.enforcer.RemovePolicy(subject, resource, action, effect)
	if err != nil {
		return fmt.Errorf("failed to remove policy: %w", err)
	}
	return nil
}

// GetPolicies gets all policies
func (s *Service) GetPolicies(ctx context.Context) ([][]string, error) {
	policies, err := s.enforcer.GetPolicy()
	if err != nil {
		return nil, fmt.Errorf("failed to get policies: %w", err)
	}
	return policies, nil
}

// GetPoliciesForRole gets policies for a specific role
func (s *Service) GetPoliciesForRole(ctx context.Context, roleName string) ([][]string, error) {
	policies, err := s.enforcer.GetFilteredPolicy(0, roleName)
	if err != nil {
		return nil, fmt.Errorf("failed to get policies for role: %w", err)
	}
	return policies, nil
}

// GetRoleMetadata gets metadata for a role (quotas, limits, etc.)
func (s *Service) GetRoleMetadata(ctx context.Context, roleName string) (*RoleMetadata, error) {
	var role Role
	if err := s.db.Where("name = ?", roleName).First(&role).Error; err != nil {
		return nil, fmt.Errorf("role not found: %w", err)
	}

	if role.Metadata == nil {
		return &RoleMetadata{}, nil
	}

	// Convert metadata to RoleMetadata struct
	metadataJSON, err := json.Marshal(role.Metadata)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal metadata: %w", err)
	}

	var metadata RoleMetadata
	if err := json.Unmarshal(metadataJSON, &metadata); err != nil {
		return nil, fmt.Errorf("failed to unmarshal metadata: %w", err)
	}

	return &metadata, nil
}

// GetUserEffectiveMetadata gets the effective metadata for a user based on all their roles
func (s *Service) GetUserEffectiveMetadata(ctx context.Context, userID string) (*RoleMetadata, error) {
	roles, err := s.GetUserRoles(ctx, userID)
	if err != nil {
		return nil, err
	}

	// Combine metadata from all roles (taking the maximum values)
	effectiveMetadata := &RoleMetadata{}
	
	for _, roleName := range roles {
		metadata, err := s.GetRoleMetadata(ctx, roleName)
		if err != nil {
			continue
		}

		// Access control settings only - quotas are managed by respective extensions
		// Combine allowed IPs
		effectiveMetadata.AllowedIPs = append(effectiveMetadata.AllowedIPs, metadata.AllowedIPs...)
		
		// Combine disabled features (intersection - a feature is disabled only if disabled in all roles)
		if len(effectiveMetadata.DisabledFeatures) == 0 {
			effectiveMetadata.DisabledFeatures = metadata.DisabledFeatures
		}
	}

	return effectiveMetadata, nil
}

// Helper function to add role metadata as policies
// Note: Quotas and limits are now managed by extensions (e.g., CloudStorage)
// This function is kept for potential future metadata-based policies
func (s *Service) addRoleMetadataPolicies(roleName string, metadata map[string]interface{}) error {
	// Extensions handle their own domain-specific limits and quotas
	// IAM only manages access permissions (who can access what endpoints)
	return nil
}

// Helper function to log access attempts
func (s *Service) logAccess(ctx context.Context, userID string, resource string, action string, allowed bool) {
	result := "deny"
	if allowed {
		result = "allow"
	}

	log := &IAMAuditLog{
		UserID:   userID,
		Action:   action,
		Resource: resource,
		Result:   result,
	}

	// Add context data if available
	if ctx.Value("ip_address") != nil {
		log.IPAddress = ctx.Value("ip_address").(string)
	}
	if ctx.Value("user_agent") != nil {
		log.UserAgent = ctx.Value("user_agent").(string)
	}

	// Save log asynchronously to avoid slowing down permission checks
	go func() {
		s.db.Create(log)
	}()
}

// GetAuditLogs retrieves audit logs based on filters
func (s *Service) GetAuditLogs(ctx context.Context, limit string, filter string, logType string) ([]IAMAuditLog, error) {
	query := s.db.Model(&IAMAuditLog{})
	
	// Apply filters
	if filter != "" {
		query = query.Where("action LIKE ? OR resource LIKE ? OR user_id LIKE ?", 
			"%"+filter+"%", "%"+filter+"%", "%"+filter+"%")
	}
	
	if logType != "" && logType != "all" {
		query = query.Where("action = ?", logType)
	}
	
	// Apply limit
	limitInt := 50 // default
	if limit != "" {
		fmt.Sscanf(limit, "%d", &limitInt)
	}
	
	var logs []IAMAuditLog
	if err := query.Order("created_at DESC").Limit(limitInt).Find(&logs).Error; err != nil {
		return nil, fmt.Errorf("failed to get audit logs: %w", err)
	}
	
	return logs, nil
}

// initializeDefaults creates default roles and policies if they don't exist
func (s *Service) initializeDefaults() error {
	// Check if any roles exist
	var count int64
	s.db.Model(&Role{}).Count(&count)
	if count > 0 {
		return nil // Already initialized
	}

	// Create default roles
	defaultRoles := []Role{
		{
			Name:        "admin",
			DisplayName: "Administrator",
			Description: "Full system access",
			IsSystem:    true,
			Metadata:    map[string]interface{}{},
		},
		{
			Name:        "admin_viewer",
			DisplayName: "Admin Viewer",
			Description: "Read-only administrative access",
			IsSystem:    true,
			Metadata:    map[string]interface{}{},
		},
		{
			Name:        "user",
			DisplayName: "User",
			Description: "Standard user access",
			IsSystem:    true,
			Metadata:    map[string]interface{}{},
		},
	}

	for _, role := range defaultRoles {
		if err := s.CreateRole(context.Background(), &role); err != nil {
			return fmt.Errorf("failed to create default role %s: %w", role.Name, err)
		}
	}

	// Add default policies
	defaultPolicies := [][]string{
		// Admin has full access to everything
		{"admin", "*", "*", "allow"},
		
		// Admin Viewer - read-only access to admin panel
		{"admin_viewer", "/api/admin/*", "GET", "allow"},               // Read access to admin endpoints
		{"admin_viewer", "/api/auth/login", "POST", "allow"},           // Allow login
		{"admin_viewer", "/api/auth/logout", "POST", "allow"},          // Allow logout
		{"admin_viewer", "/api/auth/me", "GET", "allow"},               // Allow getting own profile
		{"admin_viewer", "/api/auth/me", "PATCH", "allow"},             // Allow updating own profile
		{"admin_viewer", "/api/auth/change-password", "POST", "allow"}, // Allow changing own password
		
		// User permissions - standard user access
		{"user", "/api/admin/*", "*", "deny"},                          // Block ALL admin endpoints
		{"user", "/api/auth/logout", "POST", "allow"},                  // Logout
		{"user", "/api/auth/me", "GET", "allow"},                       // Get own profile
		{"user", "/api/auth/me", "PATCH", "allow"},                     // Update own profile
		{"user", "/api/auth/change-password", "POST", "allow"},         // Change own password
		{"user", "/api/storage/buckets", "GET|POST", "allow"},          // List and create buckets
		{"user", "/api/storage/buckets/*", "*", "allow"},               // Full access to storage operations
		{"user", "/api/storage/search", "GET", "allow"},                // Search storage
		{"user", "/api/storage/recently-viewed", "*", "allow"},         // Recently viewed items
		{"user", "/api/storage/quota", "GET", "allow"},                 // View own quota
		{"user", "/api/storage/stats", "GET", "allow"},                 // View own stats
		{"user", "/api/settings", "GET", "allow"},                      // View settings only
		{"user", "/api/dashboard/stats", "GET", "allow"},               // View dashboard
		// Allow access to user-facing extension endpoints
		{"user", "/api/ext/analytics/*", "*", "allow"},                 // Analytics tracking
		{"user", "/api/ext/products/products", "GET", "allow"},         // View products
		{"user", "/api/ext/products/groups", "*", "allow"},             // Manage own groups
		{"user", "/api/ext/products/calculate-price", "POST", "allow"}, // Calculate prices
		{"user", "/api/ext/cloudstorage/shares", "*", "allow"},         // Manage own shares
		{"user", "/api/ext/webhooks/webhooks", "GET", "allow"},         // View webhooks only
	}

	for _, policy := range defaultPolicies {
		// Convert []string to []interface{}
		policyInterface := make([]interface{}, len(policy))
		for i, v := range policy {
			policyInterface[i] = v
		}
		if _, err := s.enforcer.AddPolicy(policyInterface...); err != nil {
			return fmt.Errorf("failed to add default policy: %w", err)
		}
	}

	return s.enforcer.SavePolicy()
}