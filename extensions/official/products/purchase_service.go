package products

import (
	"database/sql"
	"encoding/json"
	"fmt"

	"github.com/suppers-ai/solobase/extensions/official/products/models"
	"github.com/suppers-ai/solobase/extensions/official/products/providers"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

// PurchaseService handles purchase operations
type PurchaseService struct {
	db              *sql.DB
	productService  *ProductService
	pricingService  *PricingService
	paymentProvider providers.PaymentProvider // Generic payment provider interface
}

// NewPurchaseService creates a new purchase service
func NewPurchaseService(db *sql.DB, productService *ProductService, pricingService *PricingService, provider providers.PaymentProvider) *PurchaseService {
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
}

// PurchaseRequestItem represents a single item in a purchase request
type PurchaseRequestItem struct {
	ProductID uint                   `json:"productId"`
	Quantity  int                    `json:"quantity"`
	Variables map[string]interface{} `json:"variables"`
}

// Create creates a new purchase and optionally initiates checkout
func (s *PurchaseService) Create(req *PurchaseRequest) (*models.Purchase, error) {
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

	// Marshal JSON fields
	lineItemsJSON, _ := json.Marshal(lineItems)
	metadataJSON, _ := json.Marshal(req.Metadata)
	paymentMethodTypesJSON, _ := json.Marshal(req.PaymentMethodTypes)

	now := apptime.NowTime()

	// Insert purchase
	result, err := s.db.Exec(`
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

	id, err := result.LastInsertId()
	if err != nil {
		return nil, fmt.Errorf("failed to get purchase ID: %w", err)
	}

	purchase := &models.Purchase{
		ID:                 uint(id),
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
	}

	// If not requiring approval, create checkout session with payment provider
	if !req.RequiresApproval && s.paymentProvider != nil && s.paymentProvider.IsEnabled() {
		sessionID, err := s.paymentProvider.CreateCheckoutSession(purchase)
		if err != nil {
			// Update purchase status
			s.db.Exec(`UPDATE ext_products_purchases SET status = ?, cancel_reason = ?, updated_at = ? WHERE id = ?`,
				models.PurchaseStatusCancelled, "Failed to create checkout session", apptime.NowTime(), id)
			return nil, fmt.Errorf("failed to create checkout session: %w", err)
		}

		// Update purchase with session ID
		_, err = s.db.Exec(`UPDATE ext_products_purchases SET provider_session_id = ?, updated_at = ? WHERE id = ?`,
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
	row := s.db.QueryRow(`
		SELECT id, user_id, provider, line_items, product_metadata, amount_cents, total_cents,
			tax_cents, tax_items, currency, status, requires_approval, approved_at, approved_by,
			provider_session_id, provider_payment_intent_id, provider_subscription_id,
			success_url, cancel_url, customer_email, payment_method_types,
			refunded_at, refund_amount, refund_reason, cancelled_at, cancel_reason,
			created_at, updated_at
		FROM ext_products_purchases WHERE id = ?`, id)

	return s.scanPurchase(row)
}

// GetByUserID retrieves purchases for a specific user
func (s *PurchaseService) GetByUserID(userID string, limit, offset int) ([]models.Purchase, int64, error) {
	// Count total
	var total int64
	err := s.db.QueryRow(`SELECT COUNT(*) FROM ext_products_purchases WHERE user_id = ?`, userID).Scan(&total)
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

	rows, err := s.db.Query(query, userID)
	if err != nil {
		return nil, 0, err
	}
	defer rows.Close()

	var purchases []models.Purchase
	for rows.Next() {
		purchase, err := s.scanPurchaseFromRows(rows)
		if err != nil {
			return nil, 0, err
		}
		purchases = append(purchases, *purchase)
	}

	return purchases, total, nil
}

// ListAll retrieves all purchases (admin function)
func (s *PurchaseService) ListAll(limit, offset int) ([]models.Purchase, int64, error) {
	// Count total
	var total int64
	err := s.db.QueryRow(`SELECT COUNT(*) FROM ext_products_purchases`).Scan(&total)
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

	rows, err := s.db.Query(query)
	if err != nil {
		return nil, 0, err
	}
	defer rows.Close()

	var purchases []models.Purchase
	for rows.Next() {
		purchase, err := s.scanPurchaseFromRows(rows)
		if err != nil {
			return nil, 0, err
		}
		purchases = append(purchases, *purchase)
	}

	return purchases, total, nil
}

// GetBySessionID retrieves a purchase by provider session ID
func (s *PurchaseService) GetBySessionID(sessionID string) (*models.Purchase, error) {
	row := s.db.QueryRow(`
		SELECT id, user_id, provider, line_items, product_metadata, amount_cents, total_cents,
			tax_cents, tax_items, currency, status, requires_approval, approved_at, approved_by,
			provider_session_id, provider_payment_intent_id, provider_subscription_id,
			success_url, cancel_url, customer_email, payment_method_types,
			refunded_at, refund_amount, refund_reason, cancelled_at, cancel_reason,
			created_at, updated_at
		FROM ext_products_purchases WHERE provider_session_id = ?`, sessionID)

	return s.scanPurchase(row)
}

// GetByPaymentIntentID retrieves a purchase by provider payment intent ID
func (s *PurchaseService) GetByPaymentIntentID(paymentIntentID string) (*models.Purchase, error) {
	row := s.db.QueryRow(`
		SELECT id, user_id, provider, line_items, product_metadata, amount_cents, total_cents,
			tax_cents, tax_items, currency, status, requires_approval, approved_at, approved_by,
			provider_session_id, provider_payment_intent_id, provider_subscription_id,
			success_url, cancel_url, customer_email, payment_method_types,
			refunded_at, refund_amount, refund_reason, cancelled_at, cancel_reason,
			created_at, updated_at
		FROM ext_products_purchases WHERE provider_payment_intent_id = ?`, paymentIntentID)

	return s.scanPurchase(row)
}

// UpdateStatus updates the status of a purchase
func (s *PurchaseService) UpdateStatus(id uint, status string, metadata map[string]interface{}) error {
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

	_, err := s.db.Exec(query, args...)
	return err
}

// Approve approves a purchase that requires approval
func (s *PurchaseService) Approve(id uint, approverID uint) error {
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
		_, err = s.db.Exec(`UPDATE ext_products_purchases SET approved_at = ?, approved_by = ?, status = ?, updated_at = ? WHERE id = ?`,
			now, approverID, models.PurchaseStatusPaid, now, id)
		return err
	}

	// Create checkout session if approval granted before payment
	if s.paymentProvider != nil && s.paymentProvider.IsEnabled() {
		sessionID, err := s.paymentProvider.CreateCheckoutSession(purchase)
		if err != nil {
			return fmt.Errorf("failed to create checkout session: %w", err)
		}
		_, err = s.db.Exec(`UPDATE ext_products_purchases SET approved_at = ?, approved_by = ?, provider_session_id = ?, status = ?, updated_at = ? WHERE id = ?`,
			now, approverID, sessionID, models.PurchaseStatusPending, now, id)
		return err
	}

	_, err = s.db.Exec(`UPDATE ext_products_purchases SET approved_at = ?, approved_by = ?, updated_at = ? WHERE id = ?`,
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

	// Cancel checkout session with payment provider if exists
	if s.paymentProvider != nil && s.paymentProvider.IsEnabled() && purchase.ProviderSessionID != "" {
		if err := s.paymentProvider.ExpireCheckoutSession(purchase.ProviderSessionID); err != nil {
			// Log error but continue with cancellation
			fmt.Printf("Warning: Failed to expire checkout session: %v\n", err)
		}
	}

	// Update purchase status
	return s.UpdateStatus(id, models.PurchaseStatusCancelled, map[string]interface{}{
		"reason": reason,
	})
}

// GetStats retrieves purchase statistics for a user
func (s *PurchaseService) GetStats(userID string) (map[string]interface{}, error) {
	stats := map[string]interface{}{
		"totalPurchases": 0,
		"totalSpent":     int64(0),
		"pending":        0,
		"completed":      0,
		"refunded":       0,
	}

	// Total purchases
	var totalPurchases int64
	s.db.QueryRow(`SELECT COUNT(*) FROM ext_products_purchases WHERE user_id = ?`, userID).Scan(&totalPurchases)
	stats["totalPurchases"] = totalPurchases

	// Total spent (paid purchases)
	var totalSpent sql.NullInt64
	s.db.QueryRow(`SELECT COALESCE(SUM(total_cents), 0) FROM ext_products_purchases WHERE user_id = ? AND status IN (?, ?)`,
		userID, models.PurchaseStatusPaid, models.PurchaseStatusRefunded).Scan(&totalSpent)
	if totalSpent.Valid {
		stats["totalSpent"] = totalSpent.Int64
	}

	// Status counts
	rows, err := s.db.Query(`SELECT status, COUNT(*) FROM ext_products_purchases WHERE user_id = ? GROUP BY status`, userID)
	if err == nil {
		defer rows.Close()
		for rows.Next() {
			var status string
			var count int64
			if err := rows.Scan(&status, &count); err == nil {
				switch status {
				case models.PurchaseStatusPending, models.PurchaseStatusRequiresApproval:
					stats["pending"] = stats["pending"].(int) + int(count)
				case models.PurchaseStatusPaid, models.PurchaseStatusPaidPendingApproval:
					stats["completed"] = stats["completed"].(int) + int(count)
				case models.PurchaseStatusRefunded:
					stats["refunded"] = int(count)
				}
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

// Helper functions for scanning purchases

func (s *PurchaseService) scanPurchase(row *sql.Row) (*models.Purchase, error) {
	var p models.Purchase
	var lineItemsJSON, metadataJSON, taxItemsJSON, paymentMethodTypesJSON []byte
	var approvedAt, refundedAt, cancelledAt sql.NullTime
	var approvedBy, refundAmount, taxCents sql.NullInt64
	var refundReason, cancelReason sql.NullString

	err := row.Scan(
		&p.ID, &p.UserID, &p.Provider, &lineItemsJSON, &metadataJSON, &p.AmountCents, &p.TotalCents,
		&taxCents, &taxItemsJSON, &p.Currency, &p.Status, &p.RequiresApproval, &approvedAt, &approvedBy,
		&p.ProviderSessionID, &p.ProviderPaymentIntentID, &p.ProviderSubscriptionID,
		&p.SuccessURL, &p.CancelURL, &p.CustomerEmail, &paymentMethodTypesJSON,
		&refundedAt, &refundAmount, &refundReason, &cancelledAt, &cancelReason,
		&p.CreatedAt, &p.UpdatedAt,
	)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, fmt.Errorf("purchase not found")
		}
		return nil, err
	}

	// Unmarshal JSON fields
	json.Unmarshal(lineItemsJSON, &p.LineItems)
	json.Unmarshal(metadataJSON, &p.ProductMetadata)
	json.Unmarshal(taxItemsJSON, &p.TaxItems)
	json.Unmarshal(paymentMethodTypesJSON, &p.PaymentMethodTypes)

	// Handle nullable fields
	if approvedAt.Valid {
		p.ApprovedAt = apptime.NewNullTime(approvedAt.Time)
	}
	if approvedBy.Valid {
		val := fmt.Sprintf("%d", approvedBy.Int64)
		p.ApprovedBy = &val
	}
	if taxCents.Valid {
		p.TaxCents = taxCents.Int64
	}
	if refundedAt.Valid {
		p.RefundedAt = apptime.NewNullTime(refundedAt.Time)
	}
	if refundAmount.Valid {
		p.RefundAmount = refundAmount.Int64
	}
	if refundReason.Valid {
		p.RefundReason = refundReason.String
	}
	if cancelledAt.Valid {
		p.CancelledAt = apptime.NewNullTime(cancelledAt.Time)
	}
	if cancelReason.Valid {
		p.CancelReason = cancelReason.String
	}

	return &p, nil
}

func (s *PurchaseService) scanPurchaseFromRows(rows *sql.Rows) (*models.Purchase, error) {
	var p models.Purchase
	var lineItemsJSON, metadataJSON, taxItemsJSON, paymentMethodTypesJSON []byte
	var approvedAt, refundedAt, cancelledAt sql.NullTime
	var approvedBy, refundAmount, taxCents sql.NullInt64
	var refundReason, cancelReason sql.NullString

	err := rows.Scan(
		&p.ID, &p.UserID, &p.Provider, &lineItemsJSON, &metadataJSON, &p.AmountCents, &p.TotalCents,
		&taxCents, &taxItemsJSON, &p.Currency, &p.Status, &p.RequiresApproval, &approvedAt, &approvedBy,
		&p.ProviderSessionID, &p.ProviderPaymentIntentID, &p.ProviderSubscriptionID,
		&p.SuccessURL, &p.CancelURL, &p.CustomerEmail, &paymentMethodTypesJSON,
		&refundedAt, &refundAmount, &refundReason, &cancelledAt, &cancelReason,
		&p.CreatedAt, &p.UpdatedAt,
	)
	if err != nil {
		return nil, err
	}

	// Unmarshal JSON fields
	json.Unmarshal(lineItemsJSON, &p.LineItems)
	json.Unmarshal(metadataJSON, &p.ProductMetadata)
	json.Unmarshal(taxItemsJSON, &p.TaxItems)
	json.Unmarshal(paymentMethodTypesJSON, &p.PaymentMethodTypes)

	// Handle nullable fields
	if approvedAt.Valid {
		p.ApprovedAt = apptime.NewNullTime(approvedAt.Time)
	}
	if approvedBy.Valid {
		val := fmt.Sprintf("%d", approvedBy.Int64)
		p.ApprovedBy = &val
	}
	if taxCents.Valid {
		p.TaxCents = taxCents.Int64
	}
	if refundedAt.Valid {
		p.RefundedAt = apptime.NewNullTime(refundedAt.Time)
	}
	if refundAmount.Valid {
		p.RefundAmount = refundAmount.Int64
	}
	if refundReason.Valid {
		p.RefundReason = refundReason.String
	}
	if cancelledAt.Valid {
		p.CancelledAt = apptime.NewNullTime(cancelledAt.Time)
	}
	if cancelReason.Valid {
		p.CancelReason = cancelReason.String
	}

	return &p, nil
}
