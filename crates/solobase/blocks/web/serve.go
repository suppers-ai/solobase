package web

import (
	"mime"
	"net/http"
	"os"
	"path/filepath"
	"strings"

	wafer "github.com/wafer-run/wafer-go"
)

// serveFile resolves the request path to a file and serves it.
func (b *WebBlock) serveFile(msg *wafer.Message, reqPath string) wafer.Result {
	// Default empty or root path to index file
	if reqPath == "" || reqPath == "/" {
		reqPath = "/" + b.config.IndexFile
	}

	// Clean the path to prevent directory traversal
	cleaned := filepath.Clean(reqPath)

	// Block dotfiles (path segments starting with ".")
	for _, seg := range strings.Split(cleaned, string(filepath.Separator)) {
		if seg != "" && seg[0] == '.' {
			return b.handleNotFound(msg, reqPath)
		}
	}

	// Join with root and resolve symlinks
	fullPath := filepath.Join(b.absRoot, cleaned)
	resolved, err := filepath.EvalSymlinks(fullPath)
	if err != nil {
		return b.handleNotFound(msg, reqPath)
	}

	// Verify resolved path is still within absRoot (prevent symlink escape)
	if !strings.HasPrefix(resolved, b.absRoot) {
		return b.handleNotFound(msg, reqPath)
	}

	// If path is a directory, append index file
	info, err := os.Stat(resolved)
	if err != nil {
		return b.handleNotFound(msg, reqPath)
	}
	if info.IsDir() {
		resolved = filepath.Join(resolved, b.config.IndexFile)
		info, err = os.Stat(resolved)
		if err != nil {
			return b.handleNotFound(msg, reqPath)
		}
	}

	// Read the file
	data, err := os.ReadFile(resolved)
	if err != nil {
		return b.handleNotFound(msg, reqPath)
	}

	contentType := detectContentType(resolved, data)
	cacheHeader := b.cacheControl(reqPath, contentType)

	return wafer.NewResponse(msg, 200).
		SetHeader("Cache-Control", cacheHeader).
		Body(data, contentType)
}

// handleNotFound returns the SPA fallback or a 404.
func (b *WebBlock) handleNotFound(msg *wafer.Message, _ string) wafer.Result {
	if b.config.SPAMode {
		indexPath := filepath.Join(b.absRoot, b.config.IndexFile)
		data, err := os.ReadFile(indexPath)
		if err != nil {
			return wafer.ErrNotFound(msg, "not found")
		}
		return wafer.NewResponse(msg, 200).
			SetHeader("Cache-Control", "no-cache").
			Body(data, "text/html; charset=utf-8")
	}
	return wafer.ErrNotFound(msg, "not found")
}

// detectContentType determines the MIME type of a file.
func detectContentType(filePath string, data []byte) string {
	ext := filepath.Ext(filePath)
	ct := mime.TypeByExtension(ext)
	if ct != "" {
		// Add charset for text types if not already present
		if strings.HasPrefix(ct, "text/") && !strings.Contains(ct, "charset") {
			ct += "; charset=utf-8"
		}
		return ct
	}
	// Fall back to content sniffing
	return http.DetectContentType(data)
}
