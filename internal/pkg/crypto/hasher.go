// Package crypto provides cryptographic utilities
package crypto

import "errors"

// ErrMismatchedPassword is returned when password doesn't match
var ErrMismatchedPassword = errors.New("crypto: hashedPassword is not the hash of the given password")

// PasswordHasher defines the interface for password hashing implementations
type PasswordHasher interface {
	// Hash hashes a plaintext password
	Hash(password string) (string, error)

	// Compare compares a password with a hash
	Compare(hashedPassword, password string) error

	// NeedsRehash checks if a hash needs to be rehashed with current settings
	NeedsRehash(hashedPassword string) bool

	// CanVerify checks if this hasher can verify the given hash format
	CanVerify(hashedPassword string) bool
}

// defaultHasher is the hasher used by package-level functions
// Set via build tags in hasher_standard.go and hasher_wasm.go
var defaultHasher PasswordHasher

// HashPassword hashes a plaintext password using the default hasher
func HashPassword(password string) (string, error) {
	return defaultHasher.Hash(password)
}

// HashPasswordWithCost hashes a password (cost parameter is ignored, kept for API compatibility)
func HashPasswordWithCost(password string, cost int) (string, error) {
	return defaultHasher.Hash(password)
}

// ComparePassword compares a password with a hash using the default hasher
func ComparePassword(hashedPassword, password string) error {
	return defaultHasher.Compare(hashedPassword, password)
}

// NeedsRehash checks if a hash needs to be rehashed
func NeedsRehash(hashedPassword string) bool {
	return defaultHasher.NeedsRehash(hashedPassword)
}
