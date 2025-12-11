package iam

import (
	"context"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/suppers-ai/solobase/constants"
)

// Middleware provides IAM middleware functionality
type Middleware struct {
	service *Service
}

// NewMiddleware creates a new IAM middleware
func NewMiddleware(service *Service) *Middleware {
	return &Middleware{
		service: service,
	}
}

// Authorize creates a middleware that checks permissions using Casbin
func (m *Middleware) Authorize() func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Get user ID from context (set by auth middleware)
			userID, ok := r.Context().Value(constants.ContextKeyUserID).(string)
			if !ok || userID == "" {
				http.Error(w, "Unauthorized", http.StatusUnauthorized)
				return
			}

			// Build context data for policy evaluation
			contextData := m.buildContextData(r)
			
			// Check permission
			allowed, err := m.service.CheckPermissionWithContext(
				r.Context(),
				userID,
				r.URL.Path,
				r.Method,
				contextData,
			)
			
			if err != nil {
				http.Error(w, "Internal server error", http.StatusInternalServerError)
				return
			}
			
			if !allowed {
				http.Error(w, "Access denied", http.StatusForbidden)
				return
			}

			// Add user roles to context for downstream handlers
			roles, _ := m.service.GetUserRoles(r.Context(), userID)
			ctx := context.WithValue(r.Context(), constants.ContextKeyUserRoles, roles)
			
			// Add effective metadata to context
			metadata, _ := m.service.GetUserEffectiveMetadata(r.Context(), userID)
			ctx = context.WithValue(ctx, "user_metadata", metadata)
			
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

// RequireRole creates a middleware that requires specific roles
func (m *Middleware) RequireRole(roles ...string) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			userID, ok := r.Context().Value(constants.ContextKeyUserID).(string)
			if !ok || userID == "" {
				http.Error(w, "Unauthorized", http.StatusUnauthorized)
				return
			}

			userRoles, err := m.service.GetUserRoles(r.Context(), userID)
			if err != nil {
				http.Error(w, "Internal server error", http.StatusInternalServerError)
				return
			}

			// Check if user has any of the required roles
			hasRole := false
			for _, requiredRole := range roles {
				for _, userRole := range userRoles {
					if userRole == requiredRole {
						hasRole = true
						break
					}
				}
				if hasRole {
					break
				}
			}

			if !hasRole {
				http.Error(w, "Access denied - insufficient role", http.StatusForbidden)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// EnforceQuota creates a middleware that enforces quotas based on IAM policies
func (m *Middleware) EnforceQuota() func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			userID, ok := r.Context().Value(constants.ContextKeyUserID).(string)
			if !ok || userID == "" {
				next.ServeHTTP(w, r)
				return
			}

			// Get user's effective metadata
			metadata, err := m.service.GetUserEffectiveMetadata(r.Context(), userID)
			if err != nil {
				next.ServeHTTP(w, r)
				return
			}

			// Upload size limits are now handled by CloudStorage extension
			
			// Add metadata to context for downstream handlers (access control only)
			ctx := context.WithValue(r.Context(), "user_quota", metadata)
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

// RateLimit creates a middleware that enforces rate limits
// Note: Rate limiting configuration should be handled by a dedicated rate limiting service
// This is a placeholder that can be extended with proper rate limiting logic
func (m *Middleware) RateLimit() func(http.Handler) http.Handler {
	// In-memory rate limit tracking (should use Redis in production)
	userRequests := make(map[string]*rateLimitData)
	
	// Default rate limit (can be made configurable via environment or config)
	const defaultMaxRequestsPerMin = 60
	
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			userID, ok := r.Context().Value(constants.ContextKeyUserID).(string)
			if !ok || userID == "" {
				next.ServeHTTP(w, r)
				return
			}

			// Use default rate limit for all users
			// In production, this could be role-based or user-specific from a rate limiting service
			maxRequests := defaultMaxRequestsPerMin

			// Check rate limit
			if !checkRateLimit(userRequests, userID, maxRequests) {
				w.Header().Set("X-RateLimit-Limit", strconv.Itoa(maxRequests))
				w.Header().Set("X-RateLimit-Remaining", "0")
				w.Header().Set("Retry-After", "60")
				http.Error(w, "Rate limit exceeded", http.StatusTooManyRequests)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// buildContextData builds context data for policy evaluation
func (m *Middleware) buildContextData(r *http.Request) map[string]interface{} {
	ctx := make(map[string]interface{})
	
	// Add request metadata
	ctx["ip"] = r.RemoteAddr
	if xff := r.Header.Get("X-Forwarded-For"); xff != "" {
		ctx["ip"] = xff
	}
	ctx["method"] = r.Method
	ctx["path"] = r.URL.Path
	ctx["user_agent"] = r.UserAgent()
	
	// Add file size for uploads
	if r.Method == "POST" && strings.Contains(r.URL.Path, "upload") {
		ctx["size"] = r.ContentLength
	}
	
	// Add query parameters
	if r.URL.Query() != nil {
		ctx["query"] = r.URL.Query()
	}
	
	return ctx
}

// rateLimitData tracks rate limit information for a user
type rateLimitData struct {
	requests  []int64
	lastReset int64
}

// checkRateLimit checks if a user has exceeded their rate limit
func checkRateLimit(userRequests map[string]*rateLimitData, userID string, limit int) bool {
	now := getCurrentMinute()
	
	data, exists := userRequests[userID]
	if !exists {
		userRequests[userID] = &rateLimitData{
			requests:  []int64{now},
			lastReset: now,
		}
		return true
	}
	
	// Reset if we're in a new minute
	if now > data.lastReset {
		data.requests = []int64{now}
		data.lastReset = now
		return true
	}
	
	// Count requests in current minute
	count := 0
	for _, reqTime := range data.requests {
		if reqTime == now {
			count++
		}
	}
	
	if count >= limit {
		return false
	}
	
	data.requests = append(data.requests, now)
	return true
}

// getCurrentMinute returns the current minute as a Unix timestamp
func getCurrentMinute() int64 {
	return (timeNow().Unix() / 60) * 60
}

// timeNow is a variable to allow mocking in tests
var timeNow = func() time.Time {
	return time.Now()
}

