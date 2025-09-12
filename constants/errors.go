package constants

// Common error messages
const (
	// Client errors
	ErrInvalidRequest  = "Invalid request"
	ErrMissingRequired = "Missing required fields"
	ErrInvalidFormat   = "Invalid format"
	ErrNotFound        = "Resource not found"
	ErrAlreadyExists   = "Resource already exists"

	// Auth errors
	ErrUnauthorized       = "Unauthorized"
	ErrForbidden          = "Forbidden"
	ErrInvalidCredentials = "Invalid credentials"
	ErrInsufficientPerms  = "Access denied - Insufficient permissions"
	ErrAccountBanned      = "Access denied - Account banned"
	ErrSessionExpired     = "Session expired"
	ErrInvalidToken       = "Invalid or expired token"
	ErrMissingAuthHeader  = "Missing authorization header"
	ErrInvalidAuthFormat  = "Invalid authorization header format"

	// Server errors
	ErrInternalServer     = "Internal server error"
	ErrDatabaseError      = "Database error"
	ErrServiceUnavailable = "Service temporarily unavailable"

	// Validation errors
	ErrInvalidUserID = "Invalid user ID"
	ErrInvalidEmail  = "Invalid email address"
	ErrInvalidRole   = "Invalid role"

	// Storage errors
	ErrBucketRequired = "Bucket name is required"
	ErrPathRequired   = "Path is required"
	ErrFileNotFound   = "File not found"
	ErrUploadFailed   = "Failed to upload file"
	ErrDeleteFailed   = "Failed to delete file"

	// Database errors
	ErrTableRequired = "Table name is required"
	ErrInvalidQuery  = "Invalid query"
	ErrQueryFailed   = "Query execution failed"

	// Success messages
	MsgSuccess           = "Operation successful"
	MsgCreated           = "Created successfully"
	MsgUpdated           = "Updated successfully"
	MsgDeleted           = "Deleted successfully"
	MsgPasswordResetSent = "Password reset email sent"
)
