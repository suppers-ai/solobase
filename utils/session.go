package utils

import (
	"net/http"

	"github.com/gorilla/sessions"
	"github.com/suppers-ai/solobase/constants"
)

// SessionHelper provides common session operations
type SessionHelper struct {
	store sessions.Store
	name  string
}

// NewSessionHelper creates a new session helper
func NewSessionHelper(store sessions.Store, sessionName string) *SessionHelper {
	return &SessionHelper{
		store: store,
		name:  sessionName,
	}
}

// GetUserEmail retrieves the user email from session
func (s *SessionHelper) GetUserEmail(r *http.Request) (string, error) {
	session, err := s.store.Get(r, s.name)
	if err != nil {
		return "", err
	}

	email, ok := session.Values[constants.SessionKeyEmail].(string)
	if !ok {
		return "", nil
	}

	return email, nil
}

// GetUserID retrieves the user ID from session
func (s *SessionHelper) GetUserID(r *http.Request) (string, error) {
	session, err := s.store.Get(r, s.name)
	if err != nil {
		return "", err
	}

	userID, ok := session.Values[constants.SessionKeyUserID].(string)
	if !ok {
		return "", nil
	}

	return userID, nil
}

// GetUserRole retrieves the user role from session
func (s *SessionHelper) GetUserRole(r *http.Request) (string, error) {
	session, err := s.store.Get(r, s.name)
	if err != nil {
		return "", err
	}

	role, ok := session.Values[constants.SessionKeyRole].(string)
	if !ok {
		return "", nil
	}

	return role, nil
}

// SetUserData sets user data in session
func (s *SessionHelper) SetUserData(w http.ResponseWriter, r *http.Request, userID, email, role string) error {
	session, err := s.store.Get(r, s.name)
	if err != nil {
		return err
	}

	session.Values[constants.SessionKeyUserID] = userID
	session.Values[constants.SessionKeyEmail] = email
	session.Values[constants.SessionKeyRole] = role

	return session.Save(r, w)
}

// Clear clears all session data
func (s *SessionHelper) Clear(w http.ResponseWriter, r *http.Request) error {
	session, err := s.store.Get(r, s.name)
	if err != nil {
		return err
	}

	session.Values = make(map[interface{}]interface{})
	session.Options.MaxAge = -1

	return session.Save(r, w)
}
