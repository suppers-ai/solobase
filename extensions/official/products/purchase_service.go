package products

import (
	"errors"
	"fmt"
	"time"

	"github.com/suppers-ai/solobase/extensions/official/products/models"
	"github.com/suppers-ai/solobase/extensions/official/products/providers"
	"gorm.io/gorm"
)

// PurchaseService handles purchase operations
type PurchaseService struct {
	db              *gorm.DB
	productService  *ProductService
	pricingService  *PricingService
	paymentProvider providers.PaymentProvider // Generic payment provider interface
}

// NewPurchaseService creates a new purchase service
func NewPurchaseService(db *gorm.DB, productService *ProductService, pricingService *PricingService, provider providers.PaymentProvider) *PurchaseService {
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

	// Create purchase record
	purchase := &models.Purchase{
		UserID:             req.UserID,
		Provider:           providerName,
		LineItems:          lineItems,
		ProductMetadata:    models.JSONB(req.Metadata),
		AmountCents:        totalAmountCents,
		TotalCents:         totalAmountCents, // Will be updated with tax later
		Currency:           "USD",
		Status:             models.PurchaseStatusPending,
		RequiresApproval:   req.RequiresApproval,
		SuccessURL:         req.SuccessURL,
		CancelURL:          req.CancelURL,
		CustomerEmail:      req.CustomerEmail,
		PaymentMethodTypes: req.PaymentMethodTypes,
	}

	// If approval is required, set appropriate status
	if req.RequiresApproval {
		purchase.Status = models.PurchaseStatusRequiresApproval
	}

	// Save purchase
	if err := s.db.Create(purchase).Error; err != nil {
		return nil, fmt.Errorf("failed to create purchase: %w", err)
	}

	// If not requiring approval, create checkout session with payment provider
	if !req.RequiresApproval && s.paymentProvider != nil && s.paymentProvider.IsEnabled() {
		sessionID, err := s.paymentProvider.CreateCheckoutSession(purchase)
		if err != nil {
			// Update purchase status
			purchase.Status = models.PurchaseStatusCancelled
			purchase.CancelReason = "Failed to create checkout session"
			s.db.Save(purchase)
			return nil, fmt.Errorf("failed to create checkout session: %w", err)
		}

		// Update purchase with session ID
		purchase.ProviderSessionID = sessionID
		if err := s.db.Save(purchase).Error; err != nil {
			return nil, fmt.Errorf("failed to update purchase with session ID: %w", err)
		}
	}

	return purchase, nil
}

// GetByID retrieves a purchase by ID
func (s *PurchaseService) GetByID(id uint) (*models.Purchase, error) {
	var purchase models.Purchase
	if err := s.db.First(&purchase, id).Error; err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, fmt.Errorf("purchase not found")
		}
		return nil, err
	}
	return &purchase, nil
}

// GetByUserID retrieves purchases for a specific user
func (s *PurchaseService) GetByUserID(userID string, limit, offset int) ([]models.Purchase, int64, error) {
	var purchases []models.Purchase
	var total int64

	// Count total
	if err := s.db.Model(&models.Purchase{}).Where("user_id = ?", userID).Count(&total).Error; err != nil {
		return nil, 0, err
	}

	// Fetch purchases
	query := s.db.Where("user_id = ?", userID).Order("created_at DESC")
	if limit > 0 {
		query = query.Limit(limit).Offset(offset)
	}

	if err := query.Find(&purchases).Error; err != nil {
		return nil, 0, err
	}

	return purchases, total, nil
}

// GetBySessionID retrieves a purchase by Stripe session ID
func (s *PurchaseService) GetBySessionID(sessionID string) (*models.Purchase, error) {
	var purchase models.Purchase
	if err := s.db.Where("provider_session_id = ?", sessionID).First(&purchase).Error; err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, fmt.Errorf("purchase not found for session ID")
		}
		return nil, err
	}
	return &purchase, nil
}

// UpdateStatus updates the status of a purchase
func (s *PurchaseService) UpdateStatus(id uint, status string, metadata map[string]interface{}) error {
	updates := map[string]interface{}{
		"status":     status,
		"updated_at": time.Now(),
	}

	// Add status-specific timestamps
	switch status {
	case models.PurchaseStatusPaid:
		// Status transitions to paid
	case models.PurchaseStatusRefunded:
		updates["refunded_at"] = time.Now()
		if reason, ok := metadata["reason"].(string); ok {
			updates["refund_reason"] = reason
		}
		if amount, ok := metadata["amount"].(int64); ok {
			updates["refund_amount"] = amount
		}
	case models.PurchaseStatusCancelled:
		updates["cancelled_at"] = time.Now()
		if reason, ok := metadata["reason"].(string); ok {
			updates["cancel_reason"] = reason
		}
	}

	// Add provider-specific IDs if provided
	if paymentIntentID, ok := metadata["payment_intent_id"].(string); ok && paymentIntentID != "" {
		updates["provider_payment_intent_id"] = paymentIntentID
	}
	if subscriptionID, ok := metadata["subscription_id"].(string); ok && subscriptionID != "" {
		updates["provider_subscription_id"] = subscriptionID
	}

	// Update tax information if provided
	if taxItems, ok := metadata["tax_items"].([]models.TaxItem); ok {
		updates["tax_items"] = taxItems
	}
	if taxCents, ok := metadata["tax_cents"].(int64); ok {
		updates["tax_cents"] = taxCents
	}
	if totalCents, ok := metadata["total_cents"].(int64); ok {
		updates["total_cents"] = totalCents
	}

	return s.db.Model(&models.Purchase{}).Where("id = ?", id).Updates(updates).Error
}

// Approve approves a purchase that requires approval
func (s *PurchaseService) Approve(id uint, approverID uint) error {
	var purchase models.Purchase
	if err := s.db.First(&purchase, id).Error; err != nil {
		return fmt.Errorf("purchase not found: %w", err)
	}

	if !purchase.NeedsApproval() {
		return fmt.Errorf("purchase does not require approval")
	}

	now := time.Now()
	updates := map[string]interface{}{
		"approved_at": &now,
		"approved_by": approverID,
		"updated_at":  now,
	}

	// If payment was already made, mark as paid
	if purchase.Status == models.PurchaseStatusPaidPendingApproval {
		updates["status"] = models.PurchaseStatusPaid
	} else {
		// Create checkout session if approval granted before payment
		if s.paymentProvider != nil && s.paymentProvider.IsEnabled() {
			sessionID, err := s.paymentProvider.CreateCheckoutSession(&purchase)
			if err != nil {
				return fmt.Errorf("failed to create checkout session: %w", err)
			}
			updates["provider_session_id"] = sessionID
			updates["status"] = models.PurchaseStatusPending
		}
	}

	return s.db.Model(&purchase).Updates(updates).Error
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
	s.db.Model(&models.Purchase{}).Where("user_id = ?", userID).Count(&totalPurchases)
	stats["totalPurchases"] = totalPurchases

	// Total spent (paid purchases)
	var totalSpent struct {
		Total int64
	}
	s.db.Model(&models.Purchase{}).
		Select("COALESCE(SUM(total_cents), 0) as total").
		Where("user_id = ? AND status IN ?", userID, []string{models.PurchaseStatusPaid, models.PurchaseStatusRefunded}).
		Scan(&totalSpent)
	stats["totalSpent"] = totalSpent.Total

	// Status counts
	var statusCounts []struct {
		Status string
		Count  int64
	}
	s.db.Model(&models.Purchase{}).
		Select("status, COUNT(*) as count").
		Where("user_id = ?", userID).
		Group("status").
		Scan(&statusCounts)

	for _, sc := range statusCounts {
		switch sc.Status {
		case models.PurchaseStatusPending, models.PurchaseStatusRequiresApproval:
			stats["pending"] = stats["pending"].(int) + int(sc.Count)
		case models.PurchaseStatusPaid, models.PurchaseStatusPaidPendingApproval:
			stats["completed"] = stats["completed"].(int) + int(sc.Count)
		case models.PurchaseStatusRefunded:
			stats["refunded"] = int(sc.Count)
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
	// In the future, we could have multiple providers registered
	if provider == s.paymentProvider.GetProviderName() {
		return s.paymentProvider.GetCheckoutURL(purchase.ProviderSessionID)
	}

	return ""
}
