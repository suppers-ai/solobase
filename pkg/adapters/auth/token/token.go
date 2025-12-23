// Package token provides token generation utilities.
// This implementation is TinyGo compatible.
package token

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"

	"github.com/suppers-ai/solobase/pkg/interfaces"
)

// Generator implements interfaces.TokenGenerator
type Generator struct{}

// Ensure Generator implements interfaces.TokenGenerator
var _ interfaces.TokenGenerator = (*Generator)(nil)

// New creates a new token generator
func New() *Generator {
	return &Generator{}
}

// GenerateToken generates a cryptographically secure random token
func (g *Generator) GenerateToken(length int) (string, error) {
	if length <= 0 {
		length = 32
	}

	// Generate random bytes (we need length/2 bytes for hex encoding)
	byteLength := (length + 1) / 2
	bytes := make([]byte, byteLength)

	if _, err := rand.Read(bytes); err != nil {
		return "", fmt.Errorf("failed to generate random bytes: %w", err)
	}

	// Encode to hex and trim to exact length
	token := hex.EncodeToString(bytes)
	if len(token) > length {
		token = token[:length]
	}

	return token, nil
}

// GenerateUUID generates a UUID v4
func (g *Generator) GenerateUUID() (string, error) {
	uuid := make([]byte, 16)
	if _, err := rand.Read(uuid); err != nil {
		return "", fmt.Errorf("failed to generate UUID: %w", err)
	}

	// Set version 4 (random)
	uuid[6] = (uuid[6] & 0x0f) | 0x40
	// Set variant (RFC 4122)
	uuid[8] = (uuid[8] & 0x3f) | 0x80

	// Format as UUID string
	return fmt.Sprintf("%08x-%04x-%04x-%04x-%012x",
		uuid[0:4],
		uuid[4:6],
		uuid[6:8],
		uuid[8:10],
		uuid[10:16],
	), nil
}
