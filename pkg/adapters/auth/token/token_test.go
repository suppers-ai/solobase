package token

import (
	"regexp"
	"testing"
)

func TestGenerateToken(t *testing.T) {
	gen := New()

	// Test various lengths
	lengths := []int{16, 32, 64, 128}

	for _, length := range lengths {
		token, err := gen.GenerateToken(length)
		if err != nil {
			t.Errorf("GenerateToken(%d) failed: %v", length, err)
			continue
		}

		if len(token) != length {
			t.Errorf("GenerateToken(%d) returned token of length %d", length, len(token))
		}

		// Should be hex characters only
		if !regexp.MustCompile(`^[0-9a-f]+$`).MatchString(token) {
			t.Errorf("GenerateToken(%d) returned non-hex token: %s", length, token)
		}
	}
}

func TestGenerateTokenUniqueness(t *testing.T) {
	gen := New()

	tokens := make(map[string]bool)
	count := 100

	for i := 0; i < count; i++ {
		token, err := gen.GenerateToken(32)
		if err != nil {
			t.Fatalf("GenerateToken failed on iteration %d: %v", i, err)
		}

		if tokens[token] {
			t.Errorf("Duplicate token generated: %s", token)
		}
		tokens[token] = true
	}
}

func TestGenerateUUID(t *testing.T) {
	gen := New()

	uuid, err := gen.GenerateUUID()
	if err != nil {
		t.Fatalf("GenerateUUID failed: %v", err)
	}

	// UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
	// where y is 8, 9, a, or b
	uuidRegex := regexp.MustCompile(`^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`)

	if !uuidRegex.MatchString(uuid) {
		t.Errorf("GenerateUUID returned invalid format: %s", uuid)
	}
}

func TestGenerateUUIDUniqueness(t *testing.T) {
	gen := New()

	uuids := make(map[string]bool)
	count := 100

	for i := 0; i < count; i++ {
		uuid, err := gen.GenerateUUID()
		if err != nil {
			t.Fatalf("GenerateUUID failed on iteration %d: %v", i, err)
		}

		if uuids[uuid] {
			t.Errorf("Duplicate UUID generated: %s", uuid)
		}
		uuids[uuid] = true
	}
}

func TestGenerateTokenDefaultLength(t *testing.T) {
	gen := New()

	// Test with 0 or negative length (should default to 32)
	token, err := gen.GenerateToken(0)
	if err != nil {
		t.Fatalf("GenerateToken(0) failed: %v", err)
	}

	if len(token) != 32 {
		t.Errorf("GenerateToken(0) returned length %d, expected 32", len(token))
	}
}
