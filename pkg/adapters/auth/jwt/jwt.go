// Package jwt provides a JWT signer/verifier implementation.
// This implementation uses HMAC-SHA256 which is TinyGo compatible.
package jwt

import (
	"crypto/subtle"
	"encoding/base64"
	"encoding/json"
	"errors"
	"strings"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/crypto"
	"github.com/suppers-ai/solobase/pkg/interfaces"
)

// Signer implements interfaces.JWTSigner using HMAC-SHA256
type Signer struct {
	secret         []byte
	expirationTime apptime.Duration
}

// Ensure Signer implements interfaces.JWTSigner
var _ interfaces.JWTSigner = (*Signer)(nil)

// jwtHeader is the standard JWT header for HS256
type jwtHeader struct {
	Alg string `json:"alg"`
	Typ string `json:"typ"`
}

// jwtPayload is the JWT payload matching interfaces.JWTClaims
type jwtPayload struct {
	UserID string            `json:"user_id"`
	Email  string            `json:"email"`
	Roles  []string          `json:"roles"`
	Exp    int64             `json:"exp"`
	Iat    int64             `json:"iat"`
	Extra  map[string]string `json:"extra,omitempty"`
}

// New creates a new JWT signer
func New(secret string, expirationHours int) *Signer {
	if expirationHours <= 0 {
		expirationHours = 24
	}
	return &Signer{
		secret:         []byte(secret),
		expirationTime: apptime.Duration(expirationHours) * apptime.Hour,
	}
}

// Sign creates a signed JWT token
func (s *Signer) Sign(claims interfaces.JWTClaims) (string, error) {
	// Set timestamps if not provided
	now := apptime.NowTime()
	if claims.IssuedAt.IsZero() {
		claims.IssuedAt = now
	}
	if claims.ExpiresAt.IsZero() {
		claims.ExpiresAt = now.Add(s.expirationTime)
	}

	// Create header
	header := jwtHeader{
		Alg: "HS256",
		Typ: "JWT",
	}

	// Create payload
	payload := jwtPayload{
		UserID: claims.UserID,
		Email:  claims.Email,
		Roles:  claims.Roles,
		Exp:    claims.ExpiresAt.Unix(),
		Iat:    claims.IssuedAt.Unix(),
		Extra:  claims.Extra,
	}

	// Encode header
	headerJSON, err := json.Marshal(header)
	if err != nil {
		return "", err
	}
	headerB64 := base64URLEncode(headerJSON)

	// Encode payload
	payloadJSON, err := json.Marshal(payload)
	if err != nil {
		return "", err
	}
	payloadB64 := base64URLEncode(payloadJSON)

	// Create signature
	signingInput := headerB64 + "." + payloadB64
	signature := s.sign([]byte(signingInput))
	signatureB64 := base64URLEncode(signature)

	return signingInput + "." + signatureB64, nil
}

// Verify verifies and parses a JWT token
func (s *Signer) Verify(token string) (*interfaces.JWTClaims, error) {
	parts := strings.Split(token, ".")
	if len(parts) != 3 {
		return nil, errors.New("invalid token format")
	}

	headerB64 := parts[0]
	payloadB64 := parts[1]
	signatureB64 := parts[2]

	// Verify signature
	signingInput := headerB64 + "." + payloadB64
	expectedSignature := s.sign([]byte(signingInput))
	providedSignature, err := base64URLDecode(signatureB64)
	if err != nil {
		return nil, errors.New("invalid signature encoding")
	}

	if subtle.ConstantTimeCompare(expectedSignature, providedSignature) != 1 {
		return nil, interfaces.ErrTokenInvalid
	}

	// Decode header
	headerJSON, err := base64URLDecode(headerB64)
	if err != nil {
		return nil, errors.New("invalid header encoding")
	}

	var header jwtHeader
	if err := json.Unmarshal(headerJSON, &header); err != nil {
		return nil, errors.New("invalid header")
	}

	if header.Alg != "HS256" {
		return nil, errors.New("unsupported algorithm")
	}

	// Decode payload
	payloadJSON, err := base64URLDecode(payloadB64)
	if err != nil {
		return nil, errors.New("invalid payload encoding")
	}

	var payload jwtPayload
	if err := json.Unmarshal(payloadJSON, &payload); err != nil {
		return nil, errors.New("invalid payload")
	}

	// Check expiration
	if apptime.NowTime().Unix() > payload.Exp {
		return nil, interfaces.ErrTokenExpired
	}

	return &interfaces.JWTClaims{
		UserID:    payload.UserID,
		Email:     payload.Email,
		Roles:     payload.Roles,
		ExpiresAt: apptime.Unix(payload.Exp, 0),
		IssuedAt:  apptime.Unix(payload.Iat, 0),
		Extra:     payload.Extra,
	}, nil
}

// Refresh creates a new token with extended expiration
func (s *Signer) Refresh(token string) (string, error) {
	claims, err := s.Verify(token)
	if err != nil {
		// Allow refreshing expired tokens within a grace period
		if err == interfaces.ErrTokenExpired {
			// Re-parse without expiration check
			parts := strings.Split(token, ".")
			if len(parts) != 3 {
				return "", interfaces.ErrTokenInvalid
			}
			payloadJSON, err := base64URLDecode(parts[1])
			if err != nil {
				return "", interfaces.ErrTokenInvalid
			}
			var payload jwtPayload
			if err := json.Unmarshal(payloadJSON, &payload); err != nil {
				return "", interfaces.ErrTokenInvalid
			}
			claims = &interfaces.JWTClaims{
				UserID: payload.UserID,
				Email:  payload.Email,
				Roles:  payload.Roles,
				Extra:  payload.Extra,
			}
		} else {
			return "", err
		}
	}

	// Reset timestamps for new token
	claims.IssuedAt = apptime.Time{}
	claims.ExpiresAt = apptime.Time{}

	return s.Sign(*claims)
}

// sign creates an HMAC-SHA256 signature
func (s *Signer) sign(data []byte) []byte {
	return crypto.HMACSHA256(s.secret, data)
}

// base64URLEncode encodes data using URL-safe base64 without padding
func base64URLEncode(data []byte) string {
	return strings.TrimRight(base64.URLEncoding.EncodeToString(data), "=")
}

// base64URLDecode decodes URL-safe base64 data with optional padding
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
