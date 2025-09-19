package products

import (
	"encoding/json"
	"errors"
	"net/http"
	"strconv"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/constants"
	"github.com/suppers-ai/solobase/extensions/official/products/models"
	"gorm.io/gorm"
)

// getUserIDFromContext extracts the user ID from the request context
func getUserIDFromContext(r *http.Request) (string, error) {
	userID, ok := r.Context().Value(constants.ContextKeyUserID).(string)
	if !ok || userID == "" {
		return "", errors.New("user not authenticated")
	}
	return userID, nil
}

// AdminAPI handles admin operations
type AdminAPI struct {
	db              *gorm.DB
	variableService *VariableService
	groupService    *GroupService
	productService  *ProductService
	pricingService  *PricingService
	extension       *ProductsExtension // Reference to extension for provider status
}

func NewAdminAPI(db *gorm.DB, vs *VariableService, es *GroupService, ps *ProductService, prs *PricingService) *AdminAPI {
	return &AdminAPI{
		db:              db,
		variableService: vs,
		groupService:    es,
		productService:  ps,
		pricingService:  prs,
	}
}

// SetExtension sets the reference to the extension (for provider status)
func (a *AdminAPI) SetExtension(ext *ProductsExtension) {
	a.extension = ext
}

// GetProviderStatus returns the payment provider status
func (a *AdminAPI) GetProviderStatus(w http.ResponseWriter, r *http.Request) {
	status := map[string]interface{}{
		"configured": false,
		"provider":   "none",
		"mode":       "none",
	}

	if a.extension != nil {
		status = a.extension.GetProviderStatus()
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(status)
}

// Variable management
func (a *AdminAPI) ListVariables(w http.ResponseWriter, r *http.Request) {
	variables, err := a.variableService.List()
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(variables)
}

func (a *AdminAPI) CreateVariable(w http.ResponseWriter, r *http.Request) {
	var variable models.Variable
	if err := json.NewDecoder(r.Body).Decode(&variable); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if err := a.variableService.Create(&variable); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(variable)
}

func (a *AdminAPI) UpdateVariable(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	var variable models.Variable
	if err := json.NewDecoder(r.Body).Decode(&variable); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if err := a.variableService.Update(uint(id), &variable); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(variable)
}

func (a *AdminAPI) DeleteVariable(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	if err := a.variableService.Delete(uint(id)); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// Group Template management
func (a *AdminAPI) ListGroupTypes(w http.ResponseWriter, r *http.Request) {
	var groupTemplates []models.GroupTemplate
	if err := a.db.Find(&groupTemplates).Error; err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(groupTemplates)
}

func (a *AdminAPI) CreateGroupType(w http.ResponseWriter, r *http.Request) {
	var groupTemplate models.GroupTemplate
	if err := json.NewDecoder(r.Body).Decode(&groupTemplate); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if err := a.db.Create(&groupTemplate).Error; err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(groupTemplate)
}

func (a *AdminAPI) UpdateGroupType(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	var groupTemplate models.GroupTemplate
	if err := json.NewDecoder(r.Body).Decode(&groupTemplate); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if err := a.db.Model(&models.GroupTemplate{}).Where("id = ?", id).Updates(&groupTemplate).Error; err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(groupTemplate)
}

func (a *AdminAPI) DeleteGroupType(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	if err := a.db.Delete(&models.GroupTemplate{}, id).Error; err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// Product Template management
func (a *AdminAPI) ListProductTypes(w http.ResponseWriter, r *http.Request) {
	var productTemplates []models.ProductTemplate
	if err := a.db.Find(&productTemplates).Error; err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(productTemplates)
}

func (a *AdminAPI) CreateProductType(w http.ResponseWriter, r *http.Request) {
	var productTemplate models.ProductTemplate
	if err := json.NewDecoder(r.Body).Decode(&productTemplate); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if err := a.db.Create(&productTemplate).Error; err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(productTemplate)
}

func (a *AdminAPI) UpdateProductType(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	var productTemplate models.ProductTemplate
	if err := json.NewDecoder(r.Body).Decode(&productTemplate); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if err := a.db.Model(&models.ProductTemplate{}).Where("id = ?", id).Updates(&productTemplate).Error; err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(productTemplate)
}

func (a *AdminAPI) DeleteProductType(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	if err := a.db.Delete(&models.ProductTemplate{}, id).Error; err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// Pricing Template management
func (a *AdminAPI) ListPricingTemplates(w http.ResponseWriter, r *http.Request) {
	var templates []models.PricingTemplate
	if err := a.db.Find(&templates).Error; err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(templates)
}

func (a *AdminAPI) CreatePricingTemplate(w http.ResponseWriter, r *http.Request) {
	var template models.PricingTemplate
	if err := json.NewDecoder(r.Body).Decode(&template); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if err := a.db.Create(&template).Error; err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(template)
}

func (a *AdminAPI) UpdatePricingTemplate(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	templateID, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	var template models.PricingTemplate
	if err := json.NewDecoder(r.Body).Decode(&template); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	// Ensure the ID matches
	template.ID = uint(templateID)

	if err := a.db.Save(&template).Error; err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(template)
}

func (a *AdminAPI) DeletePricingTemplate(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	templateID, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	if err := a.db.Delete(&models.PricingTemplate{}, uint(templateID)).Error; err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// UserAPI handles user operations
type UserAPI struct {
	db              *gorm.DB
	groupService    *GroupService
	productService  *ProductService
	pricingService  *PricingService
	purchaseService *PurchaseService
}

func NewUserAPI(db *gorm.DB, es *GroupService, ps *ProductService, prs *PricingService, purchaseService *PurchaseService) *UserAPI {
	return &UserAPI{
		db:              db,
		groupService:    es,
		productService:  ps,
		pricingService:  prs,
		purchaseService: purchaseService,
	}
}

// Group management for users
func (u *UserAPI) ListMyGroups(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	groups, err := u.groupService.ListByUser(userID)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(groups)
}

func (u *UserAPI) CreateGroup(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	var group models.Group
	if err := json.NewDecoder(r.Body).Decode(&group); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	group.UserID = userID
	if err := u.groupService.Create(&group); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(group)
}

func (u *UserAPI) UpdateGroup(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	var group models.Group
	if err := json.NewDecoder(r.Body).Decode(&group); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if err := u.groupService.Update(uint(id), userID, &group); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(group)
}

func (u *UserAPI) DeleteGroup(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	if err := u.groupService.Delete(uint(id), userID); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func (u *UserAPI) GetGroup(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	group, err := u.groupService.GetByID(uint(id), userID)
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(group)
}

// Product management
func (u *UserAPI) ListGroupProducts(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	groupID, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	products, err := u.productService.ListByGroup(uint(groupID))
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(products)
}

func (u *UserAPI) ListMyProducts(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	products, err := u.productService.ListByUser(userID)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(products)
}

func (u *UserAPI) CreateProduct(w http.ResponseWriter, r *http.Request) {
	var product models.Product
	if err := json.NewDecoder(r.Body).Decode(&product); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	// TODO: Verify user owns the group

	if err := u.productService.Create(&product); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(product)
}

func (u *UserAPI) CalculatePrice(w http.ResponseWriter, r *http.Request) {
	var req struct {
		ProductID uint                   `json:"product_id"`
		Variables map[string]interface{} `json:"variables"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	price, err := u.pricingService.CalculatePrice(req.ProductID, req.Variables)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"price":     price,
		"currency":  "USD",
		"breakdown": []interface{}{}, // TODO: Add breakdown details
	})
}

func (u *UserAPI) GetProductStats(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	// Get counts
	var groupCount int64
	u.db.Model(&models.Group{}).Where("user_id = ?", userID).Count(&groupCount)

	var productCount int64
	u.db.Model(&models.Product{}).
		Joins("JOIN groups ON products.group_id = groups.id").
		Where("groups.user_id = ?", userID).
		Count(&productCount)

	stats := map[string]interface{}{
		"totalProducts":  productCount,
		"totalEntities":  groupCount,
		"activeProducts": productCount, // TODO: Filter by active
		"totalRevenue":   0,
		"avgPrice":       0,
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}

// PublicAPI handles public operations
type PublicAPI struct {
	db             *gorm.DB
	productService *ProductService
}

func NewPublicAPI(db *gorm.DB, ps *ProductService) *PublicAPI {
	return &PublicAPI{
		db:             db,
		productService: ps,
	}
}

// Public product listing, search, etc can be added here

// Purchase management endpoints
func (u *UserAPI) CreatePurchase(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	var req PurchaseRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	req.UserID = userID

	// Create purchase and checkout session
	purchase, err := u.purchaseService.Create(&req)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	// Return purchase with checkout URL
	response := map[string]interface{}{
		"purchase": purchase,
	}

	// Get checkout URL from the purchase service (provider-agnostic)
	if checkoutURL := u.purchaseService.GetCheckoutURL(purchase); checkoutURL != "" {
		response["checkout_url"] = checkoutURL
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(response)
}

func (u *UserAPI) ListPurchases(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	// Parse query parameters
	limit := 20
	offset := 0
	if l := r.URL.Query().Get("limit"); l != "" {
		if parsed, err := strconv.Atoi(l); err == nil && parsed > 0 && parsed <= 100 {
			limit = parsed
		}
	}
	if o := r.URL.Query().Get("offset"); o != "" {
		if parsed, err := strconv.Atoi(o); err == nil && parsed >= 0 {
			offset = parsed
		}
	}

	purchases, total, err := u.purchaseService.GetByUserID(userID, limit, offset)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"purchases": purchases,
		"total":     total,
		"limit":     limit,
		"offset":    offset,
	})
}

func (u *UserAPI) GetPurchase(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	purchase, err := u.purchaseService.GetByID(uint(id))
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}

	// Verify user owns this purchase
	if purchase.UserID != userID {
		http.Error(w, "Unauthorized", http.StatusForbidden)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(purchase)
}

func (u *UserAPI) CancelPurchase(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	// Verify ownership
	purchase, err := u.purchaseService.GetByID(uint(id))
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}
	if purchase.UserID != userID {
		http.Error(w, "Unauthorized", http.StatusForbidden)
		return
	}

	var req struct {
		Reason string `json:"reason"`
	}
	json.NewDecoder(r.Body).Decode(&req)

	if err := u.purchaseService.Cancel(uint(id), req.Reason); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func (u *UserAPI) GetPurchaseStats(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	stats, err := u.purchaseService.GetStats(userID)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}

// Admin purchase endpoints
func (a *AdminAPI) RefundPurchase(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	var req struct {
		Amount int64  `json:"amount"` // Amount in cents, 0 for full refund
		Reason string `json:"reason"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	// Get purchase service (need to expose it from admin API)
	purchaseService := NewPurchaseService(a.db, a.productService, a.pricingService, nil)
	if err := purchaseService.Refund(uint(id), req.Amount, req.Reason); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func (a *AdminAPI) ApprovePurchase(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	approverID := uint(1) // TODO: Get from admin context

	// Get purchase service
	purchaseService := NewPurchaseService(a.db, a.productService, a.pricingService, nil)
	if err := purchaseService.Approve(uint(id), approverID); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func (a *AdminAPI) ListAllPurchases(w http.ResponseWriter, r *http.Request) {
	// Parse query parameters
	limit := 20
	offset := 0
	if l := r.URL.Query().Get("limit"); l != "" {
		if parsed, err := strconv.Atoi(l); err == nil && parsed > 0 && parsed <= 100 {
			limit = parsed
		}
	}
	if o := r.URL.Query().Get("offset"); o != "" {
		if parsed, err := strconv.Atoi(o); err == nil && parsed >= 0 {
			offset = parsed
		}
	}

	var purchases []models.Purchase
	var total int64

	// Count total
	if err := a.db.Model(&models.Purchase{}).Count(&total).Error; err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	// Fetch purchases
	if err := a.db.Order("created_at DESC").Limit(limit).Offset(offset).Find(&purchases).Error; err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"purchases": purchases,
		"total":     total,
		"limit":     limit,
		"offset":    offset,
	})
}

// GetProductStats returns global product statistics for admins
func (a *AdminAPI) GetProductStats(w http.ResponseWriter, r *http.Request) {
	// Get total counts across all users
	var groupCount int64
	a.db.Model(&models.Group{}).Count(&groupCount)

	var productCount int64
	a.db.Model(&models.Product{}).Count(&productCount)

	var activeProductCount int64
	a.db.Model(&models.Product{}).Where("active = ?", true).Count(&activeProductCount)

	// Get total revenue from purchases
	var totalRevenue float64
	a.db.Model(&models.Purchase{}).
		Where("status IN ?", []string{string(models.PurchaseStatusPaid), string(models.PurchaseStatusPaidPendingApproval)}).
		Select("COALESCE(SUM(total_cents), 0) / 100.0").
		Scan(&totalRevenue)

	// Calculate average price
	var avgPrice float64
	a.db.Model(&models.Product{}).
		Select("COALESCE(AVG(base_price_cents), 0) / 100.0").
		Scan(&avgPrice)

	stats := map[string]interface{}{
		"totalProducts":  productCount,
		"totalGroups":    groupCount,
		"activeProducts": activeProductCount,
		"totalRevenue":   totalRevenue,
		"avgPrice":       avgPrice,
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}
