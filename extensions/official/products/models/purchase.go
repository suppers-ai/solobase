package models

import (
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

// LineItem represents a single item in a purchase
type LineItem struct {
	ProductID   uint                   `json:"productId"`
	ProductName string                 `json:"productName"`
	Quantity    int                    `json:"quantity"`
	UnitPrice   int64                  `json:"unitPrice"`  // In cents
	TotalPrice  int64                  `json:"totalPrice"` // In cents
	Variables   map[string]interface{} `json:"variables"`   // Variable values used for pricing
	Description string                 `json:"description"` // Item description for display
	Metadata    map[string]interface{} `json:"metadata"`    // Additional item metadata
}

// TaxItem represents a tax breakdown
type TaxItem struct {
	Jurisdiction string  `json:"jurisdiction"` // e.g., "NZ", "CA", "US-NY"
	Percentage   float64 `json:"percentage"`   // Tax rate as percentage
	Amount       int64   `json:"amount"`       // Tax amount in cents
	TaxCode      string  `json:"taxCode"`     // Tax classification code
	Description  string  `json:"description"`  // Human-readable description
}

// Purchase represents a customer purchase
type Purchase struct {
	ID                      uint             `json:"id"`
	UserID                  string           `json:"userId"`
	Provider                string           `json:"provider"`                  // Payment provider (stripe, paypal, etc.)
	ProviderSessionID       string           `json:"providerSessionId"`         // Stripe Checkout Session ID (cs_xxx)
	ProviderPaymentIntentID string           `json:"providerPaymentIntentId"`   // Stripe PaymentIntent ID (pi_xxx)
	ProviderSubscriptionID  string           `json:"providerSubscriptionId"`    // For recurring purchases (sub_xxx)
	LineItems               []LineItem       `json:"lineItems"`                 // Product breakdown
	ProductMetadata         JSONB            `json:"productMetadata"`           // Business context (dates, notes, etc.)
	TaxItems                []TaxItem        `json:"taxItems"`                  // Tax breakdowns
	AmountCents             int64            `json:"amountCents"`               // Subtotal in cents (before tax)
	TaxCents                int64            `json:"taxCents"`                  // Total tax in cents
	TotalCents              int64            `json:"totalCents"`                // Total amount in cents (including tax)
	Currency                string           `json:"currency"`                  // Currency code
	Status                  string           `json:"status"`                    // pending, paid, refunded, cancelled, requires_approval, paid_pending_approval
	RequiresApproval        bool             `json:"requiresApproval"`          // Whether approval is needed
	ApprovedAt              apptime.NullTime `json:"approvedAt,omitempty"`      // When purchase was approved
	ApprovedBy              *string          `json:"approvedBy,omitempty"`      // User who approved
	RefundedAt              apptime.NullTime `json:"refundedAt,omitempty"`      // When refund was processed
	RefundReason            string           `json:"refundReason,omitempty"`    // Reason for refund
	RefundAmount            int64            `json:"refundAmount,omitempty"`    // Amount refunded in cents
	CancelledAt             apptime.NullTime `json:"cancelledAt,omitempty"`     // When purchase was cancelled
	CancelReason            string           `json:"cancelReason,omitempty"`    // Reason for cancellation
	SuccessURL              string           `json:"successUrl,omitempty"`      // Redirect URL after successful payment
	CancelURL               string           `json:"cancelUrl,omitempty"`       // Redirect URL if payment is cancelled
	CustomerEmail           string           `json:"customerEmail,omitempty"`   // Customer email for receipt
	CustomerName            string           `json:"customerName,omitempty"`    // Customer name
	BillingAddress          JSONB            `json:"billingAddress,omitempty"`  // Billing address details
	ShippingAddress         JSONB            `json:"shippingAddress,omitempty"` // Shipping address if applicable
	PaymentMethodTypes      []string         `json:"paymentMethodTypes"`        // Allowed payment methods
	ExpiresAt               apptime.NullTime `json:"expiresAt,omitempty"`       // When checkout session expires
	CreatedAt               apptime.Time     `json:"createdAt"`
	UpdatedAt               apptime.Time     `json:"updatedAt"`
}

// TableName specifies the table name with extension prefix
func (Purchase) TableName() string {
	return "ext_products_purchases"
}

// PrepareForCreate prepares the purchase for insertion
func (p *Purchase) PrepareForCreate() {
	now := apptime.NowTime()
	if p.CreatedAt.IsZero() {
		p.CreatedAt = now
	}
	p.UpdatedAt = now
	if p.Provider == "" {
		p.Provider = PaymentProviderStripe
	}
	if p.Currency == "" {
		p.Currency = "USD"
	}
	if p.Status == "" {
		p.Status = PurchaseStatusPending
	}
}

// PurchaseStatus constants
const (
	PurchaseStatusPending             = "pending"
	PurchaseStatusPaid                = "paid"
	PurchaseStatusRefunded            = "refunded"
	PurchaseStatusCancelled           = "cancelled"
	PurchaseStatusRequiresApproval    = "requires_approval"
	PurchaseStatusPaidPendingApproval = "paid_pending_approval"
)

// PaymentProvider constants
const (
	PaymentProviderStripe = "stripe"
	PaymentProviderPayPal = "paypal"
	PaymentProviderManual = "manual"
)

// IsPaid returns true if the purchase has been paid
func (p *Purchase) IsPaid() bool {
	return p.Status == PurchaseStatusPaid || p.Status == PurchaseStatusPaidPendingApproval
}

// CanRefund returns true if the purchase can be refunded
func (p *Purchase) CanRefund() bool {
	return p.IsPaid() && p.Status != PurchaseStatusRefunded
}

// CanCancel returns true if the purchase can be cancelled
func (p *Purchase) CanCancel() bool {
	return p.Status == PurchaseStatusPending || p.Status == PurchaseStatusRequiresApproval
}

// NeedsApproval returns true if the purchase needs approval
func (p *Purchase) NeedsApproval() bool {
	return p.Status == PurchaseStatusRequiresApproval || p.Status == PurchaseStatusPaidPendingApproval
}

