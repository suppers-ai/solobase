package constants

// Session constants
const (
	// Session name
	SessionName = "solobase-session"

	// Session keys
	SessionKeyUserID   = "user_id"
	SessionKeyEmail    = "user_email"
	SessionKeyRole     = "user_role"
	SessionKeyLoggedIn = "logged_in_at"
	SessionKeyPID      = "pid"
)

// Context key type for type safety
type ContextKey string

// Context keys for request context
const (
	ContextKeyUserID    ContextKey = "user_id"
	ContextKeyUserEmail ContextKey = "user_email"
	ContextKeyUserRole  ContextKey = "user_role"
)
