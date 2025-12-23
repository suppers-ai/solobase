// Package client provides a lightweight Stripe API client that works in both
// standard Go and TinyGo WASM builds by using direct REST API calls.
package client

import "github.com/suppers-ai/solobase/internal/pkg/apptime"

// Client is a Stripe API client
type Client struct {
	APIKey        string
	WebhookSecret string
	httpClient    HTTPClient
}

// HTTPClient interface for making HTTP requests
// Implemented differently for standard and WASM builds
type HTTPClient interface {
	Do(method, url string, body []byte, headers map[string]string) (*Response, error)
}

// Response represents an HTTP response
type Response struct {
	StatusCode int
	Body       []byte
}

// New creates a new Stripe client
func New(apiKey, webhookSecret string) *Client {
	return &Client{
		APIKey:        apiKey,
		WebhookSecret: webhookSecret,
		httpClient:    newHTTPClient(),
	}
}

// CheckoutSession represents a Stripe checkout session
type CheckoutSession struct {
	ID                string            `json:"id"`
	Object            string            `json:"object"`
	AmountTotal       int64             `json:"amount_total"`
	Currency          string            `json:"currency"`
	CustomerEmail     string            `json:"customer_email,omitempty"`
	ClientReferenceID string            `json:"client_reference_id,omitempty"`
	Metadata          map[string]string `json:"metadata,omitempty"`
	Mode              string            `json:"mode"`
	PaymentStatus     string            `json:"payment_status"`
	Status            string            `json:"status"`
	URL               string            `json:"url,omitempty"`
	SuccessURL        string            `json:"success_url"`
	CancelURL         string            `json:"cancel_url"`
	ExpiresAt         int64             `json:"expires_at"`
	PaymentIntent     *PaymentIntent    `json:"payment_intent,omitempty"`
	CustomerDetails   *CustomerDetails  `json:"customer_details,omitempty"`
	TotalDetails      *TotalDetails     `json:"total_details,omitempty"`
}

// PaymentIntent represents a Stripe payment intent
type PaymentIntent struct {
	ID               string            `json:"id"`
	Object           string            `json:"object"`
	Amount           int64             `json:"amount"`
	Currency         string            `json:"currency"`
	Status           string            `json:"status"`
	Metadata         map[string]string `json:"metadata,omitempty"`
	LastPaymentError *PaymentError     `json:"last_payment_error,omitempty"`
}

// PaymentError represents a payment error
type PaymentError struct {
	Code    string `json:"code,omitempty"`
	Message string `json:"message,omitempty"`
}

// CustomerDetails represents customer details
type CustomerDetails struct {
	Email string `json:"email,omitempty"`
	Name  string `json:"name,omitempty"`
}

// TotalDetails represents total details including tax
type TotalDetails struct {
	AmountTax int64 `json:"amount_tax"`
}

// Charge represents a Stripe charge
type Charge struct {
	ID             string            `json:"id"`
	Amount         int64             `json:"amount"`
	AmountRefunded int64             `json:"amount_refunded"`
	Currency       string            `json:"currency"`
	PaymentIntent  string            `json:"payment_intent,omitempty"`
	Metadata       map[string]string `json:"metadata,omitempty"`
	Refunds        *RefundList       `json:"refunds,omitempty"`
}

// RefundList represents a list of refunds
type RefundList struct {
	Data []*Refund `json:"data"`
}

// Refund represents a Stripe refund
type Refund struct {
	ID     string `json:"id"`
	Amount int64  `json:"amount"`
	Reason string `json:"reason,omitempty"`
	Status string `json:"status"`
}

// Event represents a Stripe webhook event
type Event struct {
	ID      string    `json:"id"`
	Object  string    `json:"object"`
	Type    string    `json:"type"`
	Created int64     `json:"created"`
	Data    EventData `json:"data"`
}

// EventData contains the event payload
type EventData struct {
	Object map[string]interface{} `json:"object"`
	Raw    []byte                 `json:"-"` // Set after parsing
}

// APIError represents a Stripe API error
type APIError struct {
	Type       string `json:"type"`
	Code       string `json:"code,omitempty"`
	Message    string `json:"message"`
	StatusCode int    `json:"-"`
}

func (e *APIError) Error() string {
	return e.Message
}

// CheckoutSessionParams contains parameters for creating a checkout session
type CheckoutSessionParams struct {
	Mode               string
	SuccessURL         string
	CancelURL          string
	ClientReferenceID  string
	CustomerEmail      string
	Metadata           map[string]string
	LineItems          []LineItemParams
	PaymentMethodTypes []string
	ExpiresAt          *apptime.Time
}

// LineItemParams contains parameters for a line item
type LineItemParams struct {
	Name        string
	Description string
	UnitAmount  int64
	Currency    string
	Quantity    int64
}

// RefundParams contains parameters for creating a refund
type RefundParams struct {
	PaymentIntent string
	Amount        int64  // 0 for full refund
	Reason        string // duplicate, fraudulent, requested_by_customer
}
