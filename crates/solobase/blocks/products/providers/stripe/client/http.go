package client

import (
	"bytes"
	"fmt"
	"io"
	"net/http"

	"github.com/suppers-ai/solobase/core/apptime"
)

// HTTPClientFactory creates HTTP clients.
type HTTPClientFactory interface {
	New() HTTPClient
}

var defaultHTTPClientFactory HTTPClientFactory

// SetHTTPClientFactory sets the default HTTP client factory.
func SetHTTPClientFactory(f HTTPClientFactory) {
	defaultHTTPClientFactory = f
}

// GetHTTPClientFactory returns the current HTTP client factory.
// Returns a no-op factory if none is set.
func GetHTTPClientFactory() HTTPClientFactory {
	if defaultHTTPClientFactory == nil {
		return &NoOpHTTPClientFactory{}
	}
	return defaultHTTPClientFactory
}

// newHTTPClient creates a new HTTP client using the factory.
func newHTTPClient() HTTPClient {
	return GetHTTPClientFactory().New()
}

// StandardHTTPClient implements HTTPClient using net/http.
type StandardHTTPClient struct {
	client *http.Client
}

// NewStandardHTTPClient creates a new standard HTTP client.
func NewStandardHTTPClient() *StandardHTTPClient {
	return &StandardHTTPClient{
		client: &http.Client{
			Timeout: 30 * apptime.Second,
		},
	}
}

// Do makes an HTTP request.
func (c *StandardHTTPClient) Do(method, url string, body []byte, headers map[string]string) (*Response, error) {
	var bodyReader io.Reader
	if body != nil {
		bodyReader = bytes.NewReader(body)
	}

	req, err := http.NewRequest(method, url, bodyReader)
	if err != nil {
		return nil, err
	}

	for k, v := range headers {
		req.Header.Set(k, v)
	}

	resp, err := c.client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}

	return &Response{
		StatusCode: resp.StatusCode,
		Body:       respBody,
	}, nil
}

// StandardHTTPClientFactory creates standard HTTP clients.
type StandardHTTPClientFactory struct{}

// NewStandardHTTPClientFactory creates a new standard HTTP client factory.
func NewStandardHTTPClientFactory() *StandardHTTPClientFactory {
	return &StandardHTTPClientFactory{}
}

// New creates a new standard HTTP client.
func (f *StandardHTTPClientFactory) New() HTTPClient {
	return NewStandardHTTPClient()
}

// NoOpHTTPClient returns errors for all requests.
// Used in WASM builds where outgoing HTTP is not available.
type NoOpHTTPClient struct{}

// NewNoOpHTTPClient creates a new no-op HTTP client.
func NewNoOpHTTPClient() *NoOpHTTPClient {
	return &NoOpHTTPClient{}
}

// Do returns an error (HTTP not available).
func (c *NoOpHTTPClient) Do(method, url string, body []byte, headers map[string]string) (*Response, error) {
	return nil, fmt.Errorf("outgoing HTTP not supported: host must provide HTTP capability")
}

// NoOpHTTPClientFactory creates no-op HTTP clients.
type NoOpHTTPClientFactory struct{}

// NewNoOpHTTPClientFactory creates a new no-op HTTP client factory.
func NewNoOpHTTPClientFactory() *NoOpHTTPClientFactory {
	return &NoOpHTTPClientFactory{}
}

// New creates a new no-op HTTP client.
func (f *NoOpHTTPClientFactory) New() HTTPClient {
	return NewNoOpHTTPClient()
}
