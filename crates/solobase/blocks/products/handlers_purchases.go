package products

import (
	"fmt"
	"strconv"

	wafer "github.com/wafer-run/wafer-go"
)

func (b *ProductsBlock) handleWebhook(_ wafer.Context, msg *wafer.Message) wafer.Result {
	if b.paymentProvider == nil {
		return wafer.Error(msg, 400, "no_provider", "No payment provider configured")
	}

	body := msg.Data

	providerName := b.paymentProvider.GetProviderName()
	var signature string
	switch providerName {
	case "stripe":
		signature = msg.Header("Stripe-Signature")
	case "paypal":
		signature = msg.Header("Paypal-Transmission-Sig")
	default:
		signature = msg.Header("X-Webhook-Signature")
	}

	if signature == "" {
		return wafer.Error(msg, 400, "missing_signature", "Missing webhook signature")
	}

	err := b.paymentProvider.HandleWebhook(body, signature, b.webhookHandler.processWebhookEvent)
	if err != nil {
		return wafer.Error(msg, 400, "webhook_error", fmt.Sprintf("Failed to process webhook: %v", err))
	}

	return wafer.Respond(msg, 200, nil, "")
}

func (b *ProductsBlock) handleCalculatePrice(_ wafer.Context, msg *wafer.Message) wafer.Result {
	var calcReq struct {
		ProductID uint                   `json:"productId"`
		Variables map[string]interface{} `json:"variables"`
	}
	if err := msg.Decode(&calcReq); err != nil {
		return wafer.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	price, err := b.pricingService.CalculatePrice(calcReq.ProductID, calcReq.Variables)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, map[string]interface{}{
		"price":     price,
		"currency":  "USD",
		"breakdown": []interface{}{},
	})
}

func (b *ProductsBlock) handleCreatePurchase(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	var purchaseReq PurchaseRequest
	if err := msg.Decode(&purchaseReq); err != nil {
		return wafer.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	purchaseReq.UserID = userID

	purchase, err := b.purchaseService.Create(&purchaseReq)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}

	response := map[string]interface{}{
		"purchase": purchase,
	}
	if checkoutURL := b.purchaseService.GetCheckoutURL(purchase); checkoutURL != "" {
		response["checkoutUrl"] = checkoutURL
	}
	return wafer.JSONRespond(msg, 201, response)
}

func (b *ProductsBlock) handleListPurchases(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	limit := 20
	offset := 0
	if l := msg.Query("limit"); l != "" {
		if parsed, err := strconv.Atoi(l); err == nil && parsed > 0 && parsed <= 100 {
			limit = parsed
		}
	}
	if o := msg.Query("offset"); o != "" {
		if parsed, err := strconv.Atoi(o); err == nil && parsed >= 0 {
			offset = parsed
		}
	}

	purchases, total, err := b.purchaseService.GetByUserID(userID, limit, offset)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, map[string]interface{}{
		"purchases": purchases,
		"total":     total,
		"limit":     limit,
		"offset":    offset,
	})
}

func (b *ProductsBlock) handleGetPurchase(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return wafer.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	purchase, err := b.purchaseService.GetByID(uint(id))
	if err != nil {
		return wafer.Error(msg, 404, "not_found", err.Error())
	}

	if purchase.UserID != userID {
		return wafer.Error(msg, 403, "forbidden", "Unauthorized")
	}
	return wafer.JSONRespond(msg, 200, purchase)
}

func (b *ProductsBlock) handleCancelPurchase(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return wafer.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	purchase, err := b.purchaseService.GetByID(uint(id))
	if err != nil {
		return wafer.Error(msg, 404, "not_found", err.Error())
	}
	if purchase.UserID != userID {
		return wafer.Error(msg, 403, "forbidden", "Unauthorized")
	}

	var cancelReq struct {
		Reason string `json:"reason"`
	}
	msg.Decode(&cancelReq) // Ignore error — reason is optional

	if err := b.purchaseService.Cancel(uint(id), cancelReq.Reason); err != nil {
		return wafer.Error(msg, 400, "cancel_error", err.Error())
	}
	return wafer.Respond(msg, 204, nil, "")
}

func (b *ProductsBlock) handlePurchaseStats(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	stats, err := b.purchaseService.GetStats(userID)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, stats)
}

func (b *ProductsBlock) handleListAllPurchases(_ wafer.Context, msg *wafer.Message) wafer.Result {
	limit := 20
	offset := 0
	if l := msg.Query("limit"); l != "" {
		if parsed, err := strconv.Atoi(l); err == nil && parsed > 0 && parsed <= 100 {
			limit = parsed
		}
	}
	if o := msg.Query("offset"); o != "" {
		if parsed, err := strconv.Atoi(o); err == nil && parsed >= 0 {
			offset = parsed
		}
	}

	purchases, total, err := b.purchaseService.ListAll(limit, offset)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, map[string]interface{}{
		"purchases": purchases,
		"total":     total,
		"limit":     limit,
		"offset":    offset,
	})
}

func (b *ProductsBlock) handleRefundPurchase(_ wafer.Context, msg *wafer.Message) wafer.Result {
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return wafer.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	var refundReq struct {
		Amount int64  `json:"amount"`
		Reason string `json:"reason"`
	}
	if err := msg.Decode(&refundReq); err != nil {
		return wafer.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	if err := b.purchaseService.Refund(uint(id), refundReq.Amount, refundReq.Reason); err != nil {
		return wafer.Error(msg, 400, "refund_error", err.Error())
	}
	return wafer.Respond(msg, 204, nil, "")
}

func (b *ProductsBlock) handleApprovePurchase(_ wafer.Context, msg *wafer.Message) wafer.Result {
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return wafer.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	approverID := msg.UserID()
	if approverID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Authentication required")
	}

	if err := b.purchaseService.Approve(uint(id), approverID); err != nil {
		return wafer.Error(msg, 400, "approve_error", err.Error())
	}
	return wafer.Respond(msg, 204, nil, "")
}
