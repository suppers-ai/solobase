package events

// Generic webhook event types that providers should map to
type (
	// CheckoutCompletedEvent represents a successful checkout completion
	CheckoutCompletedEvent struct {
		SessionID           string
		PaymentIntentID     string
		CustomerEmail       string
		CustomerName        string
		AmountTotal         int64
		TaxAmount           int64
		Currency            string
		Metadata            map[string]string
	}

	// CheckoutExpiredEvent represents an expired checkout session
	CheckoutExpiredEvent struct {
		SessionID string
		Metadata  map[string]string
	}

	// PaymentSucceededEvent represents a successful payment
	PaymentSucceededEvent struct {
		PaymentIntentID string
		Amount          int64
		Currency        string
		Metadata        map[string]string
	}

	// PaymentFailedEvent represents a failed payment
	PaymentFailedEvent struct {
		PaymentIntentID string
		FailureReason   string
		FailureCode     string
		Metadata        map[string]string
	}

	// RefundProcessedEvent represents a processed refund
	RefundProcessedEvent struct {
		PaymentIntentID string
		RefundAmount    int64
		RefundID        string
		Reason          string
		Metadata        map[string]string
	}
)