//go:build !wasm

// Package crypto provides JWT signing using standard library crypto/hmac
package crypto

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/json"
	"strings"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

// CreateToken creates a signed JWT token using standard library HMAC-SHA256
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

	// Signature
	signingInput := headerB64 + "." + payloadB64
	h := hmac.New(sha256.New, []byte(secret))
	h.Write([]byte(signingInput))
	signature := h.Sum(nil)
	signatureB64 := base64URLEncode(signature)

	return signingInput + "." + signatureB64, nil
}

// VerifyToken verifies a JWT token and returns the claims
func VerifyToken(tokenString, secret string) (*JWTClaims, error) {
	parts := strings.Split(tokenString, ".")
	if len(parts) != 3 {
		return nil, ErrInvalidToken
	}

	// Verify signature
	signingInput := parts[0] + "." + parts[1]
	h := hmac.New(sha256.New, []byte(secret))
	h.Write([]byte(signingInput))
	expectedSig := h.Sum(nil)
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

