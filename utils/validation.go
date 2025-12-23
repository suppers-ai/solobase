package utils

import (
	"fmt"
	"strings"

	"github.com/suppers-ai/solobase/constants"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
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
	if !isValidEmailFormat(email) {
		return fmt.Errorf("%s: %s", constants.ErrInvalidEmail, email)
	}
	return nil
}

// isValidEmailFormat validates email format without regexp
func isValidEmailFormat(email string) bool {
	// Must contain exactly one @
	atIndex := strings.Index(email, "@")
	if atIndex == -1 || atIndex == 0 || atIndex == len(email)-1 {
		return false
	}
	// Check for another @
	if strings.Index(email[atIndex+1:], "@") != -1 {
		return false
	}
	local := email[:atIndex]
	domain := email[atIndex+1:]

	if len(local) == 0 || len(local) > 64 {
		return false
	}

	// Domain must contain at least one dot
	dotIndex := strings.LastIndex(domain, ".")
	if dotIndex == -1 || dotIndex == 0 || dotIndex == len(domain)-1 {
		return false
	}

	// TLD must be at least 2 characters
	tld := domain[dotIndex+1:]
	if len(tld) < 2 {
		return false
	}

	// Check for valid characters in local part
	for _, c := range local {
		if !((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
			(c >= '0' && c <= '9') || c == '.' || c == '_' ||
			c == '%' || c == '+' || c == '-') {
			return false
		}
	}

	// Check for valid characters in domain
	for _, c := range domain {
		if !((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
			(c >= '0' && c <= '9') || c == '.' || c == '-') {
			return false
		}
	}

	return true
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
	if !isValidBucketName(name) {
		return fmt.Errorf("invalid bucket name: must start with lowercase letter and contain only lowercase letters, numbers, and hyphens")
	}
	return nil
}

// isValidBucketName validates bucket name format without regexp
// Pattern: ^[a-z][a-z0-9-]{2,62}$
func isValidBucketName(name string) bool {
	if len(name) < 3 || len(name) > 63 {
		return false
	}
	// First character must be lowercase letter
	if name[0] < 'a' || name[0] > 'z' {
		return false
	}
	// Rest can be lowercase letters, digits, or hyphens
	for i := 1; i < len(name); i++ {
		c := name[i]
		if !((c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '-') {
			return false
		}
	}
	return true
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
