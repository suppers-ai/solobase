package iam

import (
	"fmt"
	"log"
)

// SetupDemoUser creates a demo user with restricted permissions
func (s *Service) SetupDemoUser(email, password string) error {
	// Check if demo role exists, if not it was created during initialization
	var demoRole Role
	if err := s.db.Where("name = ?", "restricted").First(&demoRole).Error; err != nil {
		log.Printf("Restricted role not found, it should have been created during initialization")
		return nil
	}
	
	// The user will be created by the Auth service with the provided credentials
	// We just need to ensure they get the restricted role when they're created
	
	// Add a policy that automatically assigns the restricted role to the demo email
	s.enforcer.AddGroupingPolicy(email, "restricted")
	
	// Add specific demo user policies (in addition to role policies)
	demoUserPolicies := [][]string{
		// Allow read access to everything
		{email, "/api/*", "GET", "allow"},
		{email, "/api/*", "read", "allow"},
		{email, "/api/*", "list", "allow"},
		
		// Explicitly deny dangerous operations
		{email, "/api/*/delete", "*", "deny"},
		{email, "/api/*/bulk", "*", "deny"},
		{email, "/api/webhooks/*", "*", "deny"},
		{email, "/api/settings", "write", "deny"},
		{email, "/api/users", "write", "deny"},
		{email, "/api/iam/*", "write", "deny"},
		
		// Allow limited uploads with size restrictions
		{email, "/api/storage/upload", "POST", "allow"},
	}
	
	for _, policy := range demoUserPolicies {
		// Convert []string to []interface{}
		policyInterface := make([]interface{}, len(policy))
		for i, v := range policy {
			policyInterface[i] = v
		}
		if _, err := s.enforcer.AddPolicy(policyInterface...); err != nil {
			log.Printf("Failed to add demo policy: %v", err)
		}
	}
	
	// Save policies
	if err := s.enforcer.SavePolicy(); err != nil {
		return fmt.Errorf("failed to save demo policies: %w", err)
	}
	
	log.Printf("Demo user policies configured for %s with restricted role", email)
	return nil
}

// SetupFlyDemoEnvironment configures the system for Fly.io demo deployment
func (s *Service) SetupFlyDemoEnvironment() error {
	
	// Update restricted role with even tighter limits for demo
	var restrictedRole Role
	if err := s.db.Where("name = ?", "restricted").First(&restrictedRole).Error; err == nil {
		// Update metadata for demo environment
		restrictedRole.Metadata = map[string]interface{}{
			"storage_quota":        52428800,   // 50MB total storage
			"bandwidth_quota":      524288000,  // 500MB bandwidth per month
			"max_upload_size":      5242880,    // 5MB per file
			"max_requests_per_min": 30,         // 30 requests per minute
			"session_timeout":      1800,       // 30 minute sessions
			"disabled_features": []string{
				"webhooks",
				"external_api", 
				"bulk_operations",
				"user_management",
				"settings_write",
				"database_write",
			},
		}
		
		if err := s.db.Save(&restrictedRole).Error; err != nil {
			log.Printf("Failed to update restricted role for demo: %v", err)
		}
		
		// Update policies in Casbin
		s.addRoleMetadataPolicies("restricted", restrictedRole.Metadata)
	}
	
	// Add additional system-wide policies for demo environment
	demoPolicies := [][]string{
		// Prevent all users from accessing certain endpoints
		{"*", "/api/system/shutdown", "*", "deny"},
		{"*", "/api/system/restart", "*", "deny"},
		{"*", "/api/database/execute", "*", "deny"},
		{"*", "/api/database/drop", "*", "deny"},
		
		// Rate limiting policies
		{"restricted", "rate_limit", "max_requests", "30"},
		{"restricted", "session", "timeout", "1800"},
	}
	
	for _, policy := range demoPolicies {
		// Convert []string to []interface{}
		policyInterface := make([]interface{}, len(policy))
		for i, v := range policy {
			policyInterface[i] = v
		}
		if _, err := s.enforcer.AddPolicy(policyInterface...); err != nil {
			log.Printf("Failed to add demo environment policy: %v", err)
		}
	}
	
	// Create a public share policy for demo content
	s.enforcer.AddPolicy("restricted", "/api/shares/public/*", "read", "allow")
	
	// Save all policies
	if err := s.enforcer.SavePolicy(); err != nil {
		return fmt.Errorf("failed to save demo environment policies: %w", err)
	}
	
	log.Println("Fly.io demo environment configured with restricted policies")
	return nil
}