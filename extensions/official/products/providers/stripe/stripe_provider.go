package stripe

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"time"

	"github.com/stripe/stripe-go/v79"
	"github.com/stripe/stripe-go/v79/checkout/session"
	"github.com/stripe/stripe-go/v79/refund"
	"github.com/stripe/stripe-go/v79/webhook"
	"github.com/suppers-ai/solobase/extensions/official/products/models"
	"github.com/suppers-ai/solobase/extensions/official/products/providers/events"
)

// Provider implements the PaymentProvider interface for Stripe
type Provider struct {
	name           string
	testMode       bool
	configured     bool
	apiKey         string
	webhookSecret  string
	publishableKey string
}

// New creates a new Stripe provider
func New() *Provider {
	apiKey := os.Getenv("STRIPE_SECRET_KEY")
	webhookSecret := os.Getenv("STRIPE_WEBHOOK_SECRET")
	publishableKey := os.Getenv("STRIPE_PUBLISHABLE_KEY")

	// Detect if using test mode
	isTestMode := len(apiKey) > 3 && apiKey[:3] == "sk_test"

	provider := &Provider{
		name:           "stripe",
		testMode:       isTestMode,
		configured:     apiKey != "",
		apiKey:         apiKey,
		webhookSecret:  webhookSecret,
		publishableKey: publishableKey,
	}

	// Set the API key for the Stripe client if configured
	if provider.configured {
		stripe.Key = apiKey
	}

	return provider
}

// CreateCheckoutSession creates a new Stripe checkout session
func (p *Provider) CreateCheckoutSession(purchase *models.Purchase) (string, error) {
	if !p.IsEnabled() {
		return "", errors.New("stripe provider not enabled")
	}

	// Convert line items to Stripe format
	var lineItems []*stripe.CheckoutSessionLineItemParams
	for _, item := range purchase.LineItems {
		lineItem := &stripe.CheckoutSessionLineItemParams{
			PriceData: &stripe.CheckoutSessionLineItemPriceDataParams{
				Currency: stripe.String(purchase.Currency),
				ProductData: &stripe.CheckoutSessionLineItemPriceDataProductDataParams{
					Name:        stripe.String(item.ProductName),
					Description: stripe.String(item.Description),
				},
				UnitAmount: stripe.Int64(item.UnitPrice),
			},
			Quantity: stripe.Int64(int64(item.Quantity)),
		}
		lineItems = append(lineItems, lineItem)
	}

	// Default payment methods if not specified
	paymentMethods := purchase.PaymentMethodTypes
	if len(paymentMethods) == 0 {
		paymentMethods = []string{"card"}
	}

	params := &stripe.CheckoutSessionParams{
		Mode:               stripe.String(string(stripe.CheckoutSessionModePayment)),
		LineItems:          lineItems,
		PaymentMethodTypes: stripe.StringSlice(paymentMethods),
		SuccessURL:         stripe.String(purchase.SuccessURL),
		CancelURL:          stripe.String(purchase.CancelURL),
		ClientReferenceID:  stripe.String(fmt.Sprintf("%d", purchase.ID)),
		Metadata: map[string]string{
			"purchase_id": fmt.Sprintf("%d", purchase.ID),
			"user_id":     purchase.UserID,
		},
	}

	// Add customer email if provided
	if purchase.CustomerEmail != "" {
		params.CustomerEmail = stripe.String(purchase.CustomerEmail)
	}

	// TODO: Add automatic tax collection when CollectTax field is added to Purchase model
	// if purchase.CollectTax {
	//     params.AutomaticTax = &stripe.CheckoutSessionAutomaticTaxParams{
	//         Enabled: stripe.Bool(true),
	//     }
	// }

	// Set expiration (default 24 hours)
	if purchase.ExpiresAt != nil {
		params.ExpiresAt = stripe.Int64(purchase.ExpiresAt.Unix())
	} else {
		params.ExpiresAt = stripe.Int64(purchase.CreatedAt.Add(24 * time.Hour).Unix())
	}

	// Create the session
	sess, err := session.New(params)
	if err != nil {
		return "", fmt.Errorf("failed to create Stripe checkout session: %w", err)
	}

	return sess.ID, nil
}

// GetCheckoutURL generates the checkout URL for a given session ID
func (p *Provider) GetCheckoutURL(sessionID string) string {
	if sessionID == "" {
		return ""
	}
	if p.testMode {
		// Test mode URL format
		return fmt.Sprintf("https://checkout.stripe.com/c/pay/%s#", sessionID)
	}
	// Production URL format
	return fmt.Sprintf("https://checkout.stripe.com/pay/%s", sessionID)
}

// ExpireCheckoutSession expires a checkout session
func (p *Provider) ExpireCheckoutSession(sessionID string) error {
	if !p.IsEnabled() {
		return errors.New("stripe provider not enabled")
	}

	params := &stripe.CheckoutSessionExpireParams{}
	_, err := session.Expire(sessionID, params)
	if err != nil {
		// Check if session is already expired or completed
		stripeErr, ok := err.(*stripe.Error)
		if ok && (stripeErr.Code == "resource_missing" || stripeErr.HTTPStatusCode == 404) {
			// Session doesn't exist or already expired, not an error
			return nil
		}
		return fmt.Errorf("failed to expire checkout session: %w", err)
	}
	return nil
}

// GetCheckoutSession retrieves a checkout session by ID
func (p *Provider) GetCheckoutSession(sessionID string) (interface{}, error) {
	if !p.IsEnabled() {
		return nil, errors.New("stripe provider not enabled")
	}

	params := &stripe.CheckoutSessionParams{}
	params.AddExpand("line_items")
	params.AddExpand("payment_intent")
	params.AddExpand("subscription")

	sess, err := session.Get(sessionID, params)
	if err != nil {
		return nil, fmt.Errorf("failed to get checkout session: %w", err)
	}

	return sess, nil
}

// RefundPayment creates a refund for a payment
func (p *Provider) RefundPayment(paymentIntentID string, amountCents int64, reason string) error {
	if !p.IsEnabled() {
		return errors.New("stripe provider not enabled")
	}

	params := &stripe.RefundParams{
		PaymentIntent: stripe.String(paymentIntentID),
	}

	// If amount is specified, do partial refund
	if amountCents > 0 {
		params.Amount = stripe.Int64(amountCents)
	}

	// Map reason to Stripe refund reason
	switch reason {
	case "duplicate":
		params.Reason = stripe.String(string(stripe.RefundReasonDuplicate))
	case "fraudulent":
		params.Reason = stripe.String(string(stripe.RefundReasonFraudulent))
	case "requested_by_customer":
		params.Reason = stripe.String(string(stripe.RefundReasonRequestedByCustomer))
	default:
		// Use requested_by_customer as default
		params.Reason = stripe.String(string(stripe.RefundReasonRequestedByCustomer))
	}

	_, err := refund.New(params)
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

	event, err := webhook.ConstructEvent(payload, signature, p.webhookSecret)
	if err != nil {
		return nil, fmt.Errorf("webhook signature verification failed: %w", err)
	}
	return event, nil
}

// GetWebhookSigningSecret returns the webhook signing secret
func (p *Provider) GetWebhookSigningSecret() string {
	return p.webhookSecret
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
	return p.configured
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
	event, err := webhook.ConstructEvent(payload, signature, p.webhookSecret)
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

// Event conversion methods - convert Stripe events to generic provider events
func (p *Provider) convertCheckoutCompleted(event stripe.Event) (events.CheckoutCompletedEvent, error) {
	var session stripe.CheckoutSession
	if err := json.Unmarshal(event.Data.Raw, &session); err != nil {
		return events.CheckoutCompletedEvent{}, fmt.Errorf("failed to parse checkout session: %w", err)
	}

	// Get full session details
	fullSession, err := p.GetCheckoutSession(session.ID)
	if err != nil {
		return events.CheckoutCompletedEvent{}, fmt.Errorf("failed to retrieve full session: %w", err)
	}

	stripeSession, ok := fullSession.(*stripe.CheckoutSession)
	if !ok {
		return events.CheckoutCompletedEvent{}, fmt.Errorf("invalid session type")
	}

	genericEvent := events.CheckoutCompletedEvent{
		SessionID:       session.ID,
		AmountTotal:     stripeSession.AmountTotal,
		Currency:        string(stripeSession.Currency),
		Metadata:        stripeSession.Metadata,
	}

	if stripeSession.PaymentIntent != nil {
		genericEvent.PaymentIntentID = stripeSession.PaymentIntent.ID
	}

	if stripeSession.CustomerDetails != nil {
		genericEvent.CustomerEmail = stripeSession.CustomerDetails.Email
		genericEvent.CustomerName = stripeSession.CustomerDetails.Name
	}

	if stripeSession.TotalDetails != nil && stripeSession.TotalDetails.AmountTax > 0 {
		genericEvent.TaxAmount = stripeSession.TotalDetails.AmountTax
	}

	return genericEvent, nil
}

func (p *Provider) convertCheckoutExpired(event stripe.Event) (events.CheckoutExpiredEvent, error) {
	var session stripe.CheckoutSession
	if err := json.Unmarshal(event.Data.Raw, &session); err != nil {
		return events.CheckoutExpiredEvent{}, fmt.Errorf("failed to parse checkout session: %w", err)
	}

	return events.CheckoutExpiredEvent{
		SessionID: session.ID,
		Metadata:  session.Metadata,
	}, nil
}

func (p *Provider) convertPaymentSucceeded(event stripe.Event) (events.PaymentSucceededEvent, error) {
	var intent stripe.PaymentIntent
	if err := json.Unmarshal(event.Data.Raw, &intent); err != nil {
		return events.PaymentSucceededEvent{}, fmt.Errorf("failed to parse payment intent: %w", err)
	}

	return events.PaymentSucceededEvent{
		PaymentIntentID: intent.ID,
		Amount:          intent.Amount,
		Currency:        string(intent.Currency),
		Metadata:        intent.Metadata,
	}, nil
}

func (p *Provider) convertPaymentFailed(event stripe.Event) (events.PaymentFailedEvent, error) {
	var intent stripe.PaymentIntent
	if err := json.Unmarshal(event.Data.Raw, &intent); err != nil {
		return events.PaymentFailedEvent{}, fmt.Errorf("failed to parse payment intent: %w", err)
	}

	genericEvent := events.PaymentFailedEvent{
		PaymentIntentID: intent.ID,
		Metadata:        intent.Metadata,
		FailureReason:   "Payment failed",
	}

	if intent.LastPaymentError != nil {
		if intent.LastPaymentError.Msg != "" {
			genericEvent.FailureReason = intent.LastPaymentError.Msg
		}
		if intent.LastPaymentError.Code != "" {
			genericEvent.FailureCode = string(intent.LastPaymentError.Code)
		}
	}

	return genericEvent, nil
}

func (p *Provider) convertChargeRefunded(event stripe.Event) (events.RefundProcessedEvent, error) {
	var charge stripe.Charge
	if err := json.Unmarshal(event.Data.Raw, &charge); err != nil {
		return events.RefundProcessedEvent{}, fmt.Errorf("failed to parse charge: %w", err)
	}

	genericEvent := events.RefundProcessedEvent{
		RefundAmount: charge.AmountRefunded,
		Metadata:     charge.Metadata,
	}

	if charge.PaymentIntent != nil {
		genericEvent.PaymentIntentID = charge.PaymentIntent.ID
	}

	// Get the refund reason if available
	if charge.Refunds != nil && len(charge.Refunds.Data) > 0 {
		latestRefund := charge.Refunds.Data[0]
		genericEvent.RefundID = latestRefund.ID
		if latestRefund.Reason != "" {
			genericEvent.Reason = string(latestRefund.Reason)
		}
	}

	return genericEvent, nil
}


