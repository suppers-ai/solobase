package products

import (
	"github.com/suppers-ai/solobase/extensions/official/products"
	"net/http"
)

// ProductsExtensionHandlers wraps the products extension for the API router
type ProductsExtensionHandlers struct {
	ext *products.ProductsExtension
}

// NewProductsExtensionHandlers creates a new wrapper for products extension handlers
func NewProductsExtensionHandlers() *ProductsExtensionHandlers {
	// Start with nil extension, will be set later via SetExtension
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

// Admin API handlers - Variables
func (h *ProductsExtensionHandlers) HandleListVariables() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().ListVariables(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleCreateVariable() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().CreateVariable(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleUpdateVariable() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().UpdateVariable(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleDeleteVariable() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().DeleteVariable(w, r)
	}
}

// HandleProviderStatus returns payment provider status
func (h *ProductsExtensionHandlers) HandleProviderStatus() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().GetProviderStatus(w, r)
	}
}

// Admin API handlers - Entity Types
func (h *ProductsExtensionHandlers) HandleListGroupTypes() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().ListGroupTypes(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleCreateGroupType() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().CreateGroupType(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleUpdateGroupType() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().UpdateGroupType(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleDeleteGroupType() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().DeleteGroupType(w, r)
	}
}

// User API handlers - Entities (user's actual entities)
func (h *ProductsExtensionHandlers) HandleListGroups() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetUserAPI().ListMyGroups(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleCreateGroup() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetUserAPI().CreateGroup(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleUpdateGroup() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetUserAPI().UpdateGroup(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleDeleteGroup() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetUserAPI().DeleteGroup(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleGetGroup() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetUserAPI().GetGroup(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleGroupProducts() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetUserAPI().ListGroupProducts(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleCalculatePrice() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetUserAPI().CalculatePrice(w, r)
	}
}

// Admin API handlers - Product Types
func (h *ProductsExtensionHandlers) HandleListProductTypes() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().ListProductTypes(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleCreateProductType() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().CreateProductType(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleUpdateProductType() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().UpdateProductType(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleDeleteProductType() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().DeleteProductType(w, r)
	}
}

// Admin API handlers - Pricing Templates
func (h *ProductsExtensionHandlers) HandleListPricingTemplates() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().ListPricingTemplates(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleCreatePricingTemplate() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().CreatePricingTemplate(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleUpdatePricingTemplate() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().UpdatePricingTemplate(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleDeletePricingTemplate() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().DeletePricingTemplate(w, r)
	}
}

// Simple handlers for basic product CRUD (temporary compatibility)
func (h *ProductsExtensionHandlers) HandleProductsList() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		// List all products across all user's entities
		h.ext.GetUserAPI().ListMyProducts(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleProductsCreate() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetUserAPI().CreateProduct(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleProductsUpdate() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetUserAPI().UpdateProduct(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleProductsStats() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().GetProductStats(w, r)
	}
}

// Purchase handlers
func (h *ProductsExtensionHandlers) HandleCreatePurchase() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetUserAPI().CreatePurchase(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleListPurchases() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetUserAPI().ListPurchases(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleGetPurchase() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetUserAPI().GetPurchase(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleCancelPurchase() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetUserAPI().CancelPurchase(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandlePurchaseStats() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetUserAPI().GetPurchaseStats(w, r)
	}
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
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().RefundPurchase(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleApprovePurchase() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().ApprovePurchase(w, r)
	}
}

func (h *ProductsExtensionHandlers) HandleListAllPurchases() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetAdminAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetAdminAPI().ListAllPurchases(w, r)
	}
}
