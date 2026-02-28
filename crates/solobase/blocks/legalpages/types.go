package legalpages

import "fmt"

const (
	DocumentTypeTerms   = "terms"
	DocumentTypePrivacy = "privacy"
)

// LegalPagesConfig holds block-specific configuration.
type LegalPagesConfig struct {
	EnableTerms   bool   `json:"enableTerms"`
	EnablePrivacy bool   `json:"enablePrivacy"`
	CompanyName   string `json:"companyName"`
}

// renderPublicPageHTML generates the HTML for a public legal page.
func renderPublicPageHTML(title, content, message string) string {
	contentSection := ""
	if content != "" {
		contentSection = content
	} else {
		contentSection = fmt.Sprintf(`<div class="not-found"><p>%s</p></div>`, message)
	}

	return fmt.Sprintf(`<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>%s</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
        }
        h1 { color: #2c3e50; }
        a { color: #3498db; text-decoration: none; }
        a:hover { text-decoration: underline; }
        .back-link { margin-bottom: 20px; }
        .content { margin-top: 30px; }
        .not-found {
            text-align: center;
            padding: 50px 20px;
            background: #f8f9fa;
            border-radius: 8px;
        }
    </style>
</head>
<body>
    <div class="back-link">
        <a href="/">← Back to Home</a>
    </div>
    <h1>%s</h1>
    <div class="content">
        %s
    </div>
</body>
</html>`, title, title, contentSection)
}
