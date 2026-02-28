package auth

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"sync"
	"time"
)

// User represents a registered user in the control plane.
type User struct {
	ID         string    `json:"id"`
	Email      string    `json:"email"`
	Name       string    `json:"name"`
	AvatarURL  string    `json:"avatar_url"`
	Provider   string    `json:"provider"`
	ProviderID string    `json:"provider_id"`
	StripeID   string    `json:"stripe_id,omitempty"`
	CreatedAt  time.Time `json:"created_at"`
}

// Session represents an active user session.
type Session struct {
	Token     string    `json:"token"`
	UserID    string    `json:"user_id"`
	ExpiresAt time.Time `json:"expires_at"`
	CreatedAt time.Time `json:"created_at"`
}

// SessionStore manages user sessions in memory.
type SessionStore struct {
	mu       sync.RWMutex
	sessions map[string]*Session
	ttl      time.Duration
}

// NewSessionStore creates a session store with the given TTL.
func NewSessionStore(ttl time.Duration) *SessionStore {
	return &SessionStore{
		sessions: make(map[string]*Session),
		ttl:      ttl,
	}
}

// Create creates a new session for a user and returns the token.
func (s *SessionStore) Create(userID string) (*Session, error) {
	token, err := generateToken()
	if err != nil {
		return nil, fmt.Errorf("generate token: %w", err)
	}

	session := &Session{
		Token:     token,
		UserID:    userID,
		ExpiresAt: time.Now().Add(s.ttl),
		CreatedAt: time.Now(),
	}

	s.mu.Lock()
	s.sessions[token] = session
	s.mu.Unlock()

	return session, nil
}

// Get returns a session by token if it exists and hasn't expired.
func (s *SessionStore) Get(token string) (*Session, bool) {
	s.mu.RLock()
	session, ok := s.sessions[token]
	s.mu.RUnlock()

	if !ok {
		return nil, false
	}
	if time.Now().After(session.ExpiresAt) {
		s.Delete(token)
		return nil, false
	}
	return session, true
}

// Delete removes a session.
func (s *SessionStore) Delete(token string) {
	s.mu.Lock()
	delete(s.sessions, token)
	s.mu.Unlock()
}

// Cleanup removes all expired sessions.
func (s *SessionStore) Cleanup() int {
	s.mu.Lock()
	defer s.mu.Unlock()

	count := 0
	now := time.Now()
	for token, session := range s.sessions {
		if now.After(session.ExpiresAt) {
			delete(s.sessions, token)
			count++
		}
	}
	return count
}

// UserStore manages user accounts in memory.
type UserStore struct {
	mu    sync.RWMutex
	users map[string]*User         // by ID
	byProvider map[string]*User    // by "provider:provider_id"
}

// NewUserStore creates a user store.
func NewUserStore() *UserStore {
	return &UserStore{
		users:      make(map[string]*User),
		byProvider: make(map[string]*User),
	}
}

// FindOrCreate finds a user by OAuth provider info, or creates a new one.
func (s *UserStore) FindOrCreate(info *UserInfo) (*User, bool, error) {
	key := info.Provider + ":" + info.ProviderID

	s.mu.RLock()
	if u, ok := s.byProvider[key]; ok {
		s.mu.RUnlock()
		return u, false, nil
	}
	s.mu.RUnlock()

	id, err := generateToken()
	if err != nil {
		return nil, false, fmt.Errorf("generate user ID: %w", err)
	}

	user := &User{
		ID:         id,
		Email:      info.Email,
		Name:       info.Name,
		AvatarURL:  info.AvatarURL,
		Provider:   info.Provider,
		ProviderID: info.ProviderID,
		CreatedAt:  time.Now(),
	}

	s.mu.Lock()
	// Double-check after acquiring write lock
	if u, ok := s.byProvider[key]; ok {
		s.mu.Unlock()
		return u, false, nil
	}
	s.users[id] = user
	s.byProvider[key] = user
	s.mu.Unlock()

	return user, true, nil
}

// Get returns a user by ID.
func (s *UserStore) Get(id string) (*User, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	u, ok := s.users[id]
	return u, ok
}

// UpdateStripeID sets the Stripe customer ID for a user.
func (s *UserStore) UpdateStripeID(userID, stripeID string) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	u, ok := s.users[userID]
	if !ok {
		return fmt.Errorf("user %s not found", userID)
	}
	u.StripeID = stripeID
	return nil
}

func generateToken() (string, error) {
	b := make([]byte, 24)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return hex.EncodeToString(b), nil
}
