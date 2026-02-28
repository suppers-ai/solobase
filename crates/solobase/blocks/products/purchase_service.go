package products

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/suppers-ai/solobase/blocks/products/models"
	"github.com/suppers-ai/solobase/blocks/products/providers"
	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/waffle-go/services/database"
)

// PurchaseService handles purchase operations
type PurchaseService struct {
	db              database.Service
	productService  *ProductService
	pricingService  *PricingService
	paymentProvider providers.PaymentProvider // Generic payment provider interface
}

// NewPurchaseService creates a new purchase service
func NewPurchaseService(db database.Service, productService *ProductService, pricingService *PricingService, provider providers.PaymentProvider) *PurchaseService {
	return &PurchaseService{
		db:              db,
		productService:  productService,
		pricingService:  pricingService,
		paymentProvider: provider,
	}
}

// PurchaseRequest represents a request to create a purchase
type PurchaseRequest struct {
	UserID   string                 `json:"userId"`
	Items    []PurchaseRequestItem  `json:"items"`
	Metadata map[string]interface{} `json:"metadata"`
	// Checkout configuration
	SuccessURL         string   `json:"successUrl"`
	CancelURL          string   `json:"cancelUrl"`
	CustomerEmail      string   `json:"customerEmail"`
	PaymentMethodTypes []string `json:"paymentMethodTypes"`
	RequiresApproval   bool     `json:"requiresApproval"`

	// Stripe Connect configuration
	StripeConnectAccountID string  `json:"stripeConnectAccountId,omitempty"` // Connected account ID
	PlatformFeePercent     float64 `json:"platformFeePercent,omitempty"`     // Platform fee percentage (e.g., 3.0 for 3%)
	PlatformFeeCents       int64   `json:"platformFeeCents,omitempty"`       // Fixed platform fee in cents (overrides percent)
}

// PurchaseRequestItem represents a single item in a purchase request
type PurchaseRequestItem struct {
	ProductID uint                   `json:"productId"`
	Quantity  int                    `json:"quantity"`
	Variables map[string]interface{} `json:"variables"`
}

// Create creates a new purchase and optionally initiates checkout
func (s *PurchaseService) Create(req *PurchaseRequest) (*models.Purchase, error) {
	ctx := context.Background()
	var lineItems []models.LineItem
	var totalAmountCents int64

	// Calculate prices for all items
	for _, item := range req.Items {
		// Get product
		product, err := s.productService.GetByID(item.ProductID)
		if err != nil {
			return nil, fmt.Errorf("product %d not found: %w", item.ProductID, err)
		}

		// Calculate price
		price, err := s.pricingService.CalculatePrice(item.ProductID, item.Variables)
		if err != nil {
			return nil, fmt.Errorf("failed to calculate price for product %d: %w", item.ProductID, err)
		}

		// Convert price to cents (assuming price is in dollars)
		unitPriceCents := int64(price * 100)
		totalPriceCents := unitPriceCents * int64(item.Quantity)

		lineItems = append(lineItems, models.LineItem{
			ProductID:   item.ProductID,
			ProductName: product.Name,
			Quantity:    item.Quantity,
			UnitPrice:   unitPriceCents,
			TotalPrice:  totalPriceCents,
			Variables:   item.Variables,
			Description: product.Description,
		})

		totalAmountCents += totalPriceCents
	}

	// Determine payment provider
	providerName := "stripe" // Default to stripe for backward compatibility
	if s.paymentProvider != nil {
		providerName = s.paymentProvider.GetProviderName()
	}

	// Determine initial status
	status := models.PurchaseStatusPending
	if req.RequiresApproval {
		status = models.PurchaseStatusRequiresApproval
	}

	// Calculate platform fee for Stripe Connect
	var platformFeeCents int64
	if req.PlatformFeeCents > 0 {
		// Use fixed fee if specified
		platformFeeCents = req.PlatformFeeCents
	} else if req.PlatformFeePercent > 0 {
		// Calculate from percentage
		platformFeeCents = int64(float64(totalAmountCents) * req.PlatformFeePercent / 100.0)
	}

	// Marshal JSON fields
	lineItemsJSON, _ := json.Marshal(lineItems)
	metadataJSON, _ := json.Marshal(req.Metadata)
	paymentMethodTypesJSON, _ := json.Marshal(req.PaymentMethodTypes)

	now := apptime.NowTime()

	// Insert purchase
	_, err := s.db.ExecRaw(ctx, `
		INSERT INTO ext_products_purchases (
			user_id, provider, line_items, product_metadata, amount_cents, total_cents,
			currency, status, requires_approval, success_url, cancel_url,
			customer_email, payment_method_types, created_at, updated_at
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		req.UserID, providerName, lineItemsJSON, metadataJSON, totalAmountCents, totalAmountCents,
		"USD", status, req.RequiresApproval, req.SuccessURL, req.CancelURL,
		req.CustomerEmail, paymentMethodTypesJSON, now, now)
	if err != nil {
		return nil, fmt.Errorf("failed to create purchase: %w", err)
	}

	id, err := getLastInsertedID(ctx, s.db, "ext_products_purchases")
	if err != nil {
		return nil, fmt.Errorf("failed to get purchase ID: %w", err)
	}

	purchase := &models.Purchase{
		ID:                 id,
		UserID:             req.UserID,
		Provider:           providerName,
		LineItems:          lineItems,
		ProductMetadata:    models.JSONB(req.Metadata),
		AmountCents:        totalAmountCents,
		TotalCents:         totalAmountCents,
		Currency:           "USD",
		Status:             status,
		RequiresApproval:   req.RequiresApproval,
		SuccessURL:         req.SuccessURL,
		CancelURL:          req.CancelURL,
		CustomerEmail:      req.CustomerEmail,
		PaymentMethodTypes: req.PaymentMethodTypes,
		CreatedAt:          apptime.NewTime(now),
		UpdatedAt:          apptime.NewTime(now),

		// Stripe Connect fields
		StripeConnectAccountID: req.StripeConnectAccountID,
		PlatformFeeCents:       platformFeeCents,
	}

	// If not requiring approval, create checkout session with payment provider
	if !req.RequiresApproval && s.paymentProvider != nil && s.paymentProvider.IsEnabled() {
		sessionID, err := s.paymentProvider.CreateCheckoutSession(purchase)
		if err != nil {
			// Update purchase status
			s.db.ExecRaw(ctx, `UPDATE ext_products_purchases SET status = ?, cancel_reason = ?, updated_at = ? WHERE id = ?`,
				models.PurchaseStatusCancelled, "Failed to create checkout session", apptime.NowTime(), id)
			return nil, fmt.Errorf("failed to create checkout session: %w", err)
		}

		// Update purchase with session ID
		_, err = s.db.ExecRaw(ctx, `UPDATE ext_products_purchases SET provider_session_id = ?, updated_at = ? WHERE id = ?`,
			sessionID, apptime.NowTime(), id)
		if err != nil {
			return nil, fmt.Errorf("failed to update purchase with session ID: %w", err)
		}
		purchase.ProviderSessionID = sessionID
	}

	return purchase, nil
}

// GetByID retrieves a purchase by ID
func (s *PurchaseService) GetByID(id uint) (*models.Purchase, error) {
	ctx := context.Background()
	records, err := s.db.QueryRaw(ctx, `
		SELECT id, user_id, provider, line_items, product_metadata, amount_cents, total_cents,
			tax_cents, tax_items, currency, status, requires_approval, approved_at, approved_by,
			provider_session_id, provider_payment_intent_id, provider_subscription_id,
			success_url, cancel_url, customer_email, payment_method_types,
			refunded_at, refund_amount, refund_reason, cancelled_at, cancel_reason,
			created_at, updated_at
		FROM ext_products_purchases WHERE id = ?`, id)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, fmt.Errorf("purchase not found")
	}
	return recordToPurchase(records[0]), nil
}

// GetByUserID retrieves purchases for a specific user
func (s *PurchaseService) GetByUserID(userID string, limit, offset int) ([]models.Purchase, int64, error) {
	ctx := context.Background()

	// Count total
	total, err := s.db.Count(ctx, "ext_products_purchases", []database.Filter{
		{Field: "user_id", Operator: database.OpEqual, Value: userID},
	})
	if err != nil {
		return nil, 0, err
	}

	// Fetch purchases
	query := `
		SELECT id, user_id, provider, line_items, product_metadata, amount_cents, total_cents,
			tax_cents, tax_items, currency, status, requires_approval, approved_at, approved_by,
			provider_session_id, provider_payment_intent_id, provider_subscription_id,
			success_url, cancel_url, customer_email, payment_method_types,
			refunded_at, refund_amount, refund_reason, cancelled_at, cancel_reason,
			created_at, updated_at
		FROM ext_products_purchases WHERE user_id = ? ORDER BY created_at DESC`

	if limit > 0 {
		query += fmt.Sprintf(" LIMIT %d OFFSET %d", limit, offset)
	}

	records, err := s.db.QueryRaw(ctx, query, userID)
	if err != nil {
		return nil, 0, err
	}

	var purchases []models.Purchase
	for _, r := range records {
		purchases = append(purchases, *recordToPurchase(r))
	}

	return purchases, int64(total), nil
}

// ListAll retrieves all purchases (admin function)
func (s *PurchaseService) ListAll(limit, offset int) ([]models.Purchase, int64, error) {
	ctx := context.Background()

	// Count total
	total, err := s.db.Count(ctx, "ext_products_purchases", nil)
	if err != nil {
		return nil, 0, err
	}

	// Fetch purchases
	query := `
		SELECT id, user_id, provider, line_items, product_metadata, amount_cents, total_cents,
			tax_cents, tax_items, currency, status, requires_approval, approved_at, approved_by,
			provider_session_id, provider_payment_intent_id, provider_subscription_id,
			success_url, cancel_url, customer_email, payment_method_types,
			refunded_at, refund_amount, refund_reason, cancelled_at, cancel_reason,
			created_at, updated_at
		FROM ext_products_purchases ORDER BY created_at DESC`

	if limit > 0 {
		query += fmt.Sprintf(" LIMIT %d OFFSET %d", limit, offset)
	}

	records, err := s.db.QueryRaw(ctx, query)
	if err != nil {
		return nil, 0, err
	}

	var purchases []models.Purchase
	for _, r := range records {
		purchases = append(purchases, *recordToPurchase(r))
	}

	return purchases, int64(total), nil
}

// GetBySessionID retrieves a purchase by provider session ID
func (s *PurchaseService) GetBySessionID(sessionID string) (*models.Purchase, error) {
	ctx := context.Background()
	records, err := s.db.QueryRaw(ctx, `
		SELECT id, user_id, provider, line_items, product_metadata, amount_cents, total_cents,
			tax_cents, tax_items, currency, status, requires_approval, approved_at, approved_by,
			provider_session_id, provider_payment_intent_id, provider_subscription_id,
			success_url, cancel_url, customer_email, payment_method_types,
			refunded_at, refund_amount, refund_reason, cancelled_at, cancel_reason,
			created_at, updated_at
		FROM ext_products_purchases WHERE provider_session_id = ?`, sessionID)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, fmt.Errorf("purchase not found")
	}
	return recordToPurchase(records[0]), nil
}

// GetByPaymentIntentID retrieves a purchase by provider payment intent ID
func (s *PurchaseService) GetByPaymentIntentID(paymentIntentID string) (*models.Purchase, error) {
	ctx := context.Background()
	records, err := s.db.QueryRaw(ctx, `
		SELECT id, user_id, provider, line_items, product_metadata, amount_cents, total_cents,
			tax_cents, tax_items, currency, status, requires_approval, approved_at, approved_by,
			provider_session_id, provider_payment_intent_id, provider_subscription_id,
			success_url, cancel_url, customer_email, payment_method_types,
			refunded_at, refund_amount, refund_reason, cancelled_at, cancel_reason,
			created_at, updated_at
		FROM ext_products_purchases WHERE provider_payment_intent_id = ?`, paymentIntentID)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, fmt.Errorf("purchase not found")
	}
	return recordToPurchase(records[0]), nil
}

// UpdateStatus updates the status of a purchase
func (s *PurchaseService) UpdateStatus(id uint, status string, metadata map[string]interface{}) error {
	ctx := context.Background()
	now := apptime.NowTime()

	// Build update query dynamically based on status and metadata
	query := `UPDATE ext_products_purchases SET status = ?, updated_at = ?`
	args := []interface{}{status, now}

	// Add status-specific timestamps
	switch status {
	case models.PurchaseStatusRefunded:
		query += `, refunded_at = ?`
		args = append(args, now)
		if reason, ok := metadata["reason"].(string); ok {
			query += `, refund_reason = ?`
			args = append(args, reason)
		}
		if amount, ok := metadata["amount"].(int64); ok {
			query += `, refund_amount = ?`
			args = append(args, amount)
		}
	case models.PurchaseStatusCancelled:
		query += `, cancelled_at = ?`
		args = append(args, now)
		if reason, ok := metadata["reason"].(string); ok {
			query += `, cancel_reason = ?`
			args = append(args, reason)
		}
	}

	// Add provider-specific IDs if provided
	if paymentIntentID, ok := metadata["payment_intent_id"].(string); ok && paymentIntentID != "" {
		query += `, provider_payment_intent_id = ?`
		args = append(args, paymentIntentID)
	}
	if subscriptionID, ok := metadata["subscription_id"].(string); ok && subscriptionID != "" {
		query += `, provider_subscription_id = ?`
		args = append(args, subscriptionID)
	}

	// Update tax information if provided
	if taxItems, ok := metadata["tax_items"].([]models.TaxItem); ok {
		taxItemsJSON, _ := json.Marshal(taxItems)
		query += `, tax_items = ?`
		args = append(args, taxItemsJSON)
	}
	if taxCents, ok := metadata["tax_cents"].(int64); ok {
		query += `, tax_cents = ?`
		args = append(args, taxCents)
	}
	if totalCents, ok := metadata["total_cents"].(int64); ok {
		query += `, total_cents = ?`
		args = append(args, totalCents)
	}

	query += ` WHERE id = ?`
	args = append(args, id)

	_, err := s.db.ExecRaw(ctx, query, args...)
	return err
}

// Approve approves a purchase that requires approval
func (s *PurchaseService) Approve(id uint, approverID string) error {
	ctx := context.Background()
	purchase, err := s.GetByID(id)
	if err != nil {
		return fmt.Errorf("purchase not found: %w", err)
	}

	if !purchase.NeedsApproval() {
		return fmt.Errorf("purchase does not require approval")
	}

	now := apptime.NowTime()

	// If payment was already made, mark as paid
	if purchase.Status == models.PurchaseStatusPaidPendingApproval {
		_, err = s.db.ExecRaw(ctx, `UPDATE ext_products_purchases SET approved_at = ?, approved_by = ?, status = ?, updated_at = ? WHERE id = ?`,
			now, approverID, models.PurchaseStatusPaid, now, id)
		return err
	}

	// Create checkout session if approval granted before payment
	if s.paymentProvider != nil && s.paymentProvider.IsEnabled() {
		sessionID, err := s.paymentProvider.CreateCheckoutSession(purchase)
		if err != nil {
			return fmt.Errorf("failed to create checkout session: %w", err)
		}
		_, err = s.db.ExecRaw(ctx, `UPDATE ext_products_purchases SET approved_at = ?, approved_by = ?, provider_session_id = ?, status = ?, updated_at = ? WHERE id = ?`,
			now, approverID, sessionID, models.PurchaseStatusPending, now, id)
		return err
	}

	_, err = s.db.ExecRaw(ctx, `UPDATE ext_products_purchases SET approved_at = ?, approved_by = ?, updated_at = ? WHERE id = ?`,
		now, approverID, now, id)
	return err
}

// Refund initiates a refund for a purchase
func (s *PurchaseService) Refund(id uint, amount int64, reason string) error {
	purchase, err := s.GetByID(id)
	if err != nil {
		return err
	}

	if !purchase.CanRefund() {
		return fmt.Errorf("purchase cannot be refunded")
	}

	// Process refund with payment provider
	if s.paymentProvider != nil && s.paymentProvider.IsEnabled() && purchase.ProviderPaymentIntentID != "" {
		if err := s.paymentProvider.RefundPayment(purchase.ProviderPaymentIntentID, amount, reason); err != nil {
			return fmt.Errorf("failed to process refund: %w", err)
		}
	}

	// Update purchase status
	return s.UpdateStatus(id, models.PurchaseStatusRefunded, map[string]interface{}{
		"reason": reason,
		"amount": amount,
	})
}

// Cancel cancels a pending purchase
func (s *PurchaseService) Cancel(id uint, reason string) error {
	purchase, err := s.GetByID(id)
	if err != nil {
		return err
	}

	if !purchase.CanCancel() {
		return fmt.Errorf("purchase cannot be cancelled")
	}

	// Cancel checkout session with payment provider if exists (ignore errors)
	if s.paymentProvider != nil && s.paymentProvider.IsEnabled() && purchase.ProviderSessionID != "" {
		_ = s.paymentProvider.ExpireCheckoutSession(purchase.ProviderSessionID)
	}

	// Update purchase status
	return s.UpdateStatus(id, models.PurchaseStatusCancelled, map[string]interface{}{
		"reason": reason,
	})
}

// GetStats retrieves purchase statistics for a user
func (s *PurchaseService) GetStats(userID string) (map[string]interface{}, error) {
	ctx := context.Background()
	stats := map[string]interface{}{
		"totalPurchases": 0,
		"totalSpent":     int64(0),
		"pending":        0,
		"completed":      0,
		"refunded":       0,
	}

	// Total purchases
	totalPurchases, _ := s.db.Count(ctx, "ext_products_purchases", []database.Filter{
		{Field: "user_id", Operator: database.OpEqual, Value: userID},
	})
	stats["totalPurchases"] = totalPurchases

	// Total spent (paid purchases) -- use QueryRaw for the complex query
	spentRecords, err := s.db.QueryRaw(ctx,
		`SELECT COALESCE(SUM(total_cents), 0) as total_spent FROM ext_products_purchases WHERE user_id = ? AND status IN (?, ?)`,
		userID, models.PurchaseStatusPaid, models.PurchaseStatusRefunded)
	if err == nil && len(spentRecords) > 0 {
		stats["totalSpent"] = toInt64Val(spentRecords[0].Data["total_spent"])
	}

	// Status counts
	statusRecords, err := s.db.QueryRaw(ctx,
		`SELECT status, COUNT(*) as cnt FROM ext_products_purchases WHERE user_id = ? GROUP BY status`, userID)
	if err == nil {
		for _, r := range statusRecords {
			status := stringVal(r.Data["status"])
			count := int(toInt64Val(r.Data["cnt"]))
			switch status {
			case models.PurchaseStatusPending, models.PurchaseStatusRequiresApproval:
				stats["pending"] = stats["pending"].(int) + count
			case models.PurchaseStatusPaid, models.PurchaseStatusPaidPendingApproval:
				stats["completed"] = stats["completed"].(int) + count
			case models.PurchaseStatusRefunded:
				stats["refunded"] = count
			}
		}
	}

	return stats, nil
}

// GetCheckoutURL returns the checkout URL for a purchase
func (s *PurchaseService) GetCheckoutURL(purchase *models.Purchase) string {
	if s.paymentProvider == nil || purchase.ProviderSessionID == "" {
		return ""
	}

	// Determine provider from purchase (default to current provider)
	provider := purchase.Provider
	if provider == "" {
		provider = s.paymentProvider.GetProviderName()
	}

	// For now, only handle the configured provider
	if provider == s.paymentProvider.GetProviderName() {
		return s.paymentProvider.GetCheckoutURL(purchase.ProviderSessionID)
	}

	return ""
}
