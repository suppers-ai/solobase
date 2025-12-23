//go:build !wasm

package legalpages

import (
	"database/sql"

	"github.com/microcosm-cc/bluemonday"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
)

type htmlSanitizer interface {
	Sanitize(s string) string
}

func NewLegalPagesService(sqlDB *sql.DB) *LegalPagesService {
	// Create HTML sanitizer with allowed tags
	p := bluemonday.UGCPolicy()
	p.AllowElements("p", "br", "strong", "em", "ul", "ol", "li", "h1", "h2", "h3", "h4", "h5", "h6")
	p.AllowAttrs("href").OnElements("a")

	return &LegalPagesService{
		queries:   db.New(sqlDB),
		sqlDB:     sqlDB,
		sanitizer: p,
	}
}
