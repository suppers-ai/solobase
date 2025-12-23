//go:build !wasm

package crypto

import (
	"crypto/rand"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"errors"
	"strconv"
	"strings"

	"github.com/suppers-ai/solobase/internal/env"
	"golang.org/x/crypto/argon2"
)

// Argon2id parameters
const (
	argonTime       = 1
	argonThreads    = 1
	argonKeyLen     = 32
	saltLen         = 16
	defaultMemoryMB = 64
)

const argon2Prefix = "$argon2id$"

// Argon2Hasher implements PasswordHasher using argon2id
type Argon2Hasher struct {
	memoryKB uint32
}

// NewArgon2Hasher creates a new Argon2Hasher with memory from environment or default
func NewArgon2Hasher() *Argon2Hasher {
	memory := uint32(defaultMemoryMB * 1024)
	if memStr := env.GetEnv("ARGON2_MEMORY_MB"); memStr != "" {
		if memMB, err := strconv.Atoi(memStr); err == nil && memMB > 0 {
			memory = uint32(memMB) * 1024
		}
	}
	return &Argon2Hasher{memoryKB: memory}
}

// Hash hashes a plaintext password using argon2id
func (h *Argon2Hasher) Hash(password string) (string, error) {
	salt := make([]byte, saltLen)
	if _, err := rand.Read(salt); err != nil {
		return "", err
	}

	hash := argon2.IDKey([]byte(password), salt, argonTime, h.memoryKB, argonThreads, argonKeyLen)

	// Format: $argon2id$v=19$m=<memory>,t=<time>,p=<threads>$<base64-salt>$<base64-hash>
	result := argon2Prefix + "v=19$" +
		"m=" + strconv.FormatUint(uint64(h.memoryKB), 10) +
		",t=" + strconv.Itoa(argonTime) +
		",p=" + strconv.Itoa(argonThreads) + "$" +
		base64.RawStdEncoding.EncodeToString(salt) + "$" +
		base64.RawStdEncoding.EncodeToString(hash)

	return result, nil
}

// Compare compares a password with a hash
func (h *Argon2Hasher) Compare(hashedPassword, password string) error {
	if strings.HasPrefix(hashedPassword, argon2Prefix) {
		return h.compareArgon2(hashedPassword, password)
	}

	// Legacy bcrypt hashes ($2a$, $2b$, $2y$) cannot be verified
	if strings.HasPrefix(hashedPassword, "$2") {
		return errors.New("crypto: bcrypt hashes require migration - user must reset password")
	}

	// Legacy SHA-256 hashes from old WASM builds
	if strings.HasPrefix(hashedPassword, "$sha256$") {
		return compareSHA256Legacy(hashedPassword, password)
	}

	// SHA-256 v2 hashes from Spin builds
	if strings.HasPrefix(hashedPassword, "$sha256v2$") {
		return compareSHA256v2(hashedPassword, password)
	}

	return ErrMismatchedPassword
}

// compareArgon2 verifies a password against an argon2id hash
func (h *Argon2Hasher) compareArgon2(hashedPassword, password string) error {
	// Parse: $argon2id$v=19$m=<memory>,t=<time>,p=<threads>$<base64-salt>$<base64-hash>
	parts := strings.Split(hashedPassword, "$")
	if len(parts) != 6 || parts[1] != "argon2id" {
		return ErrMismatchedPassword
	}

	// Parse parameters from parts[3]: m=65536,t=1,p=4
	params := strings.Split(parts[3], ",")
	if len(params) != 3 {
		return ErrMismatchedPassword
	}

	var memory uint32
	var time uint32
	var threads uint8

	for _, p := range params {
		kv := strings.Split(p, "=")
		if len(kv) != 2 {
			return ErrMismatchedPassword
		}
		val, err := strconv.ParseUint(kv[1], 10, 32)
		if err != nil {
			return ErrMismatchedPassword
		}
		switch kv[0] {
		case "m":
			memory = uint32(val)
		case "t":
			time = uint32(val)
		case "p":
			threads = uint8(val)
		}
	}

	salt, err := base64.RawStdEncoding.DecodeString(parts[4])
	if err != nil {
		return ErrMismatchedPassword
	}

	expectedHash, err := base64.RawStdEncoding.DecodeString(parts[5])
	if err != nil {
		return ErrMismatchedPassword
	}

	// Hash the provided password with the same parameters
	actualHash := argon2.IDKey([]byte(password), salt, time, memory, threads, uint32(len(expectedHash)))

	if subtle.ConstantTimeCompare(expectedHash, actualHash) != 1 {
		return ErrMismatchedPassword
	}

	return nil
}

// NeedsRehash checks if a hash needs to be rehashed
func (h *Argon2Hasher) NeedsRehash(hashedPassword string) bool {
	// bcrypt and SHA-256 hashes should be rehashed to argon2id
	if strings.HasPrefix(hashedPassword, "$2") ||
		strings.HasPrefix(hashedPassword, "$sha256$") ||
		strings.HasPrefix(hashedPassword, "$sha256v2$") {
		return true
	}

	// Check if it's a valid argon2id hash with current memory setting
	if strings.HasPrefix(hashedPassword, argon2Prefix) {
		parts := strings.Split(hashedPassword, "$")
		if len(parts) == 6 {
			params := strings.Split(parts[3], ",")
			for _, p := range params {
				if strings.HasPrefix(p, "m=") {
					val, err := strconv.ParseUint(strings.TrimPrefix(p, "m="), 10, 32)
					if err == nil && uint32(val) == h.memoryKB {
						return false
					}
				}
			}
		}
		return true
	}

	return true
}

// CanVerify checks if this hasher can verify the given hash format
func (h *Argon2Hasher) CanVerify(hashedPassword string) bool {
	return strings.HasPrefix(hashedPassword, argon2Prefix) ||
		strings.HasPrefix(hashedPassword, "$sha256$") ||
		strings.HasPrefix(hashedPassword, "$sha256v2$")
}

// compareSHA256Legacy handles old SHA-256 hashes from previous WASM builds
func compareSHA256Legacy(hashedPassword, password string) error {
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

	// Hash with legacy method (10000 iterations of SHA-256)
	data := append([]byte(password), salt...)
	hash := sha256.Sum256(data)
	for i := 1; i < 10000; i++ {
		hash = sha256.Sum256(hash[:])
	}

	if subtle.ConstantTimeCompare(expectedHash, hash[:]) != 1 {
		return ErrMismatchedPassword
	}

	return nil
}

// compareSHA256v2 handles SHA-256 v2 hashes from Spin builds
func compareSHA256v2(hashedPassword, password string) error {
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

	// Hash with SHA-256 iterations
	data := append([]byte(password), salt...)
	hash := sha256.Sum256(data)
	for i := 1; i < iterations; i++ {
		hash = sha256.Sum256(hash[:])
	}

	if subtle.ConstantTimeCompare(expectedHash, hash[:]) != 1 {
		return ErrMismatchedPassword
	}

	return nil
}

func init() {
	defaultHasher = NewArgon2Hasher()
}
