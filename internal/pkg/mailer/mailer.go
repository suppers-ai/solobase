package mailer

import (
	"context"
	"time"
)

// Mailer is the main interface for sending emails
type Mailer interface {
	// Send sends a single email
	Send(ctx context.Context, email *Email) error

	// SendBatch sends multiple emails
	SendBatch(ctx context.Context, emails []*Email) error

	// SendTemplate sends an email using a template
	SendTemplate(ctx context.Context, template string, data TemplateData, email *Email) error

	// Validate validates an email configuration
	Validate(email *Email) error
}

// Email represents an email message
type Email struct {
	From        Address                `json:"from"`
	To          []Address              `json:"to"`
	CC          []Address              `json:"cc,omitempty"`
	BCC         []Address              `json:"bcc,omitempty"`
	ReplyTo     *Address               `json:"replyTo,omitempty"`
	Subject     string                 `json:"subject"`
	TextBody    string                 `json:"textBody,omitempty"`
	HTMLBody    string                 `json:"htmlBody,omitempty"`
	Attachments []Attachment           `json:"attachments,omitempty"`
	Headers     map[string]string      `json:"headers,omitempty"`
	Tags        []string               `json:"tags,omitempty"`
	Metadata    map[string]interface{} `json:"metadata,omitempty"`
}

// Address represents an email address
type Address struct {
	Name  string `json:"name,omitempty"`
	Email string `json:"email"`
}

// Attachment represents an email attachment
type Attachment struct {
	Filename    string `json:"filename"`
	ContentType string `json:"contentType"`
	Content     []byte `json:"content"`
	ContentID   string `json:"contentId,omitempty"` // For inline attachments
}

// TemplateData represents data for email templates
type TemplateData map[string]interface{}

// Config holds mailer configuration
type Config struct {
	Provider       string                 `json:"provider"` // smtp, sendgrid, mailgun, ses, etc
	From           Address                `json:"from"`
	ReplyTo        *Address               `json:"replyTo,omitempty"`
	MaxRetries     int                    `json:"maxRetries"`
	RetryDelay     time.Duration          `json:"retryDelay"`
	Timeout        time.Duration          `json:"timeout"`
	RateLimit      int                    `json:"rateLimit"`      // emails per second
	TemplateEngine string                 `json:"templateEngine"` // go, handlebars, etc
	TemplatesPath  string                 `json:"templatesPath"`
	Extra          map[string]interface{} `json:"extra"`
}

// SMTPConfig holds SMTP-specific configuration
type SMTPConfig struct {
	Host         string        `json:"host"`
	Port         int           `json:"port"`
	Username     string        `json:"username"`
	Password     string        `json:"password"`
	AuthType     string        `json:"authType"` // plain, login, cram-md5, oauth2
	TLS          bool          `json:"tls"`
	StartTLS     bool          `json:"startTls"`
	InsecureSkip bool          `json:"insecureSkip"`
	PoolSize     int           `json:"poolSize"`
	MaxIdleConns int           `json:"maxIdleConns"`
	IdleTimeout  time.Duration `json:"idleTimeout"`
}

// SendGridConfig holds SendGrid-specific configuration
type SendGridConfig struct {
	APIKey string `json:"apiKey"`
}

// MailgunConfig holds Mailgun-specific configuration
type MailgunConfig struct {
	Domain string `json:"domain"`
	APIKey string `json:"apiKey"`
	Region string `json:"region"` // us or eu
}

// SESConfig holds AWS SES-specific configuration
type SESConfig struct {
	Region          string `json:"region"`
	AccessKeyID     string `json:"accessKeyId"`
	SecretAccessKey string `json:"secretAccessKey"`
	SessionToken    string `json:"sessionToken,omitempty"`
}

// New creates a new mailer instance based on the provider
func New(config Config) (Mailer, error) {
	switch config.Provider {
	case "smtp":
		return NewSMTP(config)
	case "sendgrid":
		return nil, ErrProviderNotImplemented
	case "mailgun":
		return nil, ErrProviderNotImplemented
	case "ses":
		return nil, ErrProviderNotImplemented
	case "mock":
		return NewMock(), nil
	default:
		return nil, ErrUnsupportedProvider
	}
}

// String returns the string representation of an Address
func (a Address) String() string {
	if a.Name != "" {
		return a.Name + " <" + a.Email + ">"
	}
	return a.Email
}

// Validate validates an email address
func (a Address) Validate() error {
	if a.Email == "" {
		return ErrInvalidEmail
	}
	// Basic email validation
	if !emailRegex.MatchString(a.Email) {
		return ErrInvalidEmail
	}
	return nil
}

// Validate validates an email
func (e *Email) Validate() error {
	if err := e.From.Validate(); err != nil {
		return err
	}

	if len(e.To) == 0 {
		return ErrNoRecipients
	}

	for _, to := range e.To {
		if err := to.Validate(); err != nil {
			return err
		}
	}

	for _, cc := range e.CC {
		if err := cc.Validate(); err != nil {
			return err
		}
	}

	for _, bcc := range e.BCC {
		if err := bcc.Validate(); err != nil {
			return err
		}
	}

	if e.Subject == "" {
		return ErrNoSubject
	}

	if e.TextBody == "" && e.HTMLBody == "" {
		return ErrNoBody
	}

	return nil
}

// Clone creates a deep copy of the email
func (e *Email) Clone() *Email {
	clone := &Email{
		From:     e.From,
		Subject:  e.Subject,
		TextBody: e.TextBody,
		HTMLBody: e.HTMLBody,
	}

	clone.To = make([]Address, len(e.To))
	copy(clone.To, e.To)

	if len(e.CC) > 0 {
		clone.CC = make([]Address, len(e.CC))
		copy(clone.CC, e.CC)
	}

	if len(e.BCC) > 0 {
		clone.BCC = make([]Address, len(e.BCC))
		copy(clone.BCC, e.BCC)
	}

	if e.ReplyTo != nil {
		replyTo := *e.ReplyTo
		clone.ReplyTo = &replyTo
	}

	if len(e.Attachments) > 0 {
		clone.Attachments = make([]Attachment, len(e.Attachments))
		for i, att := range e.Attachments {
			clone.Attachments[i] = Attachment{
				Filename:    att.Filename,
				ContentType: att.ContentType,
				ContentID:   att.ContentID,
				Content:     append([]byte(nil), att.Content...),
			}
		}
	}

	if len(e.Headers) > 0 {
		clone.Headers = make(map[string]string)
		for k, v := range e.Headers {
			clone.Headers[k] = v
		}
	}

	if len(e.Tags) > 0 {
		clone.Tags = make([]string, len(e.Tags))
		copy(clone.Tags, e.Tags)
	}

	if len(e.Metadata) > 0 {
		clone.Metadata = make(map[string]interface{})
		for k, v := range e.Metadata {
			clone.Metadata[k] = v
		}
	}

	return clone
}
