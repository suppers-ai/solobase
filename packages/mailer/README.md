# Mailer Package

A flexible, extensible email sending library for Go applications with support for multiple providers and templating.

## Features

- **Multiple Providers**: SMTP, SendGrid, Mailgun, AWS SES (SMTP implemented, others ready for extension)
- **Template Support**: HTML and text templates with custom functions
- **Connection Pooling**: Efficient SMTP connection management
- **Batch Sending**: Send multiple emails efficiently
- **Rate Limiting**: Built-in rate limiting for bulk sends
- **Mock Implementation**: Perfect for testing
- **Validation**: Email address and content validation
- **Attachments**: Support for file attachments
- **Retry Logic**: Configurable retry mechanism

## Installation

```bash
go get github.com/suppers-ai/mailer
```

## Quick Start

```go
package main

import (
    "context"
    "log"
    "time"
    
    "github.com/suppers-ai/mailer"
)

func main() {
    // Create SMTP mailer
    mailService, err := mailer.New(mailer.Config{
        Provider: "smtp",
        From: mailer.Address{
            Name:  "My Application",
            Email: "noreply@example.com",
        },
        Timeout: 10 * time.Second,
        Extra: map[string]interface{}{
            "smtp_host":     "smtp.gmail.com",
            "smtp_port":     587,
            "smtp_username": "your-email@gmail.com",
            "smtp_password": "your-password",
            "smtp_start_tls": true,
        },
    })
    if err != nil {
        log.Fatal(err)
    }
    
    // Send a simple email
    email := &mailer.Email{
        From: mailer.Address{
            Name:  "John Doe",
            Email: "john@example.com",
        },
        To: []mailer.Address{
            {Email: "recipient@example.com"},
        },
        Subject:  "Hello from Mailer",
        TextBody: "This is a test email.",
        HTMLBody: "<h1>This is a test email</h1>",
    }
    
    err = mailService.Send(context.Background(), email)
    if err != nil {
        log.Fatal(err)
    }
}
```

## Configuration

### SMTP Configuration

```go
config := mailer.Config{
    Provider: "smtp",
    From: mailer.Address{
        Name:  "Default Sender",
        Email: "noreply@example.com",
    },
    MaxRetries: 3,
    RetryDelay: 5 * time.Second,
    Timeout:    30 * time.Second,
    RateLimit:  10, // emails per second
    Extra: map[string]interface{}{
        "smtp_host":       "smtp.example.com",
        "smtp_port":       587,
        "smtp_username":   "username",
        "smtp_password":   "password",
        "smtp_auth_type":  "plain", // plain, login, cram-md5
        "smtp_tls":        false,   // Direct TLS connection
        "smtp_start_tls":  true,    // STARTTLS
        "smtp_pool_size":  5,        // Connection pool size
    },
}
```

### Mock Configuration (for testing)

```go
mockMailer := mailer.NewMock()

// Send emails (they will be stored in memory)
mockMailer.Send(ctx, email)

// Check sent emails
sentEmails := mockMailer.GetSentEmails()
lastEmail := mockMailer.GetLastSentEmail()

// Configure failure for testing
mockMailer.SetShouldFail(true, errors.New("smtp error"))
```

## Email Templates

### Using Templates

```go
// Configure templates
config := mailer.Config{
    Provider:       "smtp",
    TemplatesPath:  "./templates",
    TemplateEngine: "go",
    // ... other config
}

// Send templated email
templateData := mailer.TemplateData{
    "Name":       "John Doe",
    "AppName":    "My App",
    "ConfirmURL": "https://example.com/confirm",
    "Year":       2024,
}

err := mailService.SendTemplate(
    context.Background(),
    "welcome", // template name (looks for welcome.html and welcome.txt)
    templateData,
    &mailer.Email{
        To:      []mailer.Address{{Email: "user@example.com"}},
        Subject: "Welcome to Our App",
    },
)
```

### Template Files

Create template files in your templates directory:

**templates/welcome.html**:
```html
<!DOCTYPE html>
<html>
<body>
    <h1>Welcome {{.Name}}!</h1>
    <p>Thank you for joining {{.AppName}}.</p>
    <a href="{{.ConfirmURL}}">Confirm your email</a>
</body>
</html>
```

**templates/welcome.txt**:
```text
Welcome {{.Name}}!

Thank you for joining {{.AppName}}.

Confirm your email: {{.ConfirmURL}}
```

## Batch Sending

```go
emails := []*mailer.Email{
    {To: []mailer.Address{{Email: "user1@example.com"}}, Subject: "Email 1"},
    {To: []mailer.Address{{Email: "user2@example.com"}}, Subject: "Email 2"},
    {To: []mailer.Address{{Email: "user3@example.com"}}, Subject: "Email 3"},
}

// Send with rate limiting
err := mailService.SendBatch(context.Background(), emails)
```

## Attachments

```go
email := &mailer.Email{
    To:      []mailer.Address{{Email: "user@example.com"}},
    Subject: "Email with Attachment",
    TextBody: "Please see the attached file.",
    Attachments: []mailer.Attachment{
        {
            Filename:    "report.pdf",
            ContentType: "application/pdf",
            Content:     pdfBytes,
        },
    },
}
```

## Custom Headers and Metadata

```go
email := &mailer.Email{
    To:      []mailer.Address{{Email: "user@example.com"}},
    Subject: "Custom Headers",
    Headers: map[string]string{
        "X-Campaign-ID": "summer-sale",
        "X-User-ID":     "12345",
    },
    Tags: []string{"marketing", "newsletter"},
    Metadata: map[string]interface{}{
        "user_id":     12345,
        "campaign_id": "summer-sale",
    },
}
```

## Provider Extensibility

To add a new provider, implement the `Mailer` interface:

```go
type CustomMailer struct {
    // your fields
}

func (m *CustomMailer) Send(ctx context.Context, email *Email) error {
    // Implementation
}

func (m *CustomMailer) SendBatch(ctx context.Context, emails []*Email) error {
    // Implementation
}

func (m *CustomMailer) SendTemplate(ctx context.Context, template string, data TemplateData, email *Email) error {
    // Implementation
}

func (m *CustomMailer) Validate(email *Email) error {
    return email.Validate()
}
```

## Error Handling

The package provides specific error types for different scenarios:

```go
err := mailService.Send(ctx, email)
if err != nil {
    switch err {
    case mailer.ErrInvalidEmail:
        // Handle invalid email
    case mailer.ErrConnectionFailed:
        // Handle connection failure
    case mailer.ErrAuthFailed:
        // Handle authentication failure
    case mailer.ErrRateLimitExceeded:
        // Handle rate limit
    default:
        // Handle other errors
    }
}
```

## Testing

Use the mock mailer for testing:

```go
func TestEmailSending(t *testing.T) {
    mockMailer := mailer.NewMock()
    
    // Your code that sends emails
    sendWelcomeEmail(mockMailer, "user@example.com")
    
    // Verify email was sent
    emails := mockMailer.GetSentEmails()
    if len(emails) != 1 {
        t.Errorf("Expected 1 email, got %d", len(emails))
    }
    
    // Check email content
    email := emails[0]
    if email.Subject != "Welcome" {
        t.Errorf("Expected subject 'Welcome', got %s", email.Subject)
    }
}
```

## Best Practices

1. **Use connection pooling**: For SMTP, the package automatically manages a connection pool
2. **Set appropriate timeouts**: Configure timeouts based on your needs
3. **Use templates**: Keep email content separate from code
4. **Handle errors**: Always check and handle errors appropriately
5. **Rate limiting**: Set rate limits to avoid overwhelming mail servers
6. **Testing**: Use mock mailer for unit tests
7. **Validation**: Always validate emails before sending

## License

MIT