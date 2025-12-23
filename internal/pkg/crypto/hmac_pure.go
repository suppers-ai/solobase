//go:build wasm

// HMAC-SHA256 wrapper for WASM builds.
// Delegates to packages/sha256 which provides a pure-Go implementation.

package crypto

import (
	"github.com/suppers-ai/solobase/packages/sha256"
)

// pureHMACSHA256 computes HMAC-SHA256 without using crypto/hmac package
func pureHMACSHA256(key, message []byte) []byte {
	return sha256.HMAC(key, message)
}
