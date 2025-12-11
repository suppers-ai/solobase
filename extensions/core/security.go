package core

import (
	"context"
	"fmt"
	"net/http"
	"sync"
	"time"
)

// SecurityManager handles extension security and permissions
type SecurityManager struct {
	mu          sync.RWMutex
	permissions map[string]map[string]bool // extension -> permission -> allowed
	quotas      map[string]*ResourceQuota
	rateLimits  map[string]*RateLimiter
	auditLog    []AuditEntry
}

// NewSecurityManager creates a new security manager
func NewSecurityManager() *SecurityManager {
	return &SecurityManager{
		permissions: make(map[string]map[string]bool),
		quotas:      make(map[string]*ResourceQuota),
		rateLimits:  make(map[string]*RateLimiter),
		auditLog:    []AuditEntry{},
	}
}

// ResourceQuota defines resource limits for an extension
type ResourceQuota struct {
	MaxMemoryMB       int64
	MaxGoroutines     int
	MaxDatabaseConns  int
	MaxRequestsPerSec int
	MaxStorageMB      int64

	// Current usage
	CurrentMemoryMB   int64
	CurrentGoroutines int
	CurrentDBConns    int
	CurrentStorageMB  int64
}

// RateLimiter implements token bucket rate limiting
type RateLimiter struct {
	tokens     float64
	maxTokens  float64
	refillRate float64
	lastRefill time.Time
	mu         sync.Mutex
}

// NewRateLimiter creates a new rate limiter
func NewRateLimiter(requestsPerSecond int) *RateLimiter {
	return &RateLimiter{
		tokens:     float64(requestsPerSecond),
		maxTokens:  float64(requestsPerSecond),
		refillRate: float64(requestsPerSecond),
		lastRefill: time.Now(),
	}
}

// Allow checks if a request is allowed
func (r *RateLimiter) Allow() bool {
	r.mu.Lock()
	defer r.mu.Unlock()

	// Refill tokens
	now := time.Now()
	elapsed := now.Sub(r.lastRefill).Seconds()
	r.tokens = min(r.maxTokens, r.tokens+elapsed*r.refillRate)
	r.lastRefill = now

	// Check if we have tokens
	if r.tokens >= 1 {
		r.tokens--
		return true
	}

	return false
}

func min(a, b float64) float64 {
	if a < b {
		return a
	}
	return b
}

// AuditEntry represents a security audit log entry
type AuditEntry struct {
	Timestamp time.Time              `json:"timestamp"`
	Extension string                 `json:"extension"`
	Action    string                 `json:"action"`
	Resource  string                 `json:"resource"`
	UserID    string                 `json:"userId"`
	Result    string                 `json:"result"`
	Details   map[string]interface{} `json:"details"`
}

// CheckPermission checks if an extension has a specific permission
func (sm *SecurityManager) CheckPermission(extension, permission string) bool {
	sm.mu.RLock()
	defer sm.mu.RUnlock()

	if perms, exists := sm.permissions[extension]; exists {
		return perms[permission]
	}

	return false
}

// GrantPermission grants a permission to an extension
func (sm *SecurityManager) GrantPermission(extension, permission string) {
	sm.mu.Lock()
	defer sm.mu.Unlock()

	if sm.permissions[extension] == nil {
		sm.permissions[extension] = make(map[string]bool)
	}

	sm.permissions[extension][permission] = true

	sm.addAuditEntry(AuditEntry{
		Timestamp: time.Now(),
		Extension: extension,
		Action:    "grant_permission",
		Resource:  permission,
		Result:    "success",
	})
}

// RevokePermission revokes a permission from an extension
func (sm *SecurityManager) RevokePermission(extension, permission string) {
	sm.mu.Lock()
	defer sm.mu.Unlock()

	if perms, exists := sm.permissions[extension]; exists {
		delete(perms, permission)
	}

	sm.addAuditEntry(AuditEntry{
		Timestamp: time.Now(),
		Extension: extension,
		Action:    "revoke_permission",
		Resource:  permission,
		Result:    "success",
	})
}

// SetResourceQuota sets resource quota for an extension
func (sm *SecurityManager) SetResourceQuota(extension string, quota *ResourceQuota) {
	sm.mu.Lock()
	defer sm.mu.Unlock()

	sm.quotas[extension] = quota
}

// CheckResourceQuota checks if an extension is within its resource quota
func (sm *SecurityManager) CheckResourceQuota(extension string, resource string, amount int64) error {
	sm.mu.RLock()
	defer sm.mu.RUnlock()

	quota, exists := sm.quotas[extension]
	if !exists {
		return nil // No quota set, allow
	}

	switch resource {
	case "memory":
		if quota.CurrentMemoryMB+amount > quota.MaxMemoryMB {
			return fmt.Errorf("memory quota exceeded: %d/%d MB", quota.CurrentMemoryMB+amount, quota.MaxMemoryMB)
		}
	case "goroutines":
		if quota.CurrentGoroutines+int(amount) > quota.MaxGoroutines {
			return fmt.Errorf("goroutine quota exceeded: %d/%d", quota.CurrentGoroutines+int(amount), quota.MaxGoroutines)
		}
	case "storage":
		if quota.CurrentStorageMB+amount > quota.MaxStorageMB {
			return fmt.Errorf("storage quota exceeded: %d/%d MB", quota.CurrentStorageMB+amount, quota.MaxStorageMB)
		}
	}

	return nil
}

// UpdateResourceUsage updates current resource usage for an extension
func (sm *SecurityManager) UpdateResourceUsage(extension string, resource string, amount int64) {
	sm.mu.Lock()
	defer sm.mu.Unlock()

	quota, exists := sm.quotas[extension]
	if !exists {
		return
	}

	switch resource {
	case "memory":
		quota.CurrentMemoryMB = amount
	case "goroutines":
		quota.CurrentGoroutines = int(amount)
	case "storage":
		quota.CurrentStorageMB = amount
	}
}

// CheckRateLimit checks if an extension request is rate limited
func (sm *SecurityManager) CheckRateLimit(extension string) bool {
	sm.mu.RLock()
	limiter, exists := sm.rateLimits[extension]
	sm.mu.RUnlock()

	if !exists {
		// No rate limit set, allow
		return true
	}

	return limiter.Allow()
}

// SetRateLimit sets rate limit for an extension
func (sm *SecurityManager) SetRateLimit(extension string, requestsPerSecond int) {
	sm.mu.Lock()
	defer sm.mu.Unlock()

	sm.rateLimits[extension] = NewRateLimiter(requestsPerSecond)
}

// AuditLog logs a security event
func (sm *SecurityManager) AuditLog(ctx context.Context, extension, action, resource string, details map[string]interface{}) {
	sm.mu.Lock()
	defer sm.mu.Unlock()

	entry := AuditEntry{
		Timestamp: time.Now(),
		Extension: extension,
		Action:    action,
		Resource:  resource,
		Details:   details,
	}

	// Get user ID from context if available
	if userID := ctx.Value("user_id"); userID != nil {
		entry.UserID = userID.(string)
	}

	sm.addAuditEntry(entry)
}

// GetAuditLog returns audit log entries
func (sm *SecurityManager) GetAuditLog(extension string, limit int) []AuditEntry {
	sm.mu.RLock()
	defer sm.mu.RUnlock()

	var entries []AuditEntry
	count := 0

	// Return entries in reverse order (newest first)
	for i := len(sm.auditLog) - 1; i >= 0 && count < limit; i-- {
		if extension == "" || sm.auditLog[i].Extension == extension {
			entries = append(entries, sm.auditLog[i])
			count++
		}
	}

	return entries
}

// addAuditEntry adds an entry to the audit log
func (sm *SecurityManager) addAuditEntry(entry AuditEntry) {
	sm.auditLog = append(sm.auditLog, entry)

	// Keep only last 10000 entries
	if len(sm.auditLog) > 10000 {
		sm.auditLog = sm.auditLog[len(sm.auditLog)-10000:]
	}
}

// SecureExtensionHandler wraps an extension handler with security checks
func (sm *SecurityManager) SecureExtensionHandler(extension string, handler http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Check rate limit
		if !sm.CheckRateLimit(extension) {
			http.Error(w, "Rate limit exceeded", http.StatusTooManyRequests)
			sm.AuditLog(r.Context(), extension, "rate_limit_exceeded", r.URL.Path, nil)
			return
		}

		// Log access
		sm.AuditLog(r.Context(), extension, "request", r.URL.Path, map[string]interface{}{
			"method": r.Method,
			"ip":     r.RemoteAddr,
		})

		// Execute handler
		handler(w, r)
	}
}

// ValidateExtensionSafety performs safety checks on an extension
func (sm *SecurityManager) ValidateExtensionSafety(ext Extension) error {
	metadata := ext.Metadata()

	// Check for suspicious patterns
	if metadata.Name == "" {
		return fmt.Errorf("extension name is required")
	}

	if metadata.Version == "" {
		return fmt.Errorf("extension version is required")
	}

	// Check required permissions
	permissions := ext.RequiredPermissions()
	for _, perm := range permissions {
		if perm.Name == "" || perm.Resource == "" {
			return fmt.Errorf("invalid permission definition")
		}

		// Check for dangerous permissions
		if perm.Resource == "system" && contains(perm.Actions, "execute") {
			return fmt.Errorf("extension requests dangerous system execute permission")
		}
	}

	return nil
}

func contains(slice []string, item string) bool {
	for _, s := range slice {
		if s == item {
			return true
		}
	}
	return false
}

// ExtensionCapabilities defines what an extension can do
type ExtensionCapabilities struct {
	CanAccessDatabase   bool
	CanAccessStorage    bool
	CanAccessNetwork    bool
	CanAccessFileSystem bool
	CanExecuteCommands  bool
	CanModifySystem     bool
}

// GetExtensionCapabilities returns the capabilities for an extension
func (sm *SecurityManager) GetExtensionCapabilities(extension string) ExtensionCapabilities {
	// Default safe capabilities
	return ExtensionCapabilities{
		CanAccessDatabase:   true,
		CanAccessStorage:    true,
		CanAccessNetwork:    false,
		CanAccessFileSystem: false,
		CanExecuteCommands:  false,
		CanModifySystem:     false,
	}
}
