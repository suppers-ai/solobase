package providers

import (
	"errors"
	"github.com/suppers-ai/solobase/extensions/official/products/models"
)

// Common errors
var (
	ErrProviderNotConfigured = errors.New("payment provider not configured")
	ErrProviderNotEnabled    = errors.New("payment provider not enabled")
	ErrInvalidSession        = errors.New("invalid checkout session")
	ErrWebhookValidation     = errors.New("webhook validation failed")
)

// WebhookHandler is a function that handles webhook events
type WebhookHandler func(event interface{}) error

// PaymentProvider defines the interface for payment providers
type PaymentProvider interface {
	// Provider identification
	GetProviderName() string
	IsEnabled() bool
	IsTestMode() bool

	// Checkout operations
	CreateCheckoutSession(purchase *models.Purchase) (sessionID string, err error)
	GetCheckoutURL(sessionID string) string
	ExpireCheckoutSession(sessionID string) error
	GetCheckoutSession(sessionID string) (interface{}, error)

	// Payment operations
	RefundPayment(paymentIntentID string, amountCents int64, reason string) error

	// Webhook operations
	ValidateWebhook(payload []byte, signature string) (event interface{}, err error)
	GetWebhookSigningSecret() string
	GetWebhookPath() string // Returns the path for this provider's webhooks (e.g., "/stripe", "/paypal")
	HandleWebhook(payload []byte, signature string, handler func(event interface{}) error) error
}

// BaseProvider provides common functionality for all payment providers
type BaseProvider struct {
	Name       string
	TestMode   bool
	Configured bool
}

// GetProviderName returns the provider's name
func (b *BaseProvider) GetProviderName() string {
	return b.Name
}

// IsEnabled checks if the provider is configured and enabled
func (b *BaseProvider) IsEnabled() bool {
	return b.Configured
}

// IsTestMode returns whether the provider is in test mode
func (b *BaseProvider) IsTestMode() bool {
	return b.TestMode
}

// Default implementations that should be overridden by specific providers

// CreateCheckoutSession - must be implemented by specific provider
func (b *BaseProvider) CreateCheckoutSession(purchase *models.Purchase) (string, error) {
	return "", ErrProviderNotConfigured
}

// GetCheckoutURL - must be implemented by specific provider
func (b *BaseProvider) GetCheckoutURL(sessionID string) string {
	return ""
}

// ExpireCheckoutSession - must be implemented by specific provider
func (b *BaseProvider) ExpireCheckoutSession(sessionID string) error {
	return ErrProviderNotConfigured
}

// GetCheckoutSession - must be implemented by specific provider
func (b *BaseProvider) GetCheckoutSession(sessionID string) (interface{}, error) {
	return nil, ErrProviderNotConfigured
}

// RefundPayment - must be implemented by specific provider
func (b *BaseProvider) RefundPayment(paymentIntentID string, amountCents int64, reason string) error {
	return ErrProviderNotConfigured
}

// ValidateWebhook - must be implemented by specific provider
func (b *BaseProvider) ValidateWebhook(payload []byte, signature string) (interface{}, error) {
	return nil, ErrProviderNotConfigured
}

// GetWebhookSigningSecret - must be implemented by specific provider
func (b *BaseProvider) GetWebhookSigningSecret() string {
	return ""
}

// GetWebhookPath - must be implemented by specific provider
func (b *BaseProvider) GetWebhookPath() string {
	return "/" + b.Name
}

// HandleWebhook - must be implemented by specific provider
func (b *BaseProvider) HandleWebhook(payload []byte, signature string, handler func(event interface{}) error) error {
	return ErrProviderNotConfigured
}

