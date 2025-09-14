package middleware

import (
	"net/http"

	"github.com/suppers-ai/solobase/internal/pkg/logger"
)

// Logger middleware
func Logger(log logger.Logger) func(http.Handler) http.Handler {
	return logger.HTTPMiddleware(log, &logger.MiddlewareConfig{
		SkipPaths: []string{
			"/health",
			"/static/",
			"/favicon.ico",
		},
		LogHeaders:      true,
		LogRequestBody:  false, // Don't log request bodies by default (may contain passwords)
		LogResponseBody: false,
		MaxBodySize:     4096,
	})
}
