// Package dashboard serves the embedded SPA dashboard.
package dashboard

import (
	"embed"
	"io/fs"
	"net/http"
	"strings"
)

//go:embed static/*
var staticFiles embed.FS

// Handler returns an HTTP handler that serves the embedded SPA.
// It serves static files from the embedded FS and falls back to index.html
// for client-side routing.
func Handler() http.Handler {
	staticFS, err := fs.Sub(staticFiles, "static")
	if err != nil {
		// Fallback: serve a minimal page
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			w.Header().Set("Content-Type", "text/html")
			w.Write([]byte(fallbackHTML))
		})
	}

	fileServer := http.FileServer(http.FS(staticFS))

	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Try to serve the static file first
		path := r.URL.Path
		if path == "/" {
			path = "/index.html"
		}

		// Check if file exists
		cleanPath := strings.TrimPrefix(path, "/")
		if _, err := fs.Stat(staticFS, cleanPath); err == nil {
			fileServer.ServeHTTP(w, r)
			return
		}

		// SPA fallback: serve index.html for client-side routing
		r.URL.Path = "/"
		fileServer.ServeHTTP(w, r)
	})
}

const fallbackHTML = `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Solobase Cloud</title>
  <style>
    body { font-family: -apple-system, BlinkMacSystemFont, sans-serif; max-width: 800px; margin: 80px auto; padding: 0 20px; }
    h1 { color: #1a1a1a; }
    .status { padding: 20px; background: #f0f9ff; border-radius: 8px; border: 1px solid #bae6fd; }
  </style>
</head>
<body>
  <h1>Solobase Cloud</h1>
  <div class="status">
    <p>Dashboard is loading. If this persists, the static assets may not be embedded.</p>
    <p>API is available at <code>/api/</code></p>
  </div>
</body>
</html>`
