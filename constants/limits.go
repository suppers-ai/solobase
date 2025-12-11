package constants

// File and upload limits
const (
	MaxUploadSize        = 100 << 20 // 100MB
	MaxMultipartFormSize = 32 << 20  // 32MB for multipart form parsing
	DefaultMaxFileSize   = 10 << 20  // 10MB default for presigned uploads
	MaxFileNameLength    = 255
	MaxPathLength        = 4096

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

	// Storage quota defaults
	DefaultMaxStorageBytes   = 5 * 1024 * 1024 * 1024  // 5GB
	DefaultMaxBandwidthBytes = 10 * 1024 * 1024 * 1024 // 10GB
)

// Bucket names
const (
	InternalStorageBucket = "int_storage"
	UserFilesBucket       = "user-files"
)
