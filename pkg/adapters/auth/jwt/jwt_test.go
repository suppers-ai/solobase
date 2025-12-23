package jwt

import (
	"testing"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"

	"github.com/suppers-ai/solobase/pkg/interfaces"
)

func TestJWTSignAndVerify(t *testing.T) {
	signer := New("test-secret-key-at-least-32-chars", 24)

	claims := interfaces.JWTClaims{
		UserID: "user-123",
		Email:  "test@example.com",
		Roles:  []string{"admin", "user"},
		Extra:  map[string]string{"tenant": "acme"},
	}

	// Sign
	token, err := signer.Sign(claims)
	if err != nil {
		t.Fatalf("Sign failed: %v", err)
	}

	if token == "" {
		t.Fatal("Sign returned empty token")
	}

	// Verify
	verified, err := signer.Verify(token)
	if err != nil {
		t.Fatalf("Verify failed: %v", err)
	}

	if verified.UserID != claims.UserID {
		t.Errorf("UserID mismatch: got %s, want %s", verified.UserID, claims.UserID)
	}

	if verified.Email != claims.Email {
		t.Errorf("Email mismatch: got %s, want %s", verified.Email, claims.Email)
	}

	if len(verified.Roles) != len(claims.Roles) {
		t.Errorf("Roles mismatch: got %v, want %v", verified.Roles, claims.Roles)
	}

	if verified.Extra["tenant"] != claims.Extra["tenant"] {
		t.Errorf("Extra mismatch: got %v, want %v", verified.Extra, claims.Extra)
	}
}

func TestJWTExpiration(t *testing.T) {
	signer := New("test-secret-key-at-least-32-chars", 24)

	// Create claims with already-expired timestamp
	claims := interfaces.JWTClaims{
		UserID:    "user-123",
		IssuedAt:  apptime.NowTime().Add(-apptime.Hour * 48),
		ExpiresAt: apptime.NowTime().Add(-apptime.Hour * 24), // Already expired
	}

	token, err := signer.Sign(claims)
	if err != nil {
		t.Fatalf("Sign failed: %v", err)
	}

	// Verify should fail due to expiration
	_, err = signer.Verify(token)
	if err != interfaces.ErrTokenExpired {
		t.Errorf("Expected ErrTokenExpired, got: %v", err)
	}
}

func TestJWTInvalidToken(t *testing.T) {
	signer := New("test-secret-key-at-least-32-chars", 24)

	// Test invalid format
	_, err := signer.Verify("invalid")
	if err == nil {
		t.Error("Expected error for invalid token format")
	}

	// Test tampered token
	claims := interfaces.JWTClaims{UserID: "user-123"}
	token, _ := signer.Sign(claims)

	// Tamper with token
	tampered := token[:len(token)-5] + "xxxxx"
	_, err = signer.Verify(tampered)
	if err != interfaces.ErrTokenInvalid {
		t.Errorf("Expected ErrTokenInvalid for tampered token, got: %v", err)
	}
}

func TestJWTRefresh(t *testing.T) {
	signer := New("test-secret-key-at-least-32-chars", 24)

	claims := interfaces.JWTClaims{
		UserID: "user-123",
		Email:  "test@example.com",
	}

	token, err := signer.Sign(claims)
	if err != nil {
		t.Fatalf("Sign failed: %v", err)
	}

	// Refresh
	newToken, err := signer.Refresh(token)
	if err != nil {
		t.Fatalf("Refresh failed: %v", err)
	}

	if newToken == "" {
		t.Fatal("Refresh returned empty token")
	}

	// Verify refreshed token
	verified, err := signer.Verify(newToken)
	if err != nil {
		t.Fatalf("Verify refreshed token failed: %v", err)
	}

	if verified.UserID != claims.UserID {
		t.Errorf("UserID mismatch after refresh: got %s, want %s", verified.UserID, claims.UserID)
	}
}
