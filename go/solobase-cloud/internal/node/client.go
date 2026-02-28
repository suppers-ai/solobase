// Package node provides an HTTP client for communicating with solobase-node
// instances (Firecracker orchestrator nodes).
package node

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

// Client communicates with a solobase-node management API.
type Client struct {
	baseURL    string
	secret     string
	httpClient *http.Client
}

// NewClient creates a client for a solobase-node at the given base URL.
func NewClient(baseURL, secret string) *Client {
	return &Client{
		baseURL: baseURL,
		secret:  secret,
		httpClient: &http.Client{
			Timeout: 60 * time.Second,
		},
	}
}

// NodeHealth represents the health/capacity response from a node.
type NodeHealth struct {
	Status    string `json:"status"`
	MaxVMs    int    `json:"max_vms"`
	Running   int    `json:"running"`
	Paused    int    `json:"paused"`
	Total     int    `json:"total"`
	FreeSlots int    `json:"free_slots"`
}

// CreateTenantRequest is sent to provision a new tenant VM.
type CreateTenantRequest struct {
	TenantID  string            `json:"tenant_id"`
	Subdomain string            `json:"subdomain"`
	VCPUs     int               `json:"vcpus,omitempty"`
	MemMB     int               `json:"mem_mb,omitempty"`
	Config    map[string]string `json:"config,omitempty"`
}

// TenantStatus represents the status response for a tenant.
type TenantStatus struct {
	ID           string    `json:"id"`
	Subdomain    string    `json:"subdomain"`
	State        string    `json:"state"`
	VMIP         string    `json:"vm_ip"`
	CreatedAt    time.Time `json:"created_at"`
	LastActivity time.Time `json:"last_activity"`
}

// Health fetches the node's health/capacity info.
func (c *Client) Health(ctx context.Context) (*NodeHealth, error) {
	var result NodeHealth
	if err := c.doJSON(ctx, "GET", "/api/health", nil, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// CreateTenant provisions a new tenant VM on this node.
func (c *Client) CreateTenant(ctx context.Context, req *CreateTenantRequest) (*TenantStatus, error) {
	var result TenantStatus
	if err := c.doJSON(ctx, "POST", "/api/tenants", req, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// DestroyTenant removes a tenant VM from this node.
func (c *Client) DestroyTenant(ctx context.Context, tenantID string) error {
	return c.doJSON(ctx, "DELETE", fmt.Sprintf("/api/tenants/%s", tenantID), nil, nil)
}

// PauseTenant pauses a tenant VM.
func (c *Client) PauseTenant(ctx context.Context, tenantID string) error {
	return c.doJSON(ctx, "POST", fmt.Sprintf("/api/tenants/%s/pause", tenantID), nil, nil)
}

// ResumeTenant resumes a paused tenant VM.
func (c *Client) ResumeTenant(ctx context.Context, tenantID string) error {
	return c.doJSON(ctx, "POST", fmt.Sprintf("/api/tenants/%s/resume", tenantID), nil, nil)
}

// TenantInfo fetches the status of a specific tenant.
func (c *Client) TenantInfo(ctx context.Context, tenantID string) (*TenantStatus, error) {
	var result TenantStatus
	if err := c.doJSON(ctx, "GET", fmt.Sprintf("/api/tenants/%s/status", tenantID), nil, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *Client) doJSON(ctx context.Context, method, path string, body, result interface{}) error {
	var bodyReader io.Reader
	if body != nil {
		data, err := json.Marshal(body)
		if err != nil {
			return fmt.Errorf("marshal request: %w", err)
		}
		bodyReader = bytes.NewReader(data)
	}

	req, err := http.NewRequestWithContext(ctx, method, c.baseURL+path, bodyReader)
	if err != nil {
		return fmt.Errorf("create request: %w", err)
	}
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}
	if c.secret != "" {
		req.Header.Set("Authorization", "Bearer "+c.secret)
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("do request: %w", err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("read response: %w", err)
	}

	if resp.StatusCode >= 400 {
		return fmt.Errorf("node API error (status %d): %s", resp.StatusCode, string(respBody))
	}

	if result != nil && len(respBody) > 0 {
		if err := json.Unmarshal(respBody, result); err != nil {
			return fmt.Errorf("unmarshal response: %w", err)
		}
	}

	return nil
}
