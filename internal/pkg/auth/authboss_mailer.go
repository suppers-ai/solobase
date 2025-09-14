package auth

import (
	"context"
	"fmt"

	"github.com/suppers-ai/solobase/internal/pkg/mailer"
	"github.com/volatiletech/authboss/v3"
)

// AuthbossMailer adapts the mailer package to authboss.Mailer interface
type AuthbossMailer struct {
	mailer      mailer.Mailer
	defaultFrom mailer.Address
}

// NewAuthbossMailer creates a new authboss mailer adapter
func NewAuthbossMailer(m mailer.Mailer) *AuthbossMailer {
	return &AuthbossMailer{
		mailer: m,
	}
}

// NewAuthbossMailerWithFrom creates a new authboss mailer adapter with default from address
func NewAuthbossMailerWithFrom(m mailer.Mailer, from mailer.Address) *AuthbossMailer {
	return &AuthbossMailer{
		mailer:      m,
		defaultFrom: from,
	}
}

// Send implements authboss.Mailer interface
func (a *AuthbossMailer) Send(ctx context.Context, email authboss.Email) error {
	// Convert authboss.Email to mailer.Email
	mailEmail := &mailer.Email{
		Subject:  email.Subject,
		TextBody: email.TextBody,
		HTMLBody: email.HTMLBody,
	}

	// Set From address
	if email.From != "" {
		mailEmail.From = mailer.Address{Email: email.From}
	} else if a.defaultFrom.Email != "" {
		mailEmail.From = a.defaultFrom
	} else {
		return fmt.Errorf("no from address specified")
	}

	// Convert To addresses
	for _, to := range email.To {
		mailEmail.To = append(mailEmail.To, mailer.Address{Email: to})
	}

	// Convert Cc addresses (note the lowercase 'c')
	for _, cc := range email.Cc {
		mailEmail.CC = append(mailEmail.CC, mailer.Address{Email: cc})
	}

	// Convert Bcc addresses (note the lowercase 'cc')
	for _, bcc := range email.Bcc {
		mailEmail.BCC = append(mailEmail.BCC, mailer.Address{Email: bcc})
	}

	// Convert ReplyTo address
	if email.ReplyTo != "" {
		mailEmail.ReplyTo = &mailer.Address{Email: email.ReplyTo}
	}

	// Send email
	return a.mailer.Send(ctx, mailEmail)
}
