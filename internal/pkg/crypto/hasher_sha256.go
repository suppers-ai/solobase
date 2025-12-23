//go:build wasm

package crypto

import (
	"crypto/rand"
	"crypto/subtle"
	"encoding/base64"
	"errors"
	"strconv"
	"strings"
)

// NOTE: This SHA-256 hasher is used for Spin/WASM builds because argon2id
// uses goroutines internally which are not supported in the WASM runtime.
// This is primarily intended for development and testing purposes.
// For production use with sensitive data, consider using the standard build
// with argon2id hashing.

const (
	saltLen          = 16
	sha256Iterations = 5000 // Reduced for WASM stability (higher counts cause hash divergence)
)

const sha256v2Prefix = "$sha256v2$"

// SHA256Hasher implements PasswordHasher using iterated SHA-256 (WASM-compatible)
type SHA256Hasher struct {
	iterations int
}

// NewSHA256Hasher creates a new SHA256Hasher
func NewSHA256Hasher() *SHA256Hasher {
	return &SHA256Hasher{iterations: sha256Iterations}
}

// Hash hashes a plaintext password using iterated SHA-256
func (h *SHA256Hasher) Hash(password string) (string, error) {
	salt := make([]byte, saltLen)
	if _, err := rand.Read(salt); err != nil {
		return "", err
	}

	hash := h.hashWithIterations(password, salt, h.iterations)

	// Format: $sha256v2$<iterations>$<base64-salt>$<base64-hash>
	result := sha256v2Prefix +
		strconv.Itoa(h.iterations) + "$" +
		base64.RawStdEncoding.EncodeToString(salt) + "$" +
		base64.RawStdEncoding.EncodeToString(hash)

	return result, nil
}

// hashWithIterations performs iterated SHA-256 hashing using pure-Go implementation
func (h *SHA256Hasher) hashWithIterations(password string, salt []byte, iterations int) []byte {
	data := append([]byte(password), salt...)

	// Use a fixed buffer to avoid slice allocation issues in WASM
	hashBuf := make([]byte, 32)

	hash := pureSHA256(data)
	copy(hashBuf, hash[:])

	for i := 1; i < iterations; i++ {
		hash = pureSHA256(hashBuf)
		copy(hashBuf, hash[:])
	}

	return hashBuf
}

// Compare compares a password with a hash
func (h *SHA256Hasher) Compare(hashedPassword, password string) error {
	// New SHA-256 v2 format
	if strings.HasPrefix(hashedPassword, sha256v2Prefix) {
		return h.compareSHA256v2(hashedPassword, password)
	}

	// Argon2 hashes from standard builds cannot be verified in WASM
	if strings.HasPrefix(hashedPassword, "$argon2id$") {
		return errors.New("crypto: argon2 hashes cannot be verified in WASM - user must reset password")
	}

	// Legacy bcrypt hashes ($2a$, $2b$, $2y$) cannot be verified in WASM
	if strings.HasPrefix(hashedPassword, "$2") {
		return errors.New("crypto: bcrypt hashes require migration - user must reset password")
	}

	// Legacy SHA-256 hashes from old WASM builds
	if strings.HasPrefix(hashedPassword, "$sha256$") {
		return h.compareSHA256Legacy(hashedPassword, password)
	}

	return ErrMismatchedPassword
}

// compareSHA256v2 verifies a password against a SHA-256 v2 hash
func (h *SHA256Hasher) compareSHA256v2(hashedPassword, password string) error {
	parts := strings.Split(hashedPassword, "$")
	if len(parts) != 5 || parts[1] != "sha256v2" {
		return ErrMismatchedPassword
	}

	iterations, err := strconv.Atoi(parts[2])
	if err != nil {
		return ErrMismatchedPassword
	}

	salt, err := base64.RawStdEncoding.DecodeString(parts[3])
	if err != nil {
		return ErrMismatchedPassword
	}

	expectedHash, err := base64.RawStdEncoding.DecodeString(parts[4])
	if err != nil {
		return ErrMismatchedPassword
	}

	actualHash := h.hashWithIterations(password, salt, iterations)

	if subtle.ConstantTimeCompare(expectedHash, actualHash) != 1 {
		return ErrMismatchedPassword
	}

	return nil
}

// compareSHA256Legacy handles old SHA-256 hashes from previous WASM builds
func (h *SHA256Hasher) compareSHA256Legacy(hashedPassword, password string) error {
	parts := strings.Split(hashedPassword, "$")
	if len(parts) != 4 || parts[1] != "sha256" {
		return ErrMismatchedPassword
	}

	salt, err := base64.StdEncoding.DecodeString(parts[2])
	if err != nil {
		return ErrMismatchedPassword
	}

	expectedHash, err := base64.StdEncoding.DecodeString(parts[3])
	if err != nil {
		return ErrMismatchedPassword
	}

	// Hash with legacy method (10000 iterations of SHA-256) using pure-Go implementation
	data := append([]byte(password), salt...)
	hash := pureSHA256(data)
	for i := 1; i < 10000; i++ {
		hash = pureSHA256(hash[:])
	}

	if subtle.ConstantTimeCompare(expectedHash, hash[:]) != 1 {
		return ErrMismatchedPassword
	}

	return nil
}

// NeedsRehash checks if a hash needs to be rehashed
func (h *SHA256Hasher) NeedsRehash(hashedPassword string) bool {
	// Legacy hashes should be rehashed
	if strings.HasPrefix(hashedPassword, "$2") ||
		strings.HasPrefix(hashedPassword, "$sha256$") ||
		strings.HasPrefix(hashedPassword, "$argon2id$") {
		return true
	}

	// Check if SHA-256 v2 with current iterations
	if strings.HasPrefix(hashedPassword, sha256v2Prefix) {
		parts := strings.Split(hashedPassword, "$")
		if len(parts) == 5 {
			iterations, err := strconv.Atoi(parts[2])
			if err == nil && iterations == h.iterations {
				return false
			}
		}
		return true
	}

	return true
}

// CanVerify checks if this hasher can verify the given hash format
func (h *SHA256Hasher) CanVerify(hashedPassword string) bool {
	return strings.HasPrefix(hashedPassword, sha256v2Prefix) ||
		strings.HasPrefix(hashedPassword, "$sha256$")
}

func init() {
	defaultHasher = NewSHA256Hasher()
}
