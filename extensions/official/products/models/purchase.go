package models

import (
	"time"
)

// LineItem represents a single item in a purchase
type LineItem struct {
	ProductID   uint                   `json:"product_id"`
	ProductName string                 `json:"product_name"`
	Quantity    int                    `json:"quantity"`
	UnitPrice   int64                  `json:"unit_price"`  // In cents
	TotalPrice  int64                  `json:"total_price"` // In cents
	Variables   map[string]interface{} `json:"variables"`   // Variable values used for pricing
	Description string                 `json:"description"` // Item description for display
	Metadata    map[string]interface{} `json:"metadata"`    // Additional item metadata
}

// TaxItem represents a tax breakdown
type TaxItem struct {
	Jurisdiction string  `json:"jurisdiction"` // e.g., "NZ", "CA", "US-NY"
	Percentage   float64 `json:"percentage"`   // Tax rate as percentage
	Amount       int64   `json:"amount"`       // Tax amount in cents
	TaxCode      string  `json:"tax_code"`     // Tax classification code
	Description  string  `json:"description"`  // Human-readable description
}

// Purchase represents a customer purchase
type Purchase struct {
	ID                      uint       `gorm:"primaryKey" json:"id"`
	UserID                  string     `gorm:"index;size:36;not null" json:"user_id"`
	Provider                string     `gorm:"default:'stripe'" json:"provider"`                       // Payment provider (stripe, paypal, etc.)
	ProviderSessionID       string     `gorm:"index" json:"provider_session_id"`                       // Stripe Checkout Session ID (cs_xxx)
	ProviderPaymentIntentID string     `gorm:"index" json:"provider_payment_intent_id"`                // Stripe PaymentIntent ID (pi_xxx)
	ProviderSubscriptionID  string     `gorm:"index" json:"provider_subscription_id"`                  // For recurring purchases (sub_xxx)
	LineItems               []LineItem `gorm:"type:jsonb;serializer:json" json:"line_items"`           // Product breakdown
	ProductMetadata         JSONB      `gorm:"type:jsonb" json:"product_metadata"`                     // Business context (dates, notes, etc.)
	TaxItems                []TaxItem  `gorm:"type:jsonb;serializer:json" json:"tax_items"`            // Tax breakdowns
	AmountCents             int64      `json:"amount_cents"`                                           // Subtotal in cents (before tax)
	TaxCents                int64      `json:"tax_cents"`                                              // Total tax in cents
	TotalCents              int64      `json:"total_cents"`                                            // Total amount in cents (including tax)
	Currency                string     `gorm:"default:'USD'" json:"currency"`                          // Currency code
	Status                  string     `gorm:"default:'pending';index" json:"status"`                  // pending, paid, refunded, cancelled, requires_approval, paid_pending_approval
	RequiresApproval        bool       `gorm:"default:false" json:"requires_approval"`                 // Whether approval is needed
	ApprovedAt              *time.Time `json:"approved_at,omitempty"`                                  // When purchase was approved
	ApprovedBy              *string    `json:"approved_by,omitempty"`                                  // User who approved
	RefundedAt              *time.Time `json:"refunded_at,omitempty"`                                  // When refund was processed
	RefundReason            string     `json:"refund_reason,omitempty"`                                // Reason for refund
	RefundAmount            int64      `json:"refund_amount,omitempty"`                                // Amount refunded in cents
	CancelledAt             *time.Time `json:"cancelled_at,omitempty"`                                 // When purchase was cancelled
	CancelReason            string     `json:"cancel_reason,omitempty"`                                // Reason for cancellation
	SuccessURL              string     `json:"success_url,omitempty"`                                  // Redirect URL after successful payment
	CancelURL               string     `json:"cancel_url,omitempty"`                                   // Redirect URL if payment is cancelled
	CustomerEmail           string     `json:"customer_email,omitempty"`                               // Customer email for receipt
	CustomerName            string     `json:"customer_name,omitempty"`                                // Customer name
	BillingAddress          JSONB      `gorm:"type:jsonb" json:"billing_address,omitempty"`            // Billing address details
	ShippingAddress         JSONB      `gorm:"type:jsonb" json:"shipping_address,omitempty"`           // Shipping address if applicable
	PaymentMethodTypes      []string   `gorm:"type:jsonb;serializer:json" json:"payment_method_types"` // Allowed payment methods
	ExpiresAt               *time.Time `json:"expires_at,omitempty"`                                   // When checkout session expires
	CreatedAt               time.Time  `json:"created_at"`
	UpdatedAt               time.Time  `json:"updated_at"`
}

// TableName specifies the table name with extension prefix
func (Purchase) TableName() string {
	return "ext_products_purchases"
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

