package config

import (
	"context"
	"fmt"
	"net/smtp"
	"strings"

	"github.com/volatiletech/authboss/v3"
)

type SMTPMailer struct {
	from     string
	host     string
	port     int
	username string
	password string
}

func NewSMTPMailer(from, host string, port int, username, password string) *SMTPMailer {
	return &SMTPMailer{
		from:     from,
		host:     host,
		port:     port,
		username: username,
		password: password,
	}
}

func (m *SMTPMailer) Send(ctx context.Context, email authboss.Email) error {
	addr := fmt.Sprintf("%s:%d", m.host, m.port)

	auth := smtp.PlainAuth("", m.username, m.password, m.host)

	var to []string
	to = append(to, email.To...)
	to = append(to, email.Cc...)
	to = append(to, email.Bcc...)

	msg := m.buildMessage(email)

	err := smtp.SendMail(addr, auth, m.from, to, []byte(msg))
	if err != nil {
		return fmt.Errorf("failed to send email: %w", err)
	}

	return nil
}

func (m *SMTPMailer) buildMessage(email authboss.Email) string {
	var msg strings.Builder

	msg.WriteString(fmt.Sprintf("From: %s\r\n", m.from))

	if len(email.To) > 0 {
		msg.WriteString(fmt.Sprintf("To: %s\r\n", strings.Join(email.To, ", ")))
	}

	if len(email.Cc) > 0 {
		msg.WriteString(fmt.Sprintf("Cc: %s\r\n", strings.Join(email.Cc, ", ")))
	}

	if email.ReplyTo != "" {
		msg.WriteString(fmt.Sprintf("Reply-To: %s\r\n", email.ReplyTo))
	}

	msg.WriteString(fmt.Sprintf("Subject: %s\r\n", email.Subject))

	if email.TextBody != "" && email.HTMLBody != "" {
		boundary := "boundary123"
		msg.WriteString(fmt.Sprintf("Content-Type: multipart/alternative; boundary=%s\r\n", boundary))
		msg.WriteString("\r\n")

		msg.WriteString(fmt.Sprintf("--%s\r\n", boundary))
		msg.WriteString("Content-Type: text/plain; charset=utf-8\r\n")
		msg.WriteString("\r\n")
		msg.WriteString(email.TextBody)
		msg.WriteString("\r\n")

		msg.WriteString(fmt.Sprintf("--%s\r\n", boundary))
		msg.WriteString("Content-Type: text/html; charset=utf-8\r\n")
		msg.WriteString("\r\n")
		msg.WriteString(email.HTMLBody)
		msg.WriteString("\r\n")

		msg.WriteString(fmt.Sprintf("--%s--\r\n", boundary))
	} else if email.HTMLBody != "" {
		msg.WriteString("Content-Type: text/html; charset=utf-8\r\n")
		msg.WriteString("\r\n")
		msg.WriteString(email.HTMLBody)
	} else {
		msg.WriteString("Content-Type: text/plain; charset=utf-8\r\n")
		msg.WriteString("\r\n")
		msg.WriteString(email.TextBody)
	}

	return msg.String()
}
