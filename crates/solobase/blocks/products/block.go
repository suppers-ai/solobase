package products

import (
	"github.com/suppers-ai/solobase/blocks/products/providers"
	waffle "github.com/suppers-ai/waffle-go"
	"github.com/suppers-ai/waffle-go/services/database"
)

const BlockName = "products-feature"

// ProductsWaffleBlock is a native waffle block for the products extension.
type ProductsWaffleBlock struct {
	router          *waffle.Router
	db              database.Service
	variableService *VariableService
	groupService    *GroupService
	productService  *ProductService
	pricingService  *PricingService
	purchaseService *PurchaseService
	paymentProvider providers.PaymentProvider
	webhookHandler  *WebhookHandler
	seeder          Seeder
}

// NewProductsWaffleBlock creates a native waffle products block with zero dependencies.
// The database.Service is obtained from platform services during Lifecycle(Init).
func NewProductsWaffleBlock() *ProductsWaffleBlock {
	b := &ProductsWaffleBlock{
		seeder: &DefaultSeeder{},
	}
	b.router = waffle.NewRouter()
	b.registerRoutes()
	return b
}

func (b *ProductsWaffleBlock) registerRoutes() {
	// Public
	b.router.Create("/ext/products/webhooks", b.handleWebhook)

	// Protected (user) endpoints
	b.router.Retrieve("/ext/products/products", b.handleListMyProducts)
	b.router.Create("/ext/products/products", b.handleCreateProduct)
	b.router.Update("/ext/products/products/{id}", b.handleUpdateProduct)
	b.router.Delete("/ext/products/products/{id}", b.handleDeleteProduct)
	b.router.Retrieve("/ext/products/groups", b.handleListMyGroups)
	b.router.Create("/ext/products/groups", b.handleUserCreateGroup)
	b.router.Retrieve("/ext/products/groups/{id}", b.handleGetGroup)
	b.router.Update("/ext/products/groups/{id}", b.handleUpdateGroup)
	b.router.Delete("/ext/products/groups/{id}", b.handleDeleteGroup)
	b.router.Retrieve("/ext/products/groups/{id}/products", b.handleListGroupProducts)
	b.router.Create("/ext/products/calculate-price", b.handleCalculatePrice)
	b.router.Create("/ext/products/purchase", b.handleCreatePurchase)
	b.router.Retrieve("/ext/products/purchases", b.handleListPurchases)
	b.router.Retrieve("/ext/products/purchases/stats", b.handlePurchaseStats)
	b.router.Retrieve("/ext/products/purchases/{id}", b.handleGetPurchase)
	b.router.Create("/ext/products/purchases/{id}/cancel", b.handleCancelPurchase)
	b.router.Retrieve("/ext/products/group-types", b.handleListGroupTypes)
	b.router.Retrieve("/ext/products/product-types", b.handleListProductTypes)
	b.router.Retrieve("/ext/products/variables", b.handleListVariables)

	// Admin endpoints
	b.router.Create("/admin/ext/products/products", b.handleCreateProduct)
	b.router.Update("/admin/ext/products/products/{id}", b.handleUpdateProduct)
	b.router.Delete("/admin/ext/products/products/{id}", b.handleDeleteProduct)
	b.router.Retrieve("/admin/ext/products/stats", b.handleProductStats)
	b.router.Retrieve("/admin/ext/products/provider/status", b.handleProviderStatus)
	b.router.Retrieve("/admin/ext/products/groups", b.handleListMyGroups)
	b.router.Retrieve("/admin/ext/products/variables", b.handleListVariables)
	b.router.Create("/admin/ext/products/variables", b.handleCreateVariable)
	b.router.Update("/admin/ext/products/variables/{id}", b.handleUpdateVariable)
	b.router.Delete("/admin/ext/products/variables/{id}", b.handleDeleteVariable)
	b.router.Retrieve("/admin/ext/products/group-types", b.handleListGroupTypes)
	b.router.Create("/admin/ext/products/group-types", b.handleCreateGroupType)
	b.router.Update("/admin/ext/products/group-types/{id}", b.handleUpdateGroupType)
	b.router.Delete("/admin/ext/products/group-types/{id}", b.handleDeleteGroupType)
	b.router.Retrieve("/admin/ext/products/product-types", b.handleListProductTypes)
	b.router.Create("/admin/ext/products/product-types", b.handleCreateProductType)
	b.router.Update("/admin/ext/products/product-types/{id}", b.handleUpdateProductType)
	b.router.Delete("/admin/ext/products/product-types/{id}", b.handleDeleteProductType)
	b.router.Retrieve("/admin/ext/products/pricing-templates", b.handleListPricingTemplates)
	b.router.Create("/admin/ext/products/pricing-templates", b.handleCreatePricingTemplate)
	b.router.Update("/admin/ext/products/pricing-templates/{id}", b.handleUpdatePricingTemplate)
	b.router.Delete("/admin/ext/products/pricing-templates/{id}", b.handleDeletePricingTemplate)
	b.router.Retrieve("/admin/ext/products/purchases", b.handleListAllPurchases)
	b.router.Create("/admin/ext/products/purchases/{id}/refund", b.handleRefundPurchase)
	b.router.Create("/admin/ext/products/purchases/{id}/approve", b.handleApprovePurchase)
	b.router.Create("/admin/ext/products/test-formula", b.handleTestFormula)
}

func (b *ProductsWaffleBlock) Info() waffle.BlockInfo {
	return waffle.BlockInfo{
		Name:         BlockName,
		Version:      "1.0.0",
		Interface:    "http.handler",
		Summary:      "Products and pricing extension",
		InstanceMode: waffle.Singleton,
		AllowedModes: []waffle.InstanceMode{waffle.Singleton},
		AdminUI:      &waffle.AdminUIInfo{Path: "/admin/products", Icon: "shopping-bag", Title: "Products"},
	}
}

func (b *ProductsWaffleBlock) Handle(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	return b.router.Route(ctx, msg)
}

func (b *ProductsWaffleBlock) Lifecycle(ctx waffle.Context, event waffle.LifecycleEvent) error {
	if event.Type == waffle.Init {
		// Get database.Service from platform services
		b.db = ctx.Services().Database
		if b.db == nil {
			return nil
		}

		// Initialize services
		b.variableService = NewVariableService(b.db)
		b.groupService = NewGroupService(b.db)
		b.productService = NewProductService(b.db, b.variableService)
		b.pricingService = NewPricingService(b.db, b.variableService)

		provider, _ := providers.GetDefaultProvider()
		b.paymentProvider = provider
		b.purchaseService = NewPurchaseService(b.db, b.productService, b.pricingService, provider)
		b.webhookHandler = NewWebhookHandler(provider, b.purchaseService)

		// Seed default data
		return SeedWithSeeder(b.db, b.seeder)
	}
	return nil
}

// GetProviderStatus returns payment provider status info.
func (b *ProductsWaffleBlock) GetProviderStatus() map[string]interface{} {
	status := map[string]interface{}{
		"configured":         false,
		"provider":           "none",
		"mode":               "none",
		"availableProviders": providers.ListAvailableProviders(),
		"configuredProvider": string(providers.GetConfiguredProviderType()),
	}

	if b.paymentProvider != nil && b.paymentProvider.IsEnabled() {
		status["configured"] = true
		status["provider"] = b.paymentProvider.GetProviderName()
		status["mode"] = "production"
		if b.paymentProvider.IsTestMode() {
			status["mode"] = "test"
		}
	}

	return status
}
