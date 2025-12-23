package auth

import (
	"encoding/hex"

	"github.com/suppers-ai/solobase/internal/pkg/crypto"
)

// HashToken creates a SHA-256 hash of a token string
func HashToken(token string) string {
	hash := crypto.SHA256Sum([]byte(token))
	return hex.EncodeToString(hash[:])
}
