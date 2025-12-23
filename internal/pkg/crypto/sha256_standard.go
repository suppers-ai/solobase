//go:build !wasm

package crypto

import (
	"crypto/hmac"
	"crypto/sha256"
)

// SHA256Sum computes SHA-256 hash of data
func SHA256Sum(data []byte) [32]byte {
	return sha256.Sum256(data)
}

// HMACSHA256 computes HMAC-SHA256 of message with key
func HMACSHA256(key, message []byte) []byte {
	h := hmac.New(sha256.New, key)
	h.Write(message)
	return h.Sum(nil)
}
