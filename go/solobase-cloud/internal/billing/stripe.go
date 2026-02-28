// Package billing manages Stripe subscriptions and usage metering.
package billing

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"
)

// Plan describes a billing tier.
type Plan struct {
	ID              string   `json:"id"`
	Name            string   `json:"name"`
	PriceID         string   `json:"price_id"`
	PriceCents       int      `json:"price_cents"`
	MaxVMs           int      `json:"max_vms"`
	VCPUs            int      `json:"vcpus"`
	MemMB            int      `json:"mem_mb"`
	StorageMB        int      `json:"storage_mb"`
	DatabaseMB       int      `json:"database_mb"`
	RequestsPerMonth int      `json:"requests_per_month"` // 0 means unlimited
	AlwaysOn         bool     `json:"always_on"`
	CustomDomain     bool     `json:"custom_domain"`
	AutoSleep        bool     `json:"auto_sleep"`
	Features         []string `json:"features"`
}

// DefaultPlans returns the available billing plans.
func DefaultPlans() []Plan {
	allFeatures := []string{"auth", "admin", "files", "products", "monitoring", "legalpages", "profile", "system", "userportal", "web"}

	return []Plan{
		{
			ID: "free", Name: "Free", PriceID: "", PriceCents: 0,
			MaxVMs: 1, VCPUs: 1, MemMB: 256,
			StorageMB: 512, DatabaseMB: 100, RequestsPerMonth: 100000,
			AlwaysOn: false, CustomDomain: false, AutoSleep: true,
			Features: []string{"auth", "admin", "files", "profile", "system", "web"},
		},
		{
			ID: "hobby", Name: "Hobby", PriceID: "price_hobby", PriceCents: 500,
			MaxVMs: 1, VCPUs: 1, MemMB: 512,
			StorageMB: 2048, DatabaseMB: 500, RequestsPerMonth: 1000000,
			AlwaysOn: false, CustomDomain: false, AutoSleep: true,
			Features: allFeatures,
		},
		{
			ID: "starter", Name: "Starter", PriceID: "price_starter", PriceCents: 1500,
			MaxVMs: 1, VCPUs: 2, MemMB: 1024,
			StorageMB: 10240, DatabaseMB: 5120, RequestsPerMonth: 10000000,
			AlwaysOn: true, CustomDomain: false, AutoSleep: false,
			Features: allFeatures,
		},
		{
			ID: "professional", Name: "Professional", PriceID: "price_professional", PriceCents: 7900,
			MaxVMs: 3, VCPUs: 2, MemMB: 2048,
			StorageMB: 51200, DatabaseMB: 20480, RequestsPerMonth: 100000000,
			AlwaysOn: true, CustomDomain: true, AutoSleep: false,
			Features: allFeatures,
		},
		{
			ID: "business", Name: "Business", PriceID: "price_business", PriceCents: 19900,
			MaxVMs: 10, VCPUs: 4, MemMB: 4096,
			StorageMB: 204800, DatabaseMB: 102400, RequestsPerMonth: 0, // unlimited
			AlwaysOn: true, CustomDomain: true, AutoSleep: false,
			Features: allFeatures,
		},
	}
}

// GetPlan returns the plan by ID, or nil if not found.
func GetPlan(planID string) *Plan {
	for _, p := range DefaultPlans() {
		if p.ID == planID {
			return &p
		}
	}
	return nil
}

// StripeClient wraps Stripe API calls.
type StripeClient struct {
	apiKey string
	client *http.Client
}

// NewStripeClient creates a Stripe API client.
func NewStripeClient(apiKey string) *StripeClient {
	return &StripeClient{
		apiKey: apiKey,
		client: &http.Client{Timeout: 30 * time.Second},
	}
}

// Customer represents a Stripe customer.
type Customer struct {
	ID    string `json:"id"`
	Email string `json:"email"`
}

// Subscription represents a Stripe subscription.
type Subscription struct {
	ID                 string `json:"id"`
	Status             string `json:"status"`
	CurrentPeriodStart int64  `json:"current_period_start"`
	CurrentPeriodEnd   int64  `json:"current_period_end"`
}

// UsageRecord represents a usage metering event.
type UsageRecord struct {
	SubscriptionItemID string `json:"subscription_item"`
	Quantity           int64  `json:"quantity"`
	Timestamp          int64  `json:"timestamp"`
	Action             string `json:"action"` // "increment" or "set"
}

// CreateCustomer creates a Stripe customer for a user.
func (s *StripeClient) CreateCustomer(ctx context.Context, email, userID string) (*Customer, error) {
	form := url.Values{}
	form.Set("email", email)
	form.Set("metadata[user_id]", userID)

	var result Customer
	if err := s.doForm(ctx, "POST", "/v1/customers", form, &result); err != nil {
		return nil, fmt.Errorf("create customer: %w", err)
	}
	return &result, nil
}

// CreateSubscription creates a subscription for a customer.
func (s *StripeClient) CreateSubscription(ctx context.Context, customerID, priceID string) (*Subscription, error) {
	if priceID == "" {
		// Free plan — no Stripe subscription needed
		return &Subscription{
			ID:     "free",
			Status: "active",
		}, nil
	}

	form := url.Values{}
	form.Set("customer", customerID)
	form.Set("items[0][price]", priceID)

	var result Subscription
	if err := s.doForm(ctx, "POST", "/v1/subscriptions", form, &result); err != nil {
		return nil, fmt.Errorf("create subscription: %w", err)
	}
	return &result, nil
}

// CancelSubscription cancels a subscription.
func (s *StripeClient) CancelSubscription(ctx context.Context, subscriptionID string) error {
	if subscriptionID == "free" {
		return nil
	}
	return s.doForm(ctx, "DELETE", "/v1/subscriptions/"+subscriptionID, nil, nil)
}

// ReportUsage reports metered usage for a subscription item.
func (s *StripeClient) ReportUsage(ctx context.Context, subscriptionItemID string, quantity int64) error {
	form := url.Values{}
	form.Set("quantity", fmt.Sprintf("%d", quantity))
	form.Set("timestamp", fmt.Sprintf("%d", time.Now().Unix()))
	form.Set("action", "increment")

	path := fmt.Sprintf("/v1/subscription_items/%s/usage_records", subscriptionItemID)
	return s.doForm(ctx, "POST", path, form, nil)
}

func (s *StripeClient) doForm(ctx context.Context, method, path string, form url.Values, result interface{}) error {
	var body io.Reader
	if form != nil {
		body = strings.NewReader(form.Encode())
	}

	req, err := http.NewRequestWithContext(ctx, method, "https://api.stripe.com"+path, body)
	if err != nil {
		return fmt.Errorf("create request: %w", err)
	}
	req.SetBasicAuth(s.apiKey, "")
	if form != nil {
		req.Header.Set("Content-Type", "application/x-www-form-urlencoded")
	}

	resp, err := s.client.Do(req)
	if err != nil {
		return fmt.Errorf("do request: %w", err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("read response: %w", err)
	}

	if resp.StatusCode >= 400 {
		return fmt.Errorf("stripe API error (status %d): %s", resp.StatusCode, string(respBody))
	}

	if result != nil && len(respBody) > 0 {
		if err := json.Unmarshal(respBody, result); err != nil {
			return fmt.Errorf("unmarshal response: %w", err)
		}
	}

	return nil
}

// UsageCollector periodically collects and reports usage from tenant nodes.
type UsageCollector struct {
	stripe   *StripeClient
	interval time.Duration
}

// NewUsageCollector creates a usage collector.
func NewUsageCollector(stripe *StripeClient, interval time.Duration) *UsageCollector {
	return &UsageCollector{
		stripe:   stripe,
		interval: interval,
	}
}

// UsageSummary contains metered usage for a tenant.
type UsageSummary struct {
	TenantID       string `json:"tenant_id"`
	ComputeMinutes int64  `json:"compute_minutes"`
	StorageBytes   int64  `json:"storage_bytes"`
	RequestCount   int64  `json:"request_count"`
}

// ReportBatch sends usage data for multiple tenants to Stripe.
func (uc *UsageCollector) ReportBatch(ctx context.Context, items map[string]string, summaries []UsageSummary) error {
	var buf bytes.Buffer
	for _, summary := range summaries {
		subItemID, ok := items[summary.TenantID]
		if !ok {
			continue
		}
		if err := uc.stripe.ReportUsage(ctx, subItemID, summary.ComputeMinutes); err != nil {
			fmt.Fprintf(&buf, "report usage for %s: %v\n", summary.TenantID, err)
		}
	}
	if buf.Len() > 0 {
		return fmt.Errorf("usage reporting errors:\n%s", buf.String())
	}
	return nil
}
