package utils

import (
	"fmt"
	"regexp"
	"strings"

	"github.com/google/uuid"
	"github.com/suppers-ai/solobase/constants"
)

var (
	emailRegex = regexp.MustCompile(`^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$`)
	// Bucket name must start with letter, contain only lowercase letters, numbers, and hyphens
	bucketNameRegex = regexp.MustCompile(`^[a-z][a-z0-9-]{2,62}$`)
)

// ValidateUUID validates a UUID string
func ValidateUUID(id string) error {
	if _, err := uuid.Parse(id); err != nil {
		return fmt.Errorf("invalid UUID: %w", err)
	}
	return nil
}

// ValidateUserID validates a user ID (UUID format)
func ValidateUserID(userID string) error {
	if err := ValidateUUID(userID); err != nil {
		return fmt.Errorf("%s: %w", constants.ErrInvalidUserID, err)
	}
	return nil
}

// ValidateEmail validates an email address
func ValidateEmail(email string) error {
	email = strings.TrimSpace(strings.ToLower(email))
	if !emailRegex.MatchString(email) {
		return fmt.Errorf("%s: %s", constants.ErrInvalidEmail, email)
	}
	return nil
}

// ValidateRole validates a user role
func ValidateRole(role string) error {
	r := constants.UserRole(role)
	if !r.IsValid() {
		return fmt.Errorf("%s: %s", constants.ErrInvalidRole, role)
	}
	return nil
}

// ValidateBucketName validates a storage bucket name
func ValidateBucketName(name string) error {
	if name == "" {
		return fmt.Errorf(constants.ErrBucketRequired)
	}
	if !bucketNameRegex.MatchString(name) {
		return fmt.Errorf("invalid bucket name: must start with lowercase letter and contain only lowercase letters, numbers, and hyphens")
	}
	return nil
}

// ValidatePath validates a file path
func ValidatePath(path string) error {
	if path == "" {
		return fmt.Errorf(constants.ErrPathRequired)
	}
	if len(path) > constants.MaxPathLength {
		return fmt.Errorf("path too long: maximum %d characters", constants.MaxPathLength)
	}
	// Check for path traversal attempts
	if strings.Contains(path, "../") || strings.Contains(path, "..\\") {
		return fmt.Errorf("invalid path: path traversal not allowed")
	}
	return nil
}

// ValidatePagination validates pagination parameters
func ValidatePagination(page, pageSize int) error {
	if page < 1 {
		return fmt.Errorf("invalid page number: must be >= 1")
	}
	if pageSize < constants.MinPageSize || pageSize > constants.MaxPageSize {
		return fmt.Errorf("invalid page size: must be between %d and %d", constants.MinPageSize, constants.MaxPageSize)
	}
	return nil
}

// SanitizeFileName sanitizes a file name for safe storage
func SanitizeFileName(name string) string {
	// Remove any directory separators
	name = strings.ReplaceAll(name, "/", "-")
	name = strings.ReplaceAll(name, "\\", "-")

	// Limit length
	if len(name) > constants.MaxFileNameLength {
		name = name[:constants.MaxFileNameLength]
	}

	return name
}
