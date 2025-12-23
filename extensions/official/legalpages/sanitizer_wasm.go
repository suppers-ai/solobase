//go:build wasm

package legalpages

import (
	"database/sql"
	"strings"

	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
)

type htmlSanitizer interface {
	Sanitize(s string) string
}

// simpleSanitizer provides basic HTML sanitization for WASM builds
// It uses simple string operations instead of regexp
type simpleSanitizer struct {
	allowedTags map[string]bool
}

func newSimpleSanitizer() *simpleSanitizer {
	return &simpleSanitizer{
		allowedTags: map[string]bool{
			"p": true, "br": true, "strong": true, "em": true,
			"ul": true, "ol": true, "li": true,
			"h1": true, "h2": true, "h3": true, "h4": true, "h5": true, "h6": true,
			"a": true,
		},
	}
}

// Sanitize removes potentially dangerous HTML while preserving allowed tags
func (s *simpleSanitizer) Sanitize(html string) string {
	var result strings.Builder
	i := 0
	n := len(html)

	for i < n {
		if html[i] == '<' {
			// Find the end of the tag
			tagEnd := strings.Index(html[i:], ">")
			if tagEnd == -1 {
				// No closing bracket, escape and continue
				result.WriteString("&lt;")
				i++
				continue
			}
			tagEnd += i

			tagContent := html[i+1 : tagEnd]
			isClosing := false
			if len(tagContent) > 0 && tagContent[0] == '/' {
				isClosing = true
				tagContent = tagContent[1:]
			}

			// Extract tag name (up to first space or end)
			tagName := tagContent
			spaceIdx := strings.IndexAny(tagContent, " \t\n\r")
			if spaceIdx != -1 {
				tagName = tagContent[:spaceIdx]
			}
			tagName = strings.ToLower(tagName)

			// Check if tag is allowed
			if s.allowedTags[tagName] {
				// For 'a' tags, only allow href attribute
				if tagName == "a" && !isClosing {
					result.WriteString("<a")
					// Try to extract href
					hrefStart := strings.Index(strings.ToLower(tagContent), "href=")
					if hrefStart != -1 {
						hrefVal := tagContent[hrefStart+5:]
						quote := byte('"')
						if len(hrefVal) > 0 && hrefVal[0] == '\'' {
							quote = '\''
						}
						if len(hrefVal) > 0 && (hrefVal[0] == '"' || hrefVal[0] == '\'') {
							hrefVal = hrefVal[1:]
							endQuote := strings.IndexByte(hrefVal, quote)
							if endQuote != -1 {
								href := hrefVal[:endQuote]
								// Only allow http/https URLs
								if strings.HasPrefix(href, "http://") || strings.HasPrefix(href, "https://") {
									result.WriteString(" href=\"")
									result.WriteString(escapeAttr(href))
									result.WriteString("\"")
								}
							}
						}
					}
					result.WriteString(">")
				} else if isClosing {
					result.WriteString("</")
					result.WriteString(tagName)
					result.WriteString(">")
				} else {
					result.WriteString("<")
					result.WriteString(tagName)
					result.WriteString(">")
				}
			}
			// Skip disallowed tags entirely

			i = tagEnd + 1
		} else {
			result.WriteByte(html[i])
			i++
		}
	}

	return result.String()
}

func escapeAttr(s string) string {
	s = strings.ReplaceAll(s, "&", "&amp;")
	s = strings.ReplaceAll(s, "\"", "&quot;")
	s = strings.ReplaceAll(s, "<", "&lt;")
	s = strings.ReplaceAll(s, ">", "&gt;")
	return s
}

func NewLegalPagesService(sqlDB *sql.DB) *LegalPagesService {
	return &LegalPagesService{
		queries:   db.New(sqlDB),
		sqlDB:     sqlDB,
		sanitizer: newSimpleSanitizer(),
	}
}
