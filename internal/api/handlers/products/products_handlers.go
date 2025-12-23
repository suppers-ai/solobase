package products

import (
	"net/http"

	"github.com/suppers-ai/solobase/extensions/official/products"
)

// ProductsExtensionHandlers wraps the products extension for the API router
type ProductsExtensionHandlers struct {
	ext *products.ProductsExtension
}

// NewProductsExtensionHandlers creates a new wrapper for products extension handlers
func NewProductsExtensionHandlers() *ProductsExtensionHandlers {
	return &ProductsExtensionHandlers{ext: nil}
}

// NewProductsExtensionHandlersWithExtension creates handlers with an existing extension
func NewProductsExtensionHandlersWithExtension(ext *products.ProductsExtension) *ProductsExtensionHandlers {
	return &ProductsExtensionHandlers{ext: ext}
}

// SetExtension sets the extension instance (called after initialization)
func (h *ProductsExtensionHandlers) SetExtension(ext *products.ProductsExtension) {
	h.ext = ext
}

// wrapAdmin creates a handler that delegates to an admin API method
func (h *ProductsExtensionHandlers) wrapAdmin(fn func(*products.AdminAPI) http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		fn(h.ext.GetAdminAPI())(w, r)
	}
}

// wrapUser creates a handler that delegates to a user API method
func (h *ProductsExtensionHandlers) wrapUser(fn func(*products.UserAPI) http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		fn(h.ext.GetUserAPI())(w, r)
	}
}

// Admin API handlers - Variables
func (h *ProductsExtensionHandlers) HandleListVariables() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.ListVariables })
}
func (h *ProductsExtensionHandlers) HandleCreateVariable() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.CreateVariable })
}
func (h *ProductsExtensionHandlers) HandleUpdateVariable() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.UpdateVariable })
}
func (h *ProductsExtensionHandlers) HandleDeleteVariable() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.DeleteVariable })
}
func (h *ProductsExtensionHandlers) HandleProviderStatus() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.GetProviderStatus })
}

// Admin API handlers - Group Types
func (h *ProductsExtensionHandlers) HandleListGroupTypes() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.ListGroupTypes })
}
func (h *ProductsExtensionHandlers) HandleCreateGroupType() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.CreateGroupType })
}
func (h *ProductsExtensionHandlers) HandleUpdateGroupType() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.UpdateGroupType })
}
func (h *ProductsExtensionHandlers) HandleDeleteGroupType() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.DeleteGroupType })
}

// User API handlers - Groups
func (h *ProductsExtensionHandlers) HandleListGroups() http.HandlerFunc {
	return h.wrapUser(func(api *products.UserAPI) http.HandlerFunc { return api.ListMyGroups })
}
func (h *ProductsExtensionHandlers) HandleCreateGroup() http.HandlerFunc {
	return h.wrapUser(func(api *products.UserAPI) http.HandlerFunc { return api.CreateGroup })
}
func (h *ProductsExtensionHandlers) HandleUpdateGroup() http.HandlerFunc {
	return h.wrapUser(func(api *products.UserAPI) http.HandlerFunc { return api.UpdateGroup })
}
func (h *ProductsExtensionHandlers) HandleDeleteGroup() http.HandlerFunc {
	return h.wrapUser(func(api *products.UserAPI) http.HandlerFunc { return api.DeleteGroup })
}
func (h *ProductsExtensionHandlers) HandleGetGroup() http.HandlerFunc {
	return h.wrapUser(func(api *products.UserAPI) http.HandlerFunc { return api.GetGroup })
}
func (h *ProductsExtensionHandlers) HandleGroupProducts() http.HandlerFunc {
	return h.wrapUser(func(api *products.UserAPI) http.HandlerFunc { return api.ListGroupProducts })
}
func (h *ProductsExtensionHandlers) HandleCalculatePrice() http.HandlerFunc {
	return h.wrapUser(func(api *products.UserAPI) http.HandlerFunc { return api.CalculatePrice })
}

// Admin API handlers - Product Types
func (h *ProductsExtensionHandlers) HandleListProductTypes() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.ListProductTypes })
}
func (h *ProductsExtensionHandlers) HandleCreateProductType() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.CreateProductType })
}
func (h *ProductsExtensionHandlers) HandleUpdateProductType() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.UpdateProductType })
}
func (h *ProductsExtensionHandlers) HandleDeleteProductType() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.DeleteProductType })
}

// Admin API handlers - Pricing Templates
func (h *ProductsExtensionHandlers) HandleListPricingTemplates() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.ListPricingTemplates })
}
func (h *ProductsExtensionHandlers) HandleCreatePricingTemplate() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.CreatePricingTemplate })
}
func (h *ProductsExtensionHandlers) HandleUpdatePricingTemplate() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.UpdatePricingTemplate })
}
func (h *ProductsExtensionHandlers) HandleDeletePricingTemplate() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.DeletePricingTemplate })
}

// Product CRUD handlers
func (h *ProductsExtensionHandlers) HandleProductsList() http.HandlerFunc {
	return h.wrapUser(func(api *products.UserAPI) http.HandlerFunc { return api.ListMyProducts })
}
func (h *ProductsExtensionHandlers) HandleProductsCreate() http.HandlerFunc {
	return h.wrapUser(func(api *products.UserAPI) http.HandlerFunc { return api.CreateProduct })
}
func (h *ProductsExtensionHandlers) HandleProductsUpdate() http.HandlerFunc {
	return h.wrapUser(func(api *products.UserAPI) http.HandlerFunc { return api.UpdateProduct })
}
func (h *ProductsExtensionHandlers) HandleProductsStats() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.GetProductStats })
}

// Purchase handlers
func (h *ProductsExtensionHandlers) HandleCreatePurchase() http.HandlerFunc {
	return h.wrapUser(func(api *products.UserAPI) http.HandlerFunc { return api.CreatePurchase })
}
func (h *ProductsExtensionHandlers) HandleListPurchases() http.HandlerFunc {
	return h.wrapUser(func(api *products.UserAPI) http.HandlerFunc { return api.ListPurchases })
}
func (h *ProductsExtensionHandlers) HandleGetPurchase() http.HandlerFunc {
	return h.wrapUser(func(api *products.UserAPI) http.HandlerFunc { return api.GetPurchase })
}
func (h *ProductsExtensionHandlers) HandleCancelPurchase() http.HandlerFunc {
	return h.wrapUser(func(api *products.UserAPI) http.HandlerFunc { return api.CancelPurchase })
}
func (h *ProductsExtensionHandlers) HandlePurchaseStats() http.HandlerFunc {
	return h.wrapUser(func(api *products.UserAPI) http.HandlerFunc { return api.GetPurchaseStats })
}

// Webhook handler
func (h *ProductsExtensionHandlers) HandleWebhook() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetWebhookHandler() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetWebhookHandler().HandleWebhook(w, r)
	}
}

// Admin purchase handlers
func (h *ProductsExtensionHandlers) HandleRefundPurchase() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.RefundPurchase })
}
func (h *ProductsExtensionHandlers) HandleApprovePurchase() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.ApprovePurchase })
}
func (h *ProductsExtensionHandlers) HandleListAllPurchases() http.HandlerFunc {
	return h.wrapAdmin(func(api *products.AdminAPI) http.HandlerFunc { return api.ListAllPurchases })
}
