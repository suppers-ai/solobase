package mailer

import (
	"context"
	"sync"
)

// MockMailer is a mock implementation for testing
type MockMailer struct {
	mu         sync.RWMutex
	SentEmails []*Email
	ShouldFail bool
	FailError  error
}

// NewMock creates a new mock mailer
func NewMock() *MockMailer {
	return &MockMailer{
		SentEmails: make([]*Email, 0),
	}
}

// Send mock sends an email
func (m *MockMailer) Send(ctx context.Context, email *Email) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	if m.ShouldFail {
		if m.FailError != nil {
			return m.FailError
		}
		return ErrSendFailed
	}

	if err := m.Validate(email); err != nil {
		return err
	}

	m.SentEmails = append(m.SentEmails, email.Clone())
	return nil
}

// SendBatch mock sends multiple emails
func (m *MockMailer) SendBatch(ctx context.Context, emails []*Email) error {
	for _, email := range emails {
		if err := m.Send(ctx, email); err != nil {
			return err
		}
	}
	return nil
}

// SendTemplate mock sends a templated email
func (m *MockMailer) SendTemplate(ctx context.Context, template string, data TemplateData, email *Email) error {
	// Just set some dummy content
	email.HTMLBody = "Mock HTML Body: " + template
	email.TextBody = "Mock Text Body: " + template

	return m.Send(ctx, email)
}

// Validate validates an email
func (m *MockMailer) Validate(email *Email) error {
	return email.Validate()
}

// GetSentEmails returns all sent emails
func (m *MockMailer) GetSentEmails() []*Email {
	m.mu.RLock()
	defer m.mu.RUnlock()

	emails := make([]*Email, len(m.SentEmails))
	copy(emails, m.SentEmails)
	return emails
}

// GetLastSentEmail returns the last sent email
func (m *MockMailer) GetLastSentEmail() *Email {
	m.mu.RLock()
	defer m.mu.RUnlock()

	if len(m.SentEmails) == 0 {
		return nil
	}

	return m.SentEmails[len(m.SentEmails)-1]
}

// Clear clears all sent emails
func (m *MockMailer) Clear() {
	m.mu.Lock()
	defer m.mu.Unlock()

	m.SentEmails = make([]*Email, 0)
	m.ShouldFail = false
	m.FailError = nil
}

// SetShouldFail sets whether the mock should fail
func (m *MockMailer) SetShouldFail(shouldFail bool, err error) {
	m.mu.Lock()
	defer m.mu.Unlock()

	m.ShouldFail = shouldFail
	m.FailError = err
}
