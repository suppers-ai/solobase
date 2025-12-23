package constants

// Context key type for type safety
type ContextKey string

// Context keys for request context
const (
	ContextKeyUserID    ContextKey = "user_id"
	ContextKeyUserEmail ContextKey = "user_email"
	ContextKeyUserRoles ContextKey = "user_roles"
)
