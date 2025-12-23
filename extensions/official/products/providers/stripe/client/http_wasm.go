//go:build wasm

package client

import (
	"fmt"
)

// wasmHTTPClient implements HTTPClient for WASM builds
// Currently outgoing HTTP is not supported in WASM - the host would need to provide this capability
type wasmHTTPClient struct{}

// newHTTPClient creates a new HTTP client for WASM builds
func newHTTPClient() HTTPClient {
	return &wasmHTTPClient{}
}

// Do makes an HTTP request
// Currently returns an error as outgoing HTTP is not yet supported in WASM
func (c *wasmHTTPClient) Do(method, url string, body []byte, headers map[string]string) (*Response, error) {
	// Outgoing HTTP requests would need to be added to the WIT interface
	// and implemented by the host runtime
	return nil, fmt.Errorf("outgoing HTTP not supported in WASM: would need to add to WIT interface")
}
