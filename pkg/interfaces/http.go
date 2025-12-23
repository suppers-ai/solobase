package interfaces

import (
	"context"
	"net/http"
)

// HTTPServer defines how to start/stop the HTTP server.
// Implementations:
//   - Standard: net/http server with graceful shutdown
//   - WASM: No-op (Spin handles the server)
type HTTPServer interface {
	// ListenAndServe starts the HTTP server
	ListenAndServe(addr string, handler http.Handler) error

	// Shutdown gracefully shuts down the server
	Shutdown(ctx context.Context) error
}

// HTTPClient defines outbound HTTP operations.
// Implementations:
//   - Standard: net/http client
//   - WASM: Spin outbound HTTP
type HTTPClient interface {
	// Do executes an HTTP request
	Do(req *http.Request) (*http.Response, error)

	// Get performs a GET request
	Get(ctx context.Context, url string) (*http.Response, error)

	// Post performs a POST request
	Post(ctx context.Context, url, contentType string, body []byte) (*http.Response, error)
}

// HTTPServerConfig contains HTTP server configuration
type HTTPServerConfig struct {
	Address         string
	ReadTimeout     int // in seconds
	WriteTimeout    int // in seconds
	IdleTimeout     int // in seconds
	MaxHeaderBytes  int
	TLSCertFile     string
	TLSKeyFile      string
	EnableTLS       bool
	EnableHTTP2     bool
	TrustedProxies  []string
	AllowedOrigins  []string
	AllowedMethods  []string
	AllowedHeaders  []string
	ExposeHeaders   []string
	AllowCredentials bool
	MaxAge          int // CORS max age in seconds
}

// HTTPClientConfig contains HTTP client configuration
type HTTPClientConfig struct {
	Timeout             int // in seconds
	MaxIdleConns        int
	MaxIdleConnsPerHost int
	MaxConnsPerHost     int
	DisableKeepAlives   bool
	DisableCompression  bool

	// For WASM/Spin: allowed hosts for outbound requests
	AllowedHosts []string
}
