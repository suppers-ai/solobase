package utils

import (
	"errors"
	"net/http"
	"strings"

	"github.com/suppers-ai/solobase/constants"
)

// Storage ownership errors
var (
	ErrNotOwner       = errors.New("access denied: not owner")
	ErrAppIDMismatch  = errors.New("access denied: app ID mismatch")
	ErrObjectNotFound = errors.New("object not found")
)

// GetUserIDFromRequest extracts user ID from request context or JWT token
func GetUserIDFromRequest(r *http.Request) string {
	// First try typed context key
	if userID, ok := r.Context().Value(constants.ContextKeyUserID).(string); ok && userID != "" {
		return userID
	}

	// Fallback to string context key for backward compatibility
	if userID, ok := r.Context().Value("user_id").(string); ok && userID != "" {
		return userID
	}

	// Last resort: extract from JWT token
	return extractUserIDFromToken(r)
}

// extractUserIDFromToken extracts user ID from JWT token in Authorization header
func extractUserIDFromToken(r *http.Request) string {
	authHeader := r.Header.Get("Authorization")
	if authHeader == "" {
		return ""
	}

	tokenString := strings.TrimPrefix(authHeader, "Bearer ")
	if tokenString == authHeader {
		return ""
	}

	// Use build-tag specific token parsing
	userID, err := parseStorageToken(tokenString)
	if err != nil {
		return ""
	}

	return userID
}

// NormalizeBucket converts user-facing bucket names to internal bucket names
func NormalizeBucket(bucket string) string {
	if bucket == constants.UserFilesBucket || bucket == constants.InternalStorageBucket {
		return constants.InternalStorageBucket
	}
	return bucket
}

// IsInternalBucket checks if a bucket is the internal storage bucket
func IsInternalBucket(bucket string) bool {
	return bucket == constants.UserFilesBucket || bucket == constants.InternalStorageBucket
}

// RequireAuth is a helper that checks for user authentication and returns an error response if not authenticated
func RequireAuth(w http.ResponseWriter, r *http.Request) (string, bool) {
	userID := GetUserIDFromRequest(r)
	if userID == "" {
		JSONError(w, http.StatusUnauthorized, "Authentication required")
		return "", false
	}
	return userID, true
}

// StorageObjectInfo holds the basic info needed for ownership checks
type StorageObjectInfo struct {
	UserID string
	AppID  *string
}

// CheckStorageOwnership verifies that the user owns the storage object
// Returns nil if ownership is verified, otherwise returns an appropriate error
func CheckStorageOwnership(userID string, obj *StorageObjectInfo, expectedAppID string) error {
	if obj == nil {
		return ErrObjectNotFound
	}

	// Check if object belongs to user
	if obj.UserID != userID {
		return ErrNotOwner
	}

	// If an app ID is expected, verify it matches
	if expectedAppID != "" {
		if obj.AppID == nil || *obj.AppID != expectedAppID {
			return ErrAppIDMismatch
		}
	}

	return nil
}

// HandleOwnershipError writes the appropriate HTTP error response for ownership errors
func HandleOwnershipError(w http.ResponseWriter, err error) {
	switch {
	case errors.Is(err, ErrObjectNotFound):
		JSONError(w, http.StatusNotFound, "Object not found")
	case errors.Is(err, ErrNotOwner), errors.Is(err, ErrAppIDMismatch):
		JSONError(w, http.StatusForbidden, "Access denied")
	default:
		JSONError(w, http.StatusInternalServerError, "Internal server error")
	}
}
