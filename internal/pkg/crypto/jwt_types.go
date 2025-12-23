// Package crypto provides JWT signing
package crypto

import (
	"encoding/base64"
	"strings"
)

// JWTClaims represents the claims in a JWT token
type JWTClaims struct {
	UserID string   `json:"user_id"`
	Email  string   `json:"email"`
	Roles  []string `json:"roles"`
	Exp    int64    `json:"exp"`
	Iat    int64    `json:"iat"`
}

// Custom errors
type jwtError string

func (e jwtError) Error() string { return string(e) }

const (
	ErrInvalidToken     jwtError = "invalid token"
	ErrInvalidSignature jwtError = "invalid signature"
	ErrTokenExpired     jwtError = "token expired"
)

// base64URLEncode encodes data using base64 URL-safe encoding without padding
func base64URLEncode(data []byte) string {
	return strings.TrimRight(base64.URLEncoding.EncodeToString(data), "=")
}

// base64URLDecode decodes base64 URL-safe encoded data
func base64URLDecode(s string) ([]byte, error) {
	// Add padding if needed
	switch len(s) % 4 {
	case 2:
		s += "=="
	case 3:
		s += "="
	}
	return base64.URLEncoding.DecodeString(s)
}
