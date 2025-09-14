package auth

import (
	"github.com/gorilla/sessions"
	"github.com/volatiletech/authboss/v3"
	"net/http"
)

// SessionStoreAdapter adapts gorilla/sessions.Store to authboss interfaces
type SessionStoreAdapter struct {
	store       sessions.Store
	sessionName string
}

// NewSessionStoreAdapter creates a new session store adapter
func NewSessionStoreAdapter(store sessions.Store, sessionName string) *SessionStoreAdapter {
	return &SessionStoreAdapter{
		store:       store,
		sessionName: sessionName,
	}
}

// ReadState implements authboss.ClientStateReadWriter
func (s *SessionStoreAdapter) ReadState(r *http.Request) (authboss.ClientState, error) {
	session, err := s.store.Get(r, s.sessionName)
	if err != nil {
		// New session if not found
		session, _ = s.store.New(r, s.sessionName)
	}
	return &ClientState{session: session}, nil
}

// WriteState implements authboss.ClientStateReadWriter
func (s *SessionStoreAdapter) WriteState(w http.ResponseWriter, state authboss.ClientState, _ []authboss.ClientStateEvent) error {
	if cs, ok := state.(*ClientState); ok {
		return cs.session.Save(nil, w)
	}
	return nil
}

// ClientState wraps a gorilla session
type ClientState struct {
	session *sessions.Session
}

// Get implements authboss.ClientState
func (c *ClientState) Get(key string) (string, bool) {
	val, ok := c.session.Values[key]
	if !ok {
		return "", false
	}
	str, ok := val.(string)
	return str, ok
}

// Put implements authboss.ClientState
func (c *ClientState) Put(key, value string) {
	c.session.Values[key] = value
}

// Del implements authboss.ClientState
func (c *ClientState) Del(key string) {
	delete(c.session.Values, key)
}
