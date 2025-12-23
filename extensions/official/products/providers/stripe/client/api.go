package client

import (
	"encoding/json"
	"fmt"
	"net/url"
	"strconv"
	"strings"
)

const (
	baseURL = "https://api.stripe.com/v1"
)

// CreateCheckoutSession creates a new checkout session
func (c *Client) CreateCheckoutSession(params *CheckoutSessionParams) (*CheckoutSession, error) {
	data := url.Values{}
	data.Set("mode", params.Mode)
	data.Set("success_url", params.SuccessURL)
	data.Set("cancel_url", params.CancelURL)

	if params.ClientReferenceID != "" {
		data.Set("client_reference_id", params.ClientReferenceID)
	}
	if params.CustomerEmail != "" {
		data.Set("customer_email", params.CustomerEmail)
	}

	// Add line items
	for i, item := range params.LineItems {
		prefix := fmt.Sprintf("line_items[%d]", i)
		data.Set(prefix+"[price_data][currency]", item.Currency)
		data.Set(prefix+"[price_data][unit_amount]", strconv.FormatInt(item.UnitAmount, 10))
		data.Set(prefix+"[price_data][product_data][name]", item.Name)
		if item.Description != "" {
			data.Set(prefix+"[price_data][product_data][description]", item.Description)
		}
		data.Set(prefix+"[quantity]", strconv.FormatInt(item.Quantity, 10))
	}

	// Add payment method types
	for i, pm := range params.PaymentMethodTypes {
		data.Set(fmt.Sprintf("payment_method_types[%d]", i), pm)
	}

	// Add metadata
	for k, v := range params.Metadata {
		data.Set(fmt.Sprintf("metadata[%s]", k), v)
	}

	// Add expiration
	if params.ExpiresAt != nil {
		data.Set("expires_at", strconv.FormatInt(params.ExpiresAt.Unix(), 10))
	}

	resp, err := c.request("POST", "/checkout/sessions", data)
	if err != nil {
		return nil, err
	}

	var session CheckoutSession
	if err := json.Unmarshal(resp, &session); err != nil {
		return nil, fmt.Errorf("failed to parse response: %w", err)
	}

	return &session, nil
}

// GetCheckoutSession retrieves a checkout session by ID
func (c *Client) GetCheckoutSession(sessionID string, expand []string) (*CheckoutSession, error) {
	data := url.Values{}
	for i, e := range expand {
		data.Set(fmt.Sprintf("expand[%d]", i), e)
	}

	path := "/checkout/sessions/" + sessionID
	if len(data) > 0 {
		path += "?" + data.Encode()
	}

	resp, err := c.requestGet(path)
	if err != nil {
		return nil, err
	}

	var session CheckoutSession
	if err := json.Unmarshal(resp, &session); err != nil {
		return nil, fmt.Errorf("failed to parse response: %w", err)
	}

	return &session, nil
}

// ExpireCheckoutSession expires a checkout session
func (c *Client) ExpireCheckoutSession(sessionID string) error {
	_, err := c.request("POST", "/checkout/sessions/"+sessionID+"/expire", nil)
	if err != nil {
		// Check if session is already expired (not an error)
		if apiErr, ok := err.(*APIError); ok {
			if apiErr.Code == "resource_missing" || apiErr.StatusCode == 404 {
				return nil
			}
		}
		return err
	}
	return nil
}

// CreateRefund creates a refund
func (c *Client) CreateRefund(params *RefundParams) (*Refund, error) {
	data := url.Values{}
	data.Set("payment_intent", params.PaymentIntent)

	if params.Amount > 0 {
		data.Set("amount", strconv.FormatInt(params.Amount, 10))
	}

	if params.Reason != "" {
		data.Set("reason", params.Reason)
	}

	resp, err := c.request("POST", "/refunds", data)
	if err != nil {
		return nil, err
	}

	var refund Refund
	if err := json.Unmarshal(resp, &refund); err != nil {
		return nil, fmt.Errorf("failed to parse response: %w", err)
	}

	return &refund, nil
}

// request makes a POST request to the Stripe API
func (c *Client) request(method, path string, data url.Values) ([]byte, error) {
	var body []byte
	if data != nil {
		body = []byte(data.Encode())
	}

	headers := map[string]string{
		"Authorization": "Bearer " + c.APIKey,
		"Content-Type":  "application/x-www-form-urlencoded",
	}

	resp, err := c.httpClient.Do(method, baseURL+path, body, headers)
	if err != nil {
		return nil, fmt.Errorf("request failed: %w", err)
	}

	if resp.StatusCode >= 400 {
		return nil, c.parseError(resp)
	}

	return resp.Body, nil
}

// requestGet makes a GET request to the Stripe API
func (c *Client) requestGet(path string) ([]byte, error) {
	headers := map[string]string{
		"Authorization": "Bearer " + c.APIKey,
	}

	// Handle query params in path
	fullURL := baseURL + path

	resp, err := c.httpClient.Do("GET", fullURL, nil, headers)
	if err != nil {
		return nil, fmt.Errorf("request failed: %w", err)
	}

	if resp.StatusCode >= 400 {
		return nil, c.parseError(resp)
	}

	return resp.Body, nil
}

// parseError parses a Stripe API error response
func (c *Client) parseError(resp *Response) error {
	var errResp struct {
		Error APIError `json:"error"`
	}

	if err := json.Unmarshal(resp.Body, &errResp); err != nil {
		return &APIError{
			Type:       "api_error",
			Message:    string(resp.Body),
			StatusCode: resp.StatusCode,
		}
	}

	errResp.Error.StatusCode = resp.StatusCode
	return &errResp.Error
}

// IsTestMode returns true if using test API keys
func (c *Client) IsTestMode() bool {
	return strings.HasPrefix(c.APIKey, "sk_test_")
}

// GetCheckoutURL generates the checkout URL for a session
func (c *Client) GetCheckoutURL(sessionID string) string {
	if sessionID == "" {
		return ""
	}
	if c.IsTestMode() {
		return fmt.Sprintf("https://checkout.stripe.com/c/pay/%s#", sessionID)
	}
	return fmt.Sprintf("https://checkout.stripe.com/pay/%s", sessionID)
}
