package mailer

import (
	"bytes"
	"fmt"
	"html/template"
	"io/ioutil"
	"path/filepath"
	"strings"
	textTemplate "text/template"
)

// Template handles email template rendering
type Template struct {
	basePath      string
	htmlTemplates map[string]*template.Template
	textTemplates map[string]*textTemplate.Template
}

// NewTemplate creates a new template renderer
func NewTemplate(basePath string) *Template {
	return &Template{
		basePath:      basePath,
		htmlTemplates: make(map[string]*template.Template),
		textTemplates: make(map[string]*textTemplate.Template),
	}
}

// RenderHTML renders an HTML template
func (t *Template) RenderHTML(name string, data TemplateData) (string, error) {
	// Check cache
	if tmpl, ok := t.htmlTemplates[name]; ok {
		return t.executeHTMLTemplate(tmpl, data)
	}

	// Load template
	tmpl, err := t.loadHTMLTemplate(name)
	if err != nil {
		return "", err
	}

	// Cache template
	t.htmlTemplates[name] = tmpl

	return t.executeHTMLTemplate(tmpl, data)
}

// RenderText renders a text template
func (t *Template) RenderText(name string, data TemplateData) (string, error) {
	// Check cache
	if tmpl, ok := t.textTemplates[name]; ok {
		return t.executeTextTemplate(tmpl, data)
	}

	// Load template
	tmpl, err := t.loadTextTemplate(name)
	if err != nil {
		return "", err
	}

	// Cache template
	t.textTemplates[name] = tmpl

	return t.executeTextTemplate(tmpl, data)
}

// loadHTMLTemplate loads an HTML template from file
func (t *Template) loadHTMLTemplate(name string) (*template.Template, error) {
	// Try different extensions
	extensions := []string{".html", ".htm", ".html.tmpl", ".html.tpl"}

	for _, ext := range extensions {
		path := filepath.Join(t.basePath, name+ext)
		content, err := ioutil.ReadFile(path)
		if err != nil {
			continue
		}

		// Parse template with custom functions
		tmpl, err := template.New(name).Funcs(t.htmlFuncMap()).Parse(string(content))
		if err != nil {
			return nil, fmt.Errorf("%w: %v", ErrTemplateParseError, err)
		}

		return tmpl, nil
	}

	// Try inline template
	if strings.Contains(name, "{{") {
		tmpl, err := template.New("inline").Funcs(t.htmlFuncMap()).Parse(name)
		if err != nil {
			return nil, fmt.Errorf("%w: %v", ErrTemplateParseError, err)
		}
		return tmpl, nil
	}

	return nil, ErrTemplateNotFound
}

// loadTextTemplate loads a text template from file
func (t *Template) loadTextTemplate(name string) (*textTemplate.Template, error) {
	// Try different extensions
	extensions := []string{".txt", ".text", ".txt.tmpl", ".txt.tpl"}

	for _, ext := range extensions {
		path := filepath.Join(t.basePath, name+ext)
		content, err := ioutil.ReadFile(path)
		if err != nil {
			continue
		}

		// Parse template with custom functions
		tmpl, err := textTemplate.New(name).Funcs(t.textFuncMap()).Parse(string(content))
		if err != nil {
			return nil, fmt.Errorf("%w: %v", ErrTemplateParseError, err)
		}

		return tmpl, nil
	}

	// Try inline template
	if strings.Contains(name, "{{") {
		tmpl, err := textTemplate.New("inline").Funcs(t.textFuncMap()).Parse(name)
		if err != nil {
			return nil, fmt.Errorf("%w: %v", ErrTemplateParseError, err)
		}
		return tmpl, nil
	}

	return nil, ErrTemplateNotFound
}

// executeHTMLTemplate executes an HTML template
func (t *Template) executeHTMLTemplate(tmpl *template.Template, data TemplateData) (string, error) {
	var buf bytes.Buffer
	if err := tmpl.Execute(&buf, data); err != nil {
		return "", fmt.Errorf("%w: %v", ErrTemplateExecError, err)
	}
	return buf.String(), nil
}

// executeTextTemplate executes a text template
func (t *Template) executeTextTemplate(tmpl *textTemplate.Template, data TemplateData) (string, error) {
	var buf bytes.Buffer
	if err := tmpl.Execute(&buf, data); err != nil {
		return "", fmt.Errorf("%w: %v", ErrTemplateExecError, err)
	}
	return buf.String(), nil
}

// htmlFuncMap returns custom functions for HTML templates
func (t *Template) htmlFuncMap() template.FuncMap {
	return template.FuncMap{
		"upper": strings.ToUpper,
		"lower": strings.ToLower,
		"title": strings.Title,
		"trim":  strings.TrimSpace,
		"default": func(def, val interface{}) interface{} {
			if val == nil || val == "" {
				return def
			}
			return val
		},
		"truncate": func(n int, s string) string {
			if len(s) <= n {
				return s
			}
			return s[:n] + "..."
		},
		"safe": func(s string) template.HTML {
			return template.HTML(s)
		},
		"url": func(s string) template.URL {
			return template.URL(s)
		},
		"js": func(s string) template.JS {
			return template.JS(s)
		},
		"css": func(s string) template.CSS {
			return template.CSS(s)
		},
	}
}

// textFuncMap returns custom functions for text templates
func (t *Template) textFuncMap() textTemplate.FuncMap {
	return textTemplate.FuncMap{
		"upper": strings.ToUpper,
		"lower": strings.ToLower,
		"title": strings.Title,
		"trim":  strings.TrimSpace,
		"default": func(def, val interface{}) interface{} {
			if val == nil || val == "" {
				return def
			}
			return val
		},
		"truncate": func(n int, s string) string {
			if len(s) <= n {
				return s
			}
			return s[:n] + "..."
		},
	}
}

// PrebuiltTemplates provides common email templates
type PrebuiltTemplates struct{}

// Welcome returns a welcome email template
func (p PrebuiltTemplates) Welcome() string {
	return `
<!DOCTYPE html>
<html>
<head>
    <style>
        body { font-family: Arial, sans-serif; line-height: 1.6; color: #333; }
        .container { max-width: 600px; margin: 0 auto; padding: 20px; }
        .header { background-color: #f4f4f4; padding: 20px; text-align: center; }
        .content { padding: 20px; }
        .button { display: inline-block; padding: 10px 20px; background-color: #007bff; color: white; text-decoration: none; border-radius: 5px; }
        .footer { background-color: #f4f4f4; padding: 10px; text-align: center; font-size: 12px; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Welcome to {{.AppName}}</h1>
        </div>
        <div class="content">
            <p>Hi {{.Name}},</p>
            <p>Thank you for signing up! We're excited to have you on board.</p>
            {{if .ConfirmURL}}
            <p>Please confirm your email address by clicking the button below:</p>
            <p style="text-align: center;">
                <a href="{{.ConfirmURL}}" class="button">Confirm Email</a>
            </p>
            {{end}}
            <p>If you have any questions, feel free to reach out to our support team.</p>
            <p>Best regards,<br>The {{.AppName}} Team</p>
        </div>
        <div class="footer">
            <p>&copy; {{.Year}} {{.AppName}}. All rights reserved.</p>
        </div>
    </div>
</body>
</html>
`
}

// PasswordReset returns a password reset email template
func (p PrebuiltTemplates) PasswordReset() string {
	return `
<!DOCTYPE html>
<html>
<head>
    <style>
        body { font-family: Arial, sans-serif; line-height: 1.6; color: #333; }
        .container { max-width: 600px; margin: 0 auto; padding: 20px; }
        .header { background-color: #f4f4f4; padding: 20px; text-align: center; }
        .content { padding: 20px; }
        .button { display: inline-block; padding: 10px 20px; background-color: #dc3545; color: white; text-decoration: none; border-radius: 5px; }
        .footer { background-color: #f4f4f4; padding: 10px; text-align: center; font-size: 12px; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Password Reset Request</h1>
        </div>
        <div class="content">
            <p>Hi {{.Name}},</p>
            <p>We received a request to reset your password. Click the button below to create a new password:</p>
            <p style="text-align: center;">
                <a href="{{.ResetURL}}" class="button">Reset Password</a>
            </p>
            <p>This link will expire in {{.ExpireHours}} hours.</p>
            <p>If you didn't request this, you can safely ignore this email.</p>
            <p>Best regards,<br>The {{.AppName}} Team</p>
        </div>
        <div class="footer">
            <p>&copy; {{.Year}} {{.AppName}}. All rights reserved.</p>
        </div>
    </div>
</body>
</html>
`
}

// Notification returns a notification email template
func (p PrebuiltTemplates) Notification() string {
	return `
<!DOCTYPE html>
<html>
<head>
    <style>
        body { font-family: Arial, sans-serif; line-height: 1.6; color: #333; }
        .container { max-width: 600px; margin: 0 auto; padding: 20px; }
        .header { background-color: #f4f4f4; padding: 20px; text-align: center; }
        .content { padding: 20px; }
        .alert { padding: 15px; margin: 10px 0; border-radius: 5px; }
        .alert-info { background-color: #d1ecf1; color: #0c5460; }
        .alert-warning { background-color: #fff3cd; color: #856404; }
        .alert-danger { background-color: #f8d7da; color: #721c24; }
        .footer { background-color: #f4f4f4; padding: 10px; text-align: center; font-size: 12px; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>{{.Title}}</h1>
        </div>
        <div class="content">
            {{if .AlertType}}
            <div class="alert alert-{{.AlertType}}">
                {{.AlertMessage}}
            </div>
            {{end}}
            <p>{{.Message | safe}}</p>
            {{if .ActionURL}}
            <p style="text-align: center;">
                <a href="{{.ActionURL}}" class="button">{{.ActionText | default "Take Action"}}</a>
            </p>
            {{end}}
        </div>
        <div class="footer">
            <p>&copy; {{.Year}} {{.AppName}}. All rights reserved.</p>
        </div>
    </div>
</body>
</html>
`
}
