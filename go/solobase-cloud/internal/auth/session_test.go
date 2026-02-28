package auth

import (
	"testing"
	"time"
)

func TestSessionStore_CreateAndGet(t *testing.T) {
	store := NewSessionStore(1 * time.Hour)

	session, err := store.Create("user-1")
	if err != nil {
		t.Fatalf("Create: %v", err)
	}
	if session.Token == "" {
		t.Fatal("expected non-empty token")
	}
	if session.UserID != "user-1" {
		t.Errorf("UserID = %q, want %q", session.UserID, "user-1")
	}

	got, ok := store.Get(session.Token)
	if !ok {
		t.Fatal("Get: session not found")
	}
	if got.UserID != "user-1" {
		t.Errorf("UserID = %q, want %q", got.UserID, "user-1")
	}
}

func TestSessionStore_Expiry(t *testing.T) {
	store := NewSessionStore(1 * time.Millisecond)

	session, err := store.Create("user-1")
	if err != nil {
		t.Fatalf("Create: %v", err)
	}

	time.Sleep(5 * time.Millisecond)

	_, ok := store.Get(session.Token)
	if ok {
		t.Error("expected session to be expired")
	}
}

func TestSessionStore_Delete(t *testing.T) {
	store := NewSessionStore(1 * time.Hour)

	session, _ := store.Create("user-1")
	store.Delete(session.Token)

	_, ok := store.Get(session.Token)
	if ok {
		t.Error("expected session to be deleted")
	}
}

func TestSessionStore_Cleanup(t *testing.T) {
	store := NewSessionStore(1 * time.Millisecond)

	store.Create("user-1")
	store.Create("user-2")
	store.Create("user-3")

	time.Sleep(5 * time.Millisecond)

	count := store.Cleanup()
	if count != 3 {
		t.Errorf("Cleanup = %d, want 3", count)
	}
}

func TestSessionStore_NotFound(t *testing.T) {
	store := NewSessionStore(1 * time.Hour)

	_, ok := store.Get("nonexistent")
	if ok {
		t.Error("expected session not found")
	}
}

func TestUserStore_FindOrCreate(t *testing.T) {
	store := NewUserStore()

	info := &UserInfo{
		ProviderID: "12345",
		Provider:   "github",
		Email:      "test@example.com",
		Name:       "Test User",
	}

	user, isNew, err := store.FindOrCreate(info)
	if err != nil {
		t.Fatalf("FindOrCreate: %v", err)
	}
	if !isNew {
		t.Error("expected new user")
	}
	if user.Email != "test@example.com" {
		t.Errorf("Email = %q, want %q", user.Email, "test@example.com")
	}

	// Find same user again
	user2, isNew2, err := store.FindOrCreate(info)
	if err != nil {
		t.Fatalf("FindOrCreate (2nd): %v", err)
	}
	if isNew2 {
		t.Error("expected existing user")
	}
	if user2.ID != user.ID {
		t.Errorf("ID mismatch: %q != %q", user2.ID, user.ID)
	}
}

func TestUserStore_Get(t *testing.T) {
	store := NewUserStore()

	info := &UserInfo{
		ProviderID: "99",
		Provider:   "google",
		Email:      "user@example.com",
		Name:       "User",
	}

	user, _, _ := store.FindOrCreate(info)

	got, ok := store.Get(user.ID)
	if !ok {
		t.Fatal("user not found")
	}
	if got.Email != "user@example.com" {
		t.Errorf("Email = %q, want %q", got.Email, "user@example.com")
	}

	_, ok = store.Get("nonexistent")
	if ok {
		t.Error("expected user not found for bogus ID")
	}
}

func TestUserStore_UpdateStripeID(t *testing.T) {
	store := NewUserStore()

	info := &UserInfo{
		ProviderID: "42",
		Provider:   "github",
		Email:      "stripe@example.com",
		Name:       "Stripe User",
	}
	user, _, _ := store.FindOrCreate(info)

	err := store.UpdateStripeID(user.ID, "cus_123")
	if err != nil {
		t.Fatalf("UpdateStripeID: %v", err)
	}

	got, _ := store.Get(user.ID)
	if got.StripeID != "cus_123" {
		t.Errorf("StripeID = %q, want %q", got.StripeID, "cus_123")
	}

	err = store.UpdateStripeID("nonexistent", "cus_456")
	if err == nil {
		t.Error("expected error for nonexistent user")
	}
}

func TestGenerateState(t *testing.T) {
	s1, err := GenerateState()
	if err != nil {
		t.Fatalf("GenerateState: %v", err)
	}
	s2, _ := GenerateState()

	if s1 == "" || s2 == "" {
		t.Fatal("expected non-empty states")
	}
	if s1 == s2 {
		t.Error("expected unique states")
	}
	if len(s1) != 32 { // 16 bytes = 32 hex chars
		t.Errorf("state length = %d, want 32", len(s1))
	}
}

func TestOAuthProvider_AuthorizeURL(t *testing.T) {
	p := GitHubProvider("client-id", "client-secret", "http://localhost/callback")
	url := p.AuthorizeURL("test-state")

	if url == "" {
		t.Fatal("expected non-empty URL")
	}
	// Basic checks
	if !contains(url, "client_id=client-id") {
		t.Error("URL missing client_id")
	}
	if !contains(url, "state=test-state") {
		t.Error("URL missing state")
	}
	if !contains(url, "github.com") {
		t.Error("URL should be GitHub")
	}
}

func contains(s, sub string) bool {
	return len(s) >= len(sub) && (s == sub || len(s) > 0 && containsHelper(s, sub))
}

func containsHelper(s, sub string) bool {
	for i := 0; i <= len(s)-len(sub); i++ {
		if s[i:i+len(sub)] == sub {
			return true
		}
	}
	return false
}
