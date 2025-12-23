// Package stripe provides a Stripe payment provider that works in both
// standard Go and TinyGo WASM builds using direct REST API calls.
package stripe

import (
	"errors"
	"fmt"

	"github.com/suppers-ai/solobase/internal/env"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"

	"github.com/suppers-ai/solobase/extensions/official/products/models"
	"github.com/suppers-ai/solobase/extensions/official/products/providers/events"
	"github.com/suppers-ai/solobase/extensions/official/products/providers/stripe/client"
)

// Provider implements the PaymentProvider interface for Stripe
type Provider struct {
	name           string
	testMode       bool
	configured     bool
	client         *client.Client
	publishableKey string
}

// New creates a new Stripe provider
func New() *Provider {
	apiKey := env.GetEnv("STRIPE_SECRET_KEY")
	webhookSecret := env.GetEnv("STRIPE_WEBHOOK_SECRET")
	publishableKey := env.GetEnv("STRIPE_PUBLISHABLE_KEY")

	// Detect if using test mode
	isTestMode := len(apiKey) > 7 && apiKey[:7] == "sk_test"

	provider := &Provider{
		name:           "stripe",
		testMode:       isTestMode,
		configured:     apiKey != "",
		publishableKey: publishableKey,
	}

	// Create the client if configured
	if provider.configured {
		provider.client = client.New(apiKey, webhookSecret)
	}

	return provider
}

// CreateCheckoutSession creates a new Stripe checkout session
func (p *Provider) CreateCheckoutSession(purchase *models.Purchase) (string, error) {
	if !p.IsEnabled() {
		return "", errors.New("stripe provider not enabled")
	}

	// Convert line items to client format
	var lineItems []client.LineItemParams
	for _, item := range purchase.LineItems {
		lineItems = append(lineItems, client.LineItemParams{
			Name:        item.ProductName,
			Description: item.Description,
			UnitAmount:  item.UnitPrice,
			Currency:    purchase.Currency,
			Quantity:    int64(item.Quantity),
		})
	}

	// Default payment methods if not specified
	paymentMethods := purchase.PaymentMethodTypes
	if len(paymentMethods) == 0 {
		paymentMethods = []string{"card"}
	}

	// Set expiration (default 24 hours)
	var expiresAt *apptime.Time
	if purchase.ExpiresAt.Valid {
		expiresAt = &purchase.ExpiresAt.Time
	} else {
		exp := purchase.CreatedAt.Add(24 * apptime.Hour)
		expiresAt = &exp
	}

	params := &client.CheckoutSessionParams{
		Mode:              "payment",
		SuccessURL:        purchase.SuccessURL,
		CancelURL:         purchase.CancelURL,
		ClientReferenceID: fmt.Sprintf("%d", purchase.ID),
		CustomerEmail:     purchase.CustomerEmail,
		Metadata: map[string]string{
			"purchase_id": fmt.Sprintf("%d", purchase.ID),
			"user_id":     purchase.UserID,
		},
		LineItems:          lineItems,
		PaymentMethodTypes: paymentMethods,
		ExpiresAt:          expiresAt,
	}

	session, err := p.client.CreateCheckoutSession(params)
	if err != nil {
		return "", fmt.Errorf("failed to create Stripe checkout session: %w", err)
	}

	return session.ID, nil
}

// GetCheckoutURL generates the checkout URL for a given session ID
func (p *Provider) GetCheckoutURL(sessionID string) string {
	if p.client == nil {
		return ""
	}
	return p.client.GetCheckoutURL(sessionID)
}

// ExpireCheckoutSession expires a checkout session
func (p *Provider) ExpireCheckoutSession(sessionID string) error {
	if !p.IsEnabled() {
		return errors.New("stripe provider not enabled")
	}

	return p.client.ExpireCheckoutSession(sessionID)
}

// GetCheckoutSession retrieves a checkout session by ID
func (p *Provider) GetCheckoutSession(sessionID string) (interface{}, error) {
	if !p.IsEnabled() {
		return nil, errors.New("stripe provider not enabled")
	}

	session, err := p.client.GetCheckoutSession(sessionID, []string{
		"line_items",
		"payment_intent",
	})
	if err != nil {
		return nil, fmt.Errorf("failed to get checkout session: %w", err)
	}

	return session, nil
}

// RefundPayment creates a refund for a payment
func (p *Provider) RefundPayment(paymentIntentID string, amountCents int64, reason string) error {
	if !p.IsEnabled() {
		return errors.New("stripe provider not enabled")
	}

	// Map reason to Stripe refund reason
	stripeReason := "requested_by_customer"
	switch reason {
	case "duplicate":
		stripeReason = "duplicate"
	case "fraudulent":
		stripeReason = "fraudulent"
	case "requested_by_customer":
		stripeReason = "requested_by_customer"
	}

	_, err := p.client.CreateRefund(&client.RefundParams{
		PaymentIntent: paymentIntentID,
		Amount:        amountCents,
		Reason:        stripeReason,
	})
	if err != nil {
		return fmt.Errorf("failed to create refund: %w", err)
	}

	return nil
}

// ValidateWebhook validates and parses a webhook payload
func (p *Provider) ValidateWebhook(payload []byte, signature string) (interface{}, error) {
	if !p.IsEnabled() {
		return nil, errors.New("stripe provider not enabled")
	}

	event, err := p.client.ConstructEvent(payload, signature)
	if err != nil {
		return nil, fmt.Errorf("webhook signature verification failed: %w", err)
	}
	return event, nil
}

// GetWebhookSigningSecret returns the webhook signing secret
func (p *Provider) GetWebhookSigningSecret() string {
	if p.client == nil {
		return ""
	}
	return p.client.WebhookSecret
}

// GetWebhookPath returns the webhook path for Stripe
func (p *Provider) GetWebhookPath() string {
	return "/stripe"
}

// GetPublishableKey returns the Stripe publishable key
func (p *Provider) GetPublishableKey() string {
	return p.publishableKey
}

// GetProviderName returns the provider's name
func (p *Provider) GetProviderName() string {
	return p.name
}

// IsEnabled returns whether the provider is configured and enabled
func (p *Provider) IsEnabled() bool {
	return p.configured && p.client != nil
}

// IsTestMode returns whether the provider is in test mode
func (p *Provider) IsTestMode() bool {
	return p.testMode
}

// HandleWebhook processes Stripe webhook events and converts them to generic events
func (p *Provider) HandleWebhook(payload []byte, signature string, handler func(event interface{}) error) error {
	if !p.IsEnabled() {
		return errors.New("stripe provider not enabled")
	}

	// Validate and parse the webhook
	event, err := p.client.ConstructEvent(payload, signature)
	if err != nil {
		return fmt.Errorf("webhook signature verification failed: %w", err)
	}

	// Convert Stripe events to generic provider events
	switch event.Type {
	case "checkout.session.completed":
		genericEvent, err := p.convertCheckoutCompleted(event)
		if err != nil {
			return err
		}
		return handler(genericEvent)

	case "checkout.session.expired":
		genericEvent, err := p.convertCheckoutExpired(event)
		if err != nil {
			return err
		}
		return handler(genericEvent)

	case "payment_intent.succeeded":
		genericEvent, err := p.convertPaymentSucceeded(event)
		if err != nil {
			return err
		}
		return handler(genericEvent)

	case "payment_intent.payment_failed":
		genericEvent, err := p.convertPaymentFailed(event)
		if err != nil {
			return err
		}
		return handler(genericEvent)

	case "charge.refunded":
		genericEvent, err := p.convertChargeRefunded(event)
		if err != nil {
			return err
		}
		return handler(genericEvent)

	default:
		// Return nil for unhandled events (not an error)
		fmt.Printf("Unhandled Stripe webhook event type: %s\n", event.Type)
		return nil
	}
}

// Event conversion methods

func (p *Provider) convertCheckoutCompleted(event *client.Event) (events.CheckoutCompletedEvent, error) {
	session, err := client.ParseCheckoutSession(event.Data.Raw)
	if err != nil {
		return events.CheckoutCompletedEvent{}, fmt.Errorf("failed to parse checkout session: %w", err)
	}

	// Get full session details with expanded fields
	fullSession, err := p.client.GetCheckoutSession(session.ID, []string{
		"line_items",
		"payment_intent",
	})
	if err != nil {
		return events.CheckoutCompletedEvent{}, fmt.Errorf("failed to retrieve full session: %w", err)
	}

	genericEvent := events.CheckoutCompletedEvent{
		SessionID:   session.ID,
		AmountTotal: fullSession.AmountTotal,
		Currency:    fullSession.Currency,
		Metadata:    fullSession.Metadata,
	}

	if fullSession.PaymentIntent != nil {
		genericEvent.PaymentIntentID = fullSession.PaymentIntent.ID
	}

	if fullSession.CustomerDetails != nil {
		genericEvent.CustomerEmail = fullSession.CustomerDetails.Email
		genericEvent.CustomerName = fullSession.CustomerDetails.Name
	}

	if fullSession.TotalDetails != nil && fullSession.TotalDetails.AmountTax > 0 {
		genericEvent.TaxAmount = fullSession.TotalDetails.AmountTax
	}

	return genericEvent, nil
}

func (p *Provider) convertCheckoutExpired(event *client.Event) (events.CheckoutExpiredEvent, error) {
	session, err := client.ParseCheckoutSession(event.Data.Raw)
	if err != nil {
		return events.CheckoutExpiredEvent{}, fmt.Errorf("failed to parse checkout session: %w", err)
	}

	return events.CheckoutExpiredEvent{
		SessionID: session.ID,
		Metadata:  session.Metadata,
	}, nil
}

func (p *Provider) convertPaymentSucceeded(event *client.Event) (events.PaymentSucceededEvent, error) {
	intent, err := client.ParsePaymentIntent(event.Data.Raw)
	if err != nil {
		return events.PaymentSucceededEvent{}, fmt.Errorf("failed to parse payment intent: %w", err)
	}

	return events.PaymentSucceededEvent{
		PaymentIntentID: intent.ID,
		Amount:          intent.Amount,
		Currency:        intent.Currency,
		Metadata:        intent.Metadata,
	}, nil
}

func (p *Provider) convertPaymentFailed(event *client.Event) (events.PaymentFailedEvent, error) {
	intent, err := client.ParsePaymentIntent(event.Data.Raw)
	if err != nil {
		return events.PaymentFailedEvent{}, fmt.Errorf("failed to parse payment intent: %w", err)
	}

	genericEvent := events.PaymentFailedEvent{
		PaymentIntentID: intent.ID,
		Metadata:        intent.Metadata,
		FailureReason:   "Payment failed",
	}

	if intent.LastPaymentError != nil {
		if intent.LastPaymentError.Message != "" {
			genericEvent.FailureReason = intent.LastPaymentError.Message
		}
		if intent.LastPaymentError.Code != "" {
			genericEvent.FailureCode = intent.LastPaymentError.Code
		}
	}

	return genericEvent, nil
}

func (p *Provider) convertChargeRefunded(event *client.Event) (events.RefundProcessedEvent, error) {
	charge, err := client.ParseCharge(event.Data.Raw)
	if err != nil {
		return events.RefundProcessedEvent{}, fmt.Errorf("failed to parse charge: %w", err)
	}

	genericEvent := events.RefundProcessedEvent{
		PaymentIntentID: charge.PaymentIntent,
		RefundAmount:    charge.AmountRefunded,
		Metadata:        charge.Metadata,
	}

	// Get the refund reason if available
	if charge.Refunds != nil && len(charge.Refunds.Data) > 0 {
		latestRefund := charge.Refunds.Data[0]
		genericEvent.RefundID = latestRefund.ID
		if latestRefund.Reason != "" {
			genericEvent.Reason = latestRefund.Reason
		}
	}

	return genericEvent, nil
}
