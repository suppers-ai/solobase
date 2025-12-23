package constants

import "github.com/suppers-ai/solobase/internal/pkg/apptime"

// Authentication and Session Constants
const (
	// Token expiration durations
	AccessTokenDuration        = 15 * apptime.Minute   // Short-lived JWT access token
	RefreshTokenDuration       = 7 * 24 * apptime.Hour // 7 days refresh token
	SessionExpirationDuration  = apptime.Hour
	PasswordResetTokenDuration = apptime.Hour
	EmailVerificationDuration  = 24 * apptime.Hour

	// Cookie settings
	AuthCookieName   = "auth_token"
	AuthCookieMaxAge = 900 // 15 minutes in seconds (matches AccessTokenDuration)
)

// Pagination Constants
const (
	DefaultPageSize = 100
	MaxPageSize     = 1000
	MinPageSize     = 1
	DefaultPage     = 1
)

// Storage Constants
const (
	InternalStorageBucket = "int_storage"
	UserFilesBucket       = "user-files"
	DefaultFolderName     = "My Files"

	// Upload limits
	MaxFileSize       = 100 * 1024 * 1024 // 100MB
	MaxUploadSize     = 500 * 1024 * 1024 // 500MB total per request
	MaxFilesPerUpload = 10
)

// Database Constants
const (
	// Query limits
	MaxQueryResults = 10000
	QueryTimeout    = 30 * apptime.Second

	// Connection pool
	MaxOpenConnections = 25
	MaxIdleConnections = 5
	ConnectionLifetime = 5 * apptime.Minute
)

// API Rate Limiting
const (
	RateLimitRequestsPerMinute = 60
	RateLimitBurst            = 10
)

// Password Requirements
const (
	MinPasswordLength = 8
	MaxPasswordLength = 128
)

// Context Keys
const (
	ContextKeyUserID    = "user_id"
	ContextKeyUserEmail = "user_email"
	ContextKeyUserRoles = "user_roles"
	ContextKeyRequestID = "request_id"
)

// Error Messages
const (
	ErrMissingAuthHeader  = "Missing authorization header"
	ErrInvalidAuthFormat  = "Invalid authorization format"
	ErrInvalidToken       = "Invalid or expired token"
	ErrUnauthorized       = "Authentication required"
	ErrForbidden          = "Access denied"
	ErrInternalServer     = "Internal server error"
	ErrBadRequest         = "Bad request"
	ErrNotFound           = "Resource not found"
	ErrRateLimitExceeded  = "Rate limit exceeded"
)