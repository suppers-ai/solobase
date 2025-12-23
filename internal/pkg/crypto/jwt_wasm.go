//go:build wasm

// Package crypto provides JWT signing using pure-Go HMAC-SHA256 for WASM builds
// This avoids the crypto/hmac and crypto/sha256 packages which use crypto/internal/fips140
// that doesn't work properly in TinyGo WASM.
package crypto

import (
	"encoding/json"
	"strings"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

// CreateToken creates a signed JWT token using pure-Go HMAC-SHA256
func CreateToken(claims JWTClaims, secret string) (string, error) {
	// Header
	header := map[string]string{
		"alg": "HS256",
		"typ": "JWT",
	}
	headerJSON, err := json.Marshal(header)
	if err != nil {
		return "", err
	}
	headerB64 := base64URLEncode(headerJSON)

	// Payload
	payloadJSON, err := json.Marshal(claims)
	if err != nil {
		return "", err
	}
	payloadB64 := base64URLEncode(payloadJSON)

	// Signature using pure-Go HMAC-SHA256
	signingInput := headerB64 + "." + payloadB64
	signature := pureHMACSHA256([]byte(secret), []byte(signingInput))
	signatureB64 := base64URLEncode(signature)

	return signingInput + "." + signatureB64, nil
}

// VerifyToken verifies a JWT token and returns the claims
func VerifyToken(tokenString, secret string) (*JWTClaims, error) {
	parts := strings.Split(tokenString, ".")
	if len(parts) != 3 {
		return nil, ErrInvalidToken
	}

	// Verify signature using pure-Go HMAC-SHA256
	signingInput := parts[0] + "." + parts[1]
	expectedSig := pureHMACSHA256([]byte(secret), []byte(signingInput))
	expectedSigB64 := base64URLEncode(expectedSig)

	if parts[2] != expectedSigB64 {
		return nil, ErrInvalidSignature
	}

	// Decode payload
	payloadJSON, err := base64URLDecode(parts[1])
	if err != nil {
		return nil, err
	}

	var claims JWTClaims
	if err := json.Unmarshal(payloadJSON, &claims); err != nil {
		return nil, err
	}

	// Check expiration
	if claims.Exp < apptime.NowTime().Unix() {
		return nil, ErrTokenExpired
	}

	return &claims, nil
}
