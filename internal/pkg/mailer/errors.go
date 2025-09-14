package mailer

import "errors"

var (
	// Provider errors
	ErrUnsupportedProvider    = errors.New("unsupported mail provider")
	ErrProviderNotImplemented = errors.New("mail provider not implemented")
	ErrProviderNotConfigured  = errors.New("mail provider not configured")

	// Email validation errors
	ErrInvalidEmail    = errors.New("invalid email address")
	ErrNoRecipients    = errors.New("no recipients specified")
	ErrNoSubject       = errors.New("no subject specified")
	ErrNoBody          = errors.New("no body content specified")
	ErrInvalidTemplate = errors.New("invalid template")

	// Sending errors
	ErrConnectionFailed  = errors.New("failed to connect to mail server")
	ErrAuthFailed        = errors.New("authentication failed")
	ErrSendFailed        = errors.New("failed to send email")
	ErrRateLimitExceeded = errors.New("rate limit exceeded")
	ErrTimeout           = errors.New("operation timed out")

	// Template errors
	ErrTemplateNotFound   = errors.New("template not found")
	ErrTemplateParseError = errors.New("failed to parse template")
	ErrTemplateExecError  = errors.New("failed to execute template")
)
