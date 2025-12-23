//go:build wasm

// SHA-256 wrapper for WASM builds.
// Delegates to packages/sha256 which provides a pure-Go implementation.

package crypto

import (
	"github.com/suppers-ai/solobase/packages/sha256"
)

// SHA256Sum computes SHA-256 hash of data
func SHA256Sum(data []byte) [32]byte {
	return sha256.Sum256(data)
}

// HMACSHA256 computes HMAC-SHA256 of message with key
func HMACSHA256(key, message []byte) []byte {
	return sha256.HMAC(key, message)
}

// pureSHA256 is used internally by hasher_sha256.go
func pureSHA256(data []byte) [32]byte {
	return sha256.Sum256(data)
}
