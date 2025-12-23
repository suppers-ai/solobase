package uuid

import (
	"crypto/rand"
	"encoding/hex"
	"errors"
	"fmt"
	"strings"
)

// UUID represents a UUID as a 16-byte array
type UUID [16]byte

// Nil is the zero UUID
var Nil UUID

// ErrInvalidUUID is returned when parsing an invalid UUID string
var ErrInvalidUUID = errors.New("invalid UUID")

// New generates a random UUID v4
func New() UUID {
	var u UUID

	// Generate random bytes using crypto/rand
	_, err := rand.Read(u[:])
	if err != nil {
		// Fallback to zero UUID if random fails (shouldn't happen)
		return Nil
	}

	// Set version (4) and variant (RFC 4122)
	u[6] = (u[6] & 0x0f) | 0x40 // Version 4
	u[8] = (u[8] & 0x3f) | 0x80 // Variant RFC 4122

	return u
}

// NewString generates a random UUID v4 and returns it as a string
func NewString() string {
	return New().String()
}

// String returns the UUID in canonical form: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
func (u UUID) String() string {
	return fmt.Sprintf("%08x-%04x-%04x-%04x-%012x",
		u[0:4], u[4:6], u[6:8], u[8:10], u[10:16])
}

// Parse parses a UUID from its string representation
// Supported formats:
//   - xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx (canonical)
//   - xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx (no dashes)
func Parse(s string) (UUID, error) {
	var u UUID

	// Remove dashes if present
	s = strings.ReplaceAll(s, "-", "")

	if len(s) != 32 {
		return Nil, ErrInvalidUUID
	}

	b, err := hex.DecodeString(s)
	if err != nil {
		return Nil, ErrInvalidUUID
	}

	copy(u[:], b)
	return u, nil
}

// MustParse is like Parse but panics if the string cannot be parsed
func MustParse(s string) UUID {
	u, err := Parse(s)
	if err != nil {
		panic(err)
	}
	return u
}

// IsNil returns true if the UUID is the zero value
func (u UUID) IsNil() bool {
	return u == Nil
}

// Bytes returns the UUID as a byte slice
func (u UUID) Bytes() []byte {
	return u[:]
}

// FromBytes creates a UUID from a byte slice
func FromBytes(b []byte) (UUID, error) {
	if len(b) != 16 {
		return Nil, ErrInvalidUUID
	}
	var u UUID
	copy(u[:], b)
	return u, nil
}
