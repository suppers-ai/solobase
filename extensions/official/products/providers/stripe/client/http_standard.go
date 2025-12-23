//go:build !wasm

package client

import (
	"bytes"
	"io"
	"net/http"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

// standardHTTPClient implements HTTPClient using net/http
type standardHTTPClient struct {
	client *http.Client
}

// newHTTPClient creates a new HTTP client for standard builds
func newHTTPClient() HTTPClient {
	return &standardHTTPClient{
		client: &http.Client{
			Timeout: 30 * apptime.Second,
		},
	}
}

// Do makes an HTTP request
func (c *standardHTTPClient) Do(method, url string, body []byte, headers map[string]string) (*Response, error) {
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
