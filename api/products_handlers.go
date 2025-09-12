package api

import (
	"github.com/suppers-ai/solobase/extensions/official/products"
	"gorm.io/gorm"
	"net/http"
)

// ProductsExtensionHandlers wraps the products extension for the API router
type ProductsExtensionHandlers struct {
	ext *products.ProductsExtension
}

// NewProductsExtensionHandlers creates a new wrapper for products extension handlers
func NewProductsExtensionHandlers() *ProductsExtensionHandlers {
	ext := products.NewProductsExtensionWithDB(nil)
	return &ProductsExtensionHandlers{ext: ext}
}

// NewProductsExtensionHandlersWithDB creates handlers with database
func NewProductsExtensionHandlersWithDB(db *gorm.DB) *ProductsExtensionHandlers {
	ext := products.NewProductsExtensionWithDB(db)
	// SetDatabase will run migrations and seed data
	ext.SetDatabase(db)
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

func (h *ProductsExtensionHandlers) HandleProductsStats() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if h.ext == nil || h.ext.GetUserAPI() == nil {
			http.Error(w, "Extension not initialized", http.StatusServiceUnavailable)
			return
		}
		h.ext.GetUserAPI().GetProductStats(w, r)
	}
}
