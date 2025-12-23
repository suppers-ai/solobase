package products

import (
	"fmt"
	"io"
	"net/http"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"

	"github.com/suppers-ai/solobase/extensions/official/products/models"
	"github.com/suppers-ai/solobase/extensions/official/products/providers"
	"github.com/suppers-ai/solobase/extensions/official/products/providers/events"
)

// WebhookHandler handles payment provider webhook events
type WebhookHandler struct {
	paymentProvider providers.PaymentProvider
	purchaseService *PurchaseService
}

// NewWebhookHandler creates a new webhook handler
func NewWebhookHandler(paymentProvider providers.PaymentProvider, purchaseService *PurchaseService) *WebhookHandler {
	return &WebhookHandler{
		paymentProvider: paymentProvider,
		purchaseService: purchaseService,
	}
}

// HandleWebhook is the main entry point for all webhook requests
// It uses the configured payment provider from environment variables
func (h *WebhookHandler) HandleWebhook(w http.ResponseWriter, r *http.Request) {
	// Check if a payment provider is configured
	if h.paymentProvider == nil {
		http.Error(w, "No payment provider configured", http.StatusBadRequest)
		return
	}

	// Get the provider name for header selection
	providerName := h.paymentProvider.GetProviderName()

	// Read the request body
	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "Failed to read request body", http.StatusBadRequest)
		return
	}

	// Get the signature from headers (provider-specific)
	var signature string
	switch providerName {
	case "stripe":
		signature = r.Header.Get("Stripe-Signature")
	case "paypal":
		signature = r.Header.Get("Paypal-Transmission-Sig")
	default:
		signature = r.Header.Get("X-Webhook-Signature")
	}

	if signature == "" {
		http.Error(w, "Missing webhook signature", http.StatusBadRequest)
		return
	}

	// Delegate to the provider's webhook handler
	err = h.paymentProvider.HandleWebhook(body, signature, h.processWebhookEvent)
	if err != nil {
		http.Error(w, fmt.Sprintf("Failed to process webhook: %v", err), http.StatusBadRequest)
		return
	}

	// Return success response
	w.WriteHeader(http.StatusOK)
}

// processWebhookEvent handles the business logic for webhook events
func (h *WebhookHandler) processWebhookEvent(event interface{}) error {
	// Type switch on generic event types
	switch e := event.(type) {
	case events.CheckoutCompletedEvent:
		return h.handleCheckoutCompleted(e)
	case events.CheckoutExpiredEvent:
		return h.handleCheckoutExpired(e)
	case events.PaymentSucceededEvent:
		return h.handlePaymentSucceeded(e)
	case events.PaymentFailedEvent:
		return h.handlePaymentFailed(e)
	case events.RefundProcessedEvent:
		return h.handleRefundProcessed(e)
	default:
		// Unknown event type, log it but don't error
		fmt.Printf("Unknown webhook event type: %T\n", event)
		return nil
	}
}

// Generic event handlers (provider-agnostic)

func (h *WebhookHandler) handleCheckoutCompleted(event events.CheckoutCompletedEvent) error {
	// Find the purchase by session ID
	purchase, err := h.purchaseService.GetBySessionID(event.SessionID)
	if err != nil {
		return fmt.Errorf("purchase not found for session %s: %w", event.SessionID, err)
	}

	// Extract tax information
	var taxItems []models.TaxItem
	if event.TaxAmount > 0 {
		taxItems = append(taxItems, models.TaxItem{
			Amount:      event.TaxAmount,
			Description: "Tax",
		})
	}

	// Update purchase status based on approval requirements
	status := models.PurchaseStatusPaid
	if purchase.RequiresApproval {
		status = models.PurchaseStatusPaidPendingApproval
	}

	// Update purchase with payment details
	updates := map[string]interface{}{
		"status":                     status,
		"provider_payment_intent_id": event.PaymentIntentID,
		"tax_cents":                  event.TaxAmount,
		"tax_items":                  taxItems,
		"total_cents":                event.AmountTotal,
	}

	// Add customer information if available
	if event.CustomerEmail != "" {
		updates["customer_email"] = event.CustomerEmail
	}
	if event.CustomerName != "" {
		updates["customer_name"] = event.CustomerName
	}

	return h.purchaseService.UpdateStatus(purchase.ID, status, updates)
}

func (h *WebhookHandler) handleCheckoutExpired(event events.CheckoutExpiredEvent) error {
	// Find and cancel the purchase
	purchase, err := h.purchaseService.GetBySessionID(event.SessionID)
	if err != nil {
		// Purchase not found, might have been already processed
		return nil
	}

	// Only cancel if still pending
	if purchase.Status == models.PurchaseStatusPending {
		return h.purchaseService.Cancel(purchase.ID, "Checkout session expired")
	}

	return nil
}

func (h *WebhookHandler) handlePaymentSucceeded(event events.PaymentSucceededEvent) error {
	// Find purchase by payment intent ID
	purchase, err := h.purchaseService.GetByPaymentIntentID(event.PaymentIntentID)
	if err != nil {
		// No purchase found for this payment intent
		return nil
	}

	// Update status if not already paid
	if purchase.Status != models.PurchaseStatusPaid && purchase.Status != models.PurchaseStatusPaidPendingApproval {
		status := models.PurchaseStatusPaid
		if purchase.RequiresApproval {
			status = models.PurchaseStatusPaidPendingApproval
		}
		return h.purchaseService.UpdateStatus(purchase.ID, status, nil)
	}

	return nil
}

func (h *WebhookHandler) handlePaymentFailed(event events.PaymentFailedEvent) error {
	// Find purchase by payment intent ID
	purchase, err := h.purchaseService.GetByPaymentIntentID(event.PaymentIntentID)
	if err != nil {
		// No purchase found for this payment intent
		return nil
	}

	// Cancel the purchase with failure reason
	reason := event.FailureReason
	if reason == "" {
		reason = "Payment failed"
	}

	return h.purchaseService.Cancel(purchase.ID, reason)
}

func (h *WebhookHandler) handleRefundProcessed(event events.RefundProcessedEvent) error {
	// Find purchase by payment intent ID
	purchase, err := h.purchaseService.GetByPaymentIntentID(event.PaymentIntentID)
	if err != nil {
		// No purchase found for this refund
		return nil
	}

	// Update purchase status to refunded
	return h.purchaseService.UpdateStatus(purchase.ID, models.PurchaseStatusRefunded, map[string]interface{}{
		"refund_amount": event.RefundAmount,
		"refunded_at":   apptime.NowTime(),
	})
}
