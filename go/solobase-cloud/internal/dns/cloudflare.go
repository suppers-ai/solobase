// Package dns manages DNS records via the Cloudflare API.
package dns

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

// CloudflareClient manages DNS records for tenant subdomains.
type CloudflareClient struct {
	apiToken string
	zoneID   string
	domain   string // e.g. "solobase.app"
	client   *http.Client
}

// NewCloudflareClient creates a DNS client.
func NewCloudflareClient(apiToken, zoneID, domain string) *CloudflareClient {
	return &CloudflareClient{
		apiToken: apiToken,
		zoneID:   zoneID,
		domain:   domain,
		client:   &http.Client{Timeout: 30 * time.Second},
	}
}

// DNSRecord represents a Cloudflare DNS record.
type DNSRecord struct {
	ID      string `json:"id"`
	Type    string `json:"type"`
	Name    string `json:"name"`
	Content string `json:"content"`
	TTL     int    `json:"ttl"`
	Proxied bool   `json:"proxied"`
}

type cfResponse struct {
	Success bool        `json:"success"`
	Errors  []cfError   `json:"errors"`
	Result  interface{} `json:"result"`
}

type cfError struct {
	Code    int    `json:"code"`
	Message string `json:"message"`
}

type cfListResponse struct {
	Success bool        `json:"success"`
	Errors  []cfError   `json:"errors"`
	Result  []DNSRecord `json:"result"`
}

// CreateRecord creates an A record for a subdomain pointing to nodeIP.
func (c *CloudflareClient) CreateRecord(ctx context.Context, subdomain, nodeIP string) (*DNSRecord, error) {
	body := map[string]interface{}{
		"type":    "A",
		"name":    fmt.Sprintf("%s.%s", subdomain, c.domain),
		"content": nodeIP,
		"ttl":     60,
		"proxied": false,
	}

	data, err := json.Marshal(body)
	if err != nil {
		return nil, fmt.Errorf("marshal request: %w", err)
	}

	url := fmt.Sprintf("https://api.cloudflare.com/client/v4/zones/%s/dns_records", c.zoneID)
	req, err := http.NewRequestWithContext(ctx, "POST", url, bytes.NewReader(data))
	if err != nil {
		return nil, fmt.Errorf("create request: %w", err)
	}
	c.setHeaders(req)

	resp, err := c.client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("do request: %w", err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("read response: %w", err)
	}

	var result struct {
		Success bool      `json:"success"`
		Errors  []cfError `json:"errors"`
		Result  DNSRecord `json:"result"`
	}
	if err := json.Unmarshal(respBody, &result); err != nil {
		return nil, fmt.Errorf("unmarshal response: %w", err)
	}
	if !result.Success {
		return nil, fmt.Errorf("cloudflare error: %v", result.Errors)
	}
	return &result.Result, nil
}

// DeleteRecord deletes a DNS record by ID.
func (c *CloudflareClient) DeleteRecord(ctx context.Context, recordID string) error {
	url := fmt.Sprintf("https://api.cloudflare.com/client/v4/zones/%s/dns_records/%s", c.zoneID, recordID)
	req, err := http.NewRequestWithContext(ctx, "DELETE", url, nil)
	if err != nil {
		return fmt.Errorf("create request: %w", err)
	}
	c.setHeaders(req)

	resp, err := c.client.Do(req)
	if err != nil {
		return fmt.Errorf("do request: %w", err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("read response: %w", err)
	}

	var result cfResponse
	if err := json.Unmarshal(respBody, &result); err != nil {
		return fmt.Errorf("unmarshal response: %w", err)
	}
	if !result.Success {
		return fmt.Errorf("cloudflare error: %v", result.Errors)
	}
	return nil
}

// UpdateRecord updates an existing DNS record's IP.
func (c *CloudflareClient) UpdateRecord(ctx context.Context, recordID, nodeIP string) error {
	body := map[string]interface{}{
		"type":    "A",
		"content": nodeIP,
		"ttl":     60,
		"proxied": false,
	}

	data, err := json.Marshal(body)
	if err != nil {
		return fmt.Errorf("marshal request: %w", err)
	}

	url := fmt.Sprintf("https://api.cloudflare.com/client/v4/zones/%s/dns_records/%s", c.zoneID, recordID)
	req, err := http.NewRequestWithContext(ctx, "PATCH", url, bytes.NewReader(data))
	if err != nil {
		return fmt.Errorf("create request: %w", err)
	}
	c.setHeaders(req)

	resp, err := c.client.Do(req)
	if err != nil {
		return fmt.Errorf("do request: %w", err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("read response: %w", err)
	}

	var result cfResponse
	if err := json.Unmarshal(respBody, &result); err != nil {
		return fmt.Errorf("unmarshal response: %w", err)
	}
	if !result.Success {
		return fmt.Errorf("cloudflare error: %v", result.Errors)
	}
	return nil
}

// FindRecord looks up a DNS record by subdomain name.
func (c *CloudflareClient) FindRecord(ctx context.Context, subdomain string) (*DNSRecord, error) {
	name := fmt.Sprintf("%s.%s", subdomain, c.domain)
	url := fmt.Sprintf("https://api.cloudflare.com/client/v4/zones/%s/dns_records?type=A&name=%s", c.zoneID, name)

	req, err := http.NewRequestWithContext(ctx, "GET", url, nil)
	if err != nil {
		return nil, fmt.Errorf("create request: %w", err)
	}
	c.setHeaders(req)

	resp, err := c.client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("do request: %w", err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("read response: %w", err)
	}

	var result cfListResponse
	if err := json.Unmarshal(respBody, &result); err != nil {
		return nil, fmt.Errorf("unmarshal response: %w", err)
	}
	if !result.Success {
		return nil, fmt.Errorf("cloudflare error: %v", result.Errors)
	}
	if len(result.Result) == 0 {
		return nil, nil
	}
	return &result.Result[0], nil
}

func (c *CloudflareClient) setHeaders(req *http.Request) {
	req.Header.Set("Authorization", "Bearer "+c.apiToken)
	req.Header.Set("Content-Type", "application/json")
}
