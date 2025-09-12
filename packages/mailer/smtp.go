package mailer

import (
	"context"
	"crypto/tls"
	"fmt"
	"net"
	"net/smtp"
	"regexp"
	"strings"
	"sync"
	"time"
)

var emailRegex = regexp.MustCompile(`^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$`)

// SMTPMailer implements the Mailer interface using SMTP
type SMTPMailer struct {
	config     Config
	smtpConfig SMTPConfig
	pool       chan *smtpConnection
	mu         sync.RWMutex
	closed     bool
}

type smtpConnection struct {
	client   *smtp.Client
	lastUsed time.Time
}

// NewSMTP creates a new SMTP mailer
func NewSMTP(config Config) (*SMTPMailer, error) {
	smtpConfig := SMTPConfig{
		Port:         587,
		AuthType:     "plain",
		StartTLS:     true,
		PoolSize:     5,
		MaxIdleConns: 5,
		IdleTimeout:  5 * time.Minute,
	}

	// Parse SMTP config from Extra
	if config.Extra != nil {
		if host, ok := config.Extra["smtp_host"].(string); ok {
			smtpConfig.Host = host
		}
		if port, ok := config.Extra["smtp_port"].(int); ok {
			smtpConfig.Port = port
		}
		if username, ok := config.Extra["smtp_username"].(string); ok {
			smtpConfig.Username = username
		}
		if password, ok := config.Extra["smtp_password"].(string); ok {
			smtpConfig.Password = password
		}
		if authType, ok := config.Extra["smtp_auth_type"].(string); ok {
			smtpConfig.AuthType = authType
		}
		if tls, ok := config.Extra["smtp_tls"].(bool); ok {
			smtpConfig.TLS = tls
		}
		if startTLS, ok := config.Extra["smtp_start_tls"].(bool); ok {
			smtpConfig.StartTLS = startTLS
		}
		if poolSize, ok := config.Extra["smtp_pool_size"].(int); ok {
			smtpConfig.PoolSize = poolSize
		}
	}

	if smtpConfig.Host == "" {
		return nil, ErrProviderNotConfigured
	}

	mailer := &SMTPMailer{
		config:     config,
		smtpConfig: smtpConfig,
		pool:       make(chan *smtpConnection, smtpConfig.PoolSize),
	}

	// Initialize connection pool
	for i := 0; i < smtpConfig.PoolSize; i++ {
		mailer.pool <- nil
	}

	return mailer, nil
}

// Send sends a single email via SMTP
func (m *SMTPMailer) Send(ctx context.Context, email *Email) error {
	if err := m.Validate(email); err != nil {
		return err
	}

	// Get connection from pool
	conn, err := m.getConnection(ctx)
	if err != nil {
		return err
	}
	defer m.putConnection(conn)

	// Set sender
	if err := conn.client.Mail(email.From.Email); err != nil {
		return fmt.Errorf("%w: %v", ErrSendFailed, err)
	}

	// Add recipients
	recipients := make([]string, 0)
	for _, to := range email.To {
		if err := conn.client.Rcpt(to.Email); err != nil {
			return fmt.Errorf("%w: %v", ErrSendFailed, err)
		}
		recipients = append(recipients, to.Email)
	}

	for _, cc := range email.CC {
		if err := conn.client.Rcpt(cc.Email); err != nil {
			return fmt.Errorf("%w: %v", ErrSendFailed, err)
		}
		recipients = append(recipients, cc.Email)
	}

	for _, bcc := range email.BCC {
		if err := conn.client.Rcpt(bcc.Email); err != nil {
			return fmt.Errorf("%w: %v", ErrSendFailed, err)
		}
		recipients = append(recipients, bcc.Email)
	}

	// Send email data
	w, err := conn.client.Data()
	if err != nil {
		return fmt.Errorf("%w: %v", ErrSendFailed, err)
	}

	// Build email message
	msg := m.buildMessage(email)
	_, err = w.Write([]byte(msg))
	if err != nil {
		return fmt.Errorf("%w: %v", ErrSendFailed, err)
	}

	err = w.Close()
	if err != nil {
		return fmt.Errorf("%w: %v", ErrSendFailed, err)
	}

	return nil
}

// SendBatch sends multiple emails
func (m *SMTPMailer) SendBatch(ctx context.Context, emails []*Email) error {
	var wg sync.WaitGroup
	errors := make(chan error, len(emails))

	// Rate limiting
	rateLimiter := time.NewTicker(time.Second / time.Duration(m.config.RateLimit))
	if m.config.RateLimit <= 0 {
		rateLimiter.Stop()
	} else {
		defer rateLimiter.Stop()
	}

	for _, email := range emails {
		if m.config.RateLimit > 0 {
			<-rateLimiter.C
		}

		wg.Add(1)
		go func(e *Email) {
			defer wg.Done()
			if err := m.Send(ctx, e); err != nil {
				errors <- err
			}
		}(email)
	}

	wg.Wait()
	close(errors)

	// Collect errors
	var errs []error
	for err := range errors {
		errs = append(errs, err)
	}

	if len(errs) > 0 {
		return fmt.Errorf("batch send failed: %v", errs)
	}

	return nil
}

// SendTemplate sends an email using a template
func (m *SMTPMailer) SendTemplate(ctx context.Context, template string, data TemplateData, email *Email) error {
	// Parse and execute template
	tmpl := NewTemplate(m.config.TemplatesPath)

	htmlBody, err := tmpl.RenderHTML(template, data)
	if err != nil {
		return err
	}

	textBody, err := tmpl.RenderText(template, data)
	if err != nil {
		// If text template doesn't exist, that's okay
		textBody = ""
	}

	// Update email body
	email.HTMLBody = htmlBody
	if textBody != "" {
		email.TextBody = textBody
	}

	return m.Send(ctx, email)
}

// Validate validates an email
func (m *SMTPMailer) Validate(email *Email) error {
	return email.Validate()
}

// getConnection gets a connection from the pool
func (m *SMTPMailer) getConnection(ctx context.Context) (*smtpConnection, error) {
	select {
	case conn := <-m.pool:
		if conn != nil && time.Since(conn.lastUsed) < m.smtpConfig.IdleTimeout {
			// Test connection
			if err := conn.client.Noop(); err == nil {
				return conn, nil
			}
			// Connection is dead, close it
			conn.client.Close()
		}

		// Create new connection
		return m.createConnection()
	case <-ctx.Done():
		return nil, ctx.Err()
	}
}

// putConnection returns a connection to the pool
func (m *SMTPMailer) putConnection(conn *smtpConnection) {
	if conn == nil {
		return
	}

	conn.lastUsed = time.Now()

	select {
	case m.pool <- conn:
		// Connection returned to pool
	default:
		// Pool is full, close connection
		conn.client.Close()
	}
}

// createConnection creates a new SMTP connection
func (m *SMTPMailer) createConnection() (*smtpConnection, error) {
	addr := fmt.Sprintf("%s:%d", m.smtpConfig.Host, m.smtpConfig.Port)

	var client *smtp.Client

	if m.smtpConfig.TLS {
		// Direct TLS connection
		tlsConfig := &tls.Config{
			ServerName:         m.smtpConfig.Host,
			InsecureSkipVerify: m.smtpConfig.InsecureSkip,
		}

		tlsConn, err := tls.Dial("tcp", addr, tlsConfig)
		if err != nil {
			return nil, fmt.Errorf("%w: %v", ErrConnectionFailed, err)
		}

		client, err = smtp.NewClient(tlsConn, m.smtpConfig.Host)
		if err != nil {
			tlsConn.Close()
			return nil, fmt.Errorf("%w: %v", ErrConnectionFailed, err)
		}
	} else {
		// Plain connection
		conn, err := net.DialTimeout("tcp", addr, m.config.Timeout)
		if err != nil {
			return nil, fmt.Errorf("%w: %v", ErrConnectionFailed, err)
		}

		client, err = smtp.NewClient(conn, m.smtpConfig.Host)
		if err != nil {
			conn.Close()
			return nil, fmt.Errorf("%w: %v", ErrConnectionFailed, err)
		}

		// STARTTLS if required
		if m.smtpConfig.StartTLS {
			tlsConfig := &tls.Config{
				ServerName:         m.smtpConfig.Host,
				InsecureSkipVerify: m.smtpConfig.InsecureSkip,
			}

			if err := client.StartTLS(tlsConfig); err != nil {
				client.Close()
				return nil, fmt.Errorf("%w: %v", ErrConnectionFailed, err)
			}
		}
	}

	// Authenticate
	if m.smtpConfig.Username != "" && m.smtpConfig.Password != "" {
		var auth smtp.Auth

		switch m.smtpConfig.AuthType {
		case "plain":
			auth = smtp.PlainAuth("", m.smtpConfig.Username, m.smtpConfig.Password, m.smtpConfig.Host)
		case "login":
			auth = &loginAuth{m.smtpConfig.Username, m.smtpConfig.Password}
		case "cram-md5":
			auth = smtp.CRAMMD5Auth(m.smtpConfig.Username, m.smtpConfig.Password)
		default:
			auth = smtp.PlainAuth("", m.smtpConfig.Username, m.smtpConfig.Password, m.smtpConfig.Host)
		}

		if err := client.Auth(auth); err != nil {
			client.Close()
			return nil, fmt.Errorf("%w: %v", ErrAuthFailed, err)
		}
	}

	return &smtpConnection{
		client:   client,
		lastUsed: time.Now(),
	}, nil
}

// buildMessage builds the email message
func (m *SMTPMailer) buildMessage(email *Email) string {
	var msg strings.Builder

	// Headers
	msg.WriteString(fmt.Sprintf("From: %s\r\n", email.From.String()))

	if len(email.To) > 0 {
		to := make([]string, len(email.To))
		for i, addr := range email.To {
			to[i] = addr.String()
		}
		msg.WriteString(fmt.Sprintf("To: %s\r\n", strings.Join(to, ", ")))
	}

	if len(email.CC) > 0 {
		cc := make([]string, len(email.CC))
		for i, addr := range email.CC {
			cc[i] = addr.String()
		}
		msg.WriteString(fmt.Sprintf("Cc: %s\r\n", strings.Join(cc, ", ")))
	}

	if email.ReplyTo != nil {
		msg.WriteString(fmt.Sprintf("Reply-To: %s\r\n", email.ReplyTo.String()))
	}

	msg.WriteString(fmt.Sprintf("Subject: %s\r\n", email.Subject))
	msg.WriteString(fmt.Sprintf("Date: %s\r\n", time.Now().Format(time.RFC1123Z)))
	msg.WriteString("MIME-Version: 1.0\r\n")

	// Custom headers
	for key, value := range email.Headers {
		msg.WriteString(fmt.Sprintf("%s: %s\r\n", key, value))
	}

	// Body
	if email.HTMLBody != "" && email.TextBody != "" {
		// Multipart message
		boundary := fmt.Sprintf("boundary_%d", time.Now().Unix())
		msg.WriteString(fmt.Sprintf("Content-Type: multipart/alternative; boundary=\"%s\"\r\n", boundary))
		msg.WriteString("\r\n")

		// Text part
		msg.WriteString(fmt.Sprintf("--%s\r\n", boundary))
		msg.WriteString("Content-Type: text/plain; charset=\"UTF-8\"\r\n")
		msg.WriteString("Content-Transfer-Encoding: quoted-printable\r\n")
		msg.WriteString("\r\n")
		msg.WriteString(email.TextBody)
		msg.WriteString("\r\n")

		// HTML part
		msg.WriteString(fmt.Sprintf("--%s\r\n", boundary))
		msg.WriteString("Content-Type: text/html; charset=\"UTF-8\"\r\n")
		msg.WriteString("Content-Transfer-Encoding: quoted-printable\r\n")
		msg.WriteString("\r\n")
		msg.WriteString(email.HTMLBody)
		msg.WriteString("\r\n")

		msg.WriteString(fmt.Sprintf("--%s--\r\n", boundary))
	} else if email.HTMLBody != "" {
		// HTML only
		msg.WriteString("Content-Type: text/html; charset=\"UTF-8\"\r\n")
		msg.WriteString("Content-Transfer-Encoding: quoted-printable\r\n")
		msg.WriteString("\r\n")
		msg.WriteString(email.HTMLBody)
	} else {
		// Text only
		msg.WriteString("Content-Type: text/plain; charset=\"UTF-8\"\r\n")
		msg.WriteString("Content-Transfer-Encoding: quoted-printable\r\n")
		msg.WriteString("\r\n")
		msg.WriteString(email.TextBody)
	}

	return msg.String()
}

// Close closes the SMTP mailer
func (m *SMTPMailer) Close() error {
	m.mu.Lock()
	defer m.mu.Unlock()

	if m.closed {
		return nil
	}

	m.closed = true
	close(m.pool)

	// Close all connections
	for conn := range m.pool {
		if conn != nil {
			conn.client.Close()
		}
	}

	return nil
}

// loginAuth implements LOGIN authentication
type loginAuth struct {
	username, password string
}

func (a *loginAuth) Start(server *smtp.ServerInfo) (string, []byte, error) {
	return "LOGIN", []byte(a.username), nil
}

func (a *loginAuth) Next(fromServer []byte, more bool) ([]byte, error) {
	if more {
		switch string(fromServer) {
		case "Username:":
			return []byte(a.username), nil
		case "Password:":
			return []byte(a.password), nil
		}
	}
	return nil, nil
}
