package constants

// File and upload limits
const (
	MaxUploadSize     = 100 << 20 // 100MB
	MaxFileNameLength = 255
	MaxPathLength     = 4096

	// Signed URL expiry limits (in seconds)
	MinSignedURLExpiry = 60       // 1 minute
	MaxSignedURLExpiry = 31536000 // 1 year
	DefaultURLExpiry   = 3600     // 1 hour

	// Timeout constants (in seconds)
	DefaultTimeout  = 30
	LongTimeout     = 120
	DatabaseTimeout = 10

	// Rate limiting
	MaxRequestsPerMin  = 60
	MaxRequestsPerHour = 1000
)
