package products

import (
	"fmt"
	"os"
	"testing"

	"github.com/suppers-ai/solobase/blocks/products/models"
	waffle "github.com/suppers-ai/waffle-go"
	"github.com/suppers-ai/waffle-go/services/database"
	"github.com/suppers-ai/waffle-go/waffletest"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// noopSeeder prevents the default seeder from running so tests start clean.
type noopSeeder struct{}

func (n *noopSeeder) ShouldSeed(_ database.Service) bool                                        { return false }
func (n *noopSeeder) SeedVariables(_ database.Service) ([]models.Variable, error)                { return nil, nil }
func (n *noopSeeder) SeedGroupTemplates(_ database.Service) ([]models.GroupTemplate, error)      { return nil, nil }
func (n *noopSeeder) SeedProductTemplates(_ database.Service) ([]models.ProductTemplate, error)  { return nil, nil }
func (n *noopSeeder) SeedPricingTemplates(_ database.Service) ([]models.PricingTemplate, error)  { return nil, nil }

// setupProductsBlock creates a test DB from block.json, initializes the block
// with a no-op seeder so tests start with empty tables, and returns the block
// and context.
func setupProductsBlock(t *testing.T) (*ProductsWaffleBlock, waffle.Context) {
	t.Helper()

	manifest, err := os.ReadFile("block.json")
	require.NoError(t, err, "reading block.json")

	db := waffletest.SetupDBFromManifest(t, manifest)
	ctx := waffletest.NewContext(db)

	block := NewProductsWaffleBlock()
	block.seeder = &noopSeeder{}
	waffletest.InitBlock(t, block, ctx)

	return block, ctx
}

// setupProductsBlockSeeded creates a test DB and lets the default seeder run.
func setupProductsBlockSeeded(t *testing.T) (*ProductsWaffleBlock, waffle.Context) {
	t.Helper()

	manifest, err := os.ReadFile("block.json")
	require.NoError(t, err, "reading block.json")

	db := waffletest.SetupDBFromManifest(t, manifest)
	ctx := waffletest.NewContext(db)

	block := NewProductsWaffleBlock()
	waffletest.InitBlock(t, block, ctx)

	return block, ctx
}

func TestProductsBlock_Info(t *testing.T) {
	block := NewProductsWaffleBlock()
	info := block.Info()

	assert.Equal(t, "products-feature", info.Name)
	assert.Equal(t, "1.0.0", info.Version)
	assert.Equal(t, waffle.Singleton, info.InstanceMode)
}

func TestProductsBlock_ListVariablesEmpty(t *testing.T) {
	block, ctx := setupProductsBlock(t)

	msg := waffletest.Retrieve("/ext/products/variables")
	waffletest.WithAuth(msg, "user-1", "user@test.com")

	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, waffletest.Status(result))

	var variables []map[string]any
	waffletest.DecodeResponse(t, result, &variables)

	// Even with empty DB, the system variable "running_total" is always returned.
	require.NotEmpty(t, variables, "expected at least system variables")

	found := false
	for _, v := range variables {
		if v["name"] == "running_total" {
			found = true
			assert.Equal(t, "system", v["type"])
		}
	}
	assert.True(t, found, "expected running_total system variable")
}

func TestProductsBlock_ListVariablesSeeded(t *testing.T) {
	block, ctx := setupProductsBlockSeeded(t)

	msg := waffletest.Retrieve("/ext/products/variables")
	waffletest.WithAuth(msg, "user-1", "user@test.com")

	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, waffletest.Status(result))

	var variables []map[string]any
	waffletest.DecodeResponse(t, result, &variables)

	// Seeded data includes default variables + system variable
	assert.Greater(t, len(variables), 1, "expected seeded variables plus system variables")
}

func TestProductsBlock_CreateVariable(t *testing.T) {
	block, ctx := setupProductsBlock(t)

	msg := waffletest.Create("/admin/ext/products/variables", map[string]any{
		"name":        "test_var",
		"displayName": "Test Variable",
		"valueType":   "number",
		"type":        "user",
		"description": "A test variable",
		"status":      "active",
	})
	waffletest.WithAuth(msg, "admin-1", "admin@test.com")
	waffletest.WithRoles(msg, "admin")

	result := block.Handle(ctx, msg)

	assert.Equal(t, 201, waffletest.Status(result))

	var created map[string]any
	waffletest.DecodeResponse(t, result, &created)
	assert.Equal(t, "test_var", created["name"])
	assert.Equal(t, "number", created["valueType"])

	// Verify it shows up in the list
	listMsg := waffletest.Retrieve("/ext/products/variables")
	waffletest.WithAuth(listMsg, "user-1", "user@test.com")
	listResult := block.Handle(ctx, listMsg)

	var variables []map[string]any
	waffletest.DecodeResponse(t, listResult, &variables)

	found := false
	for _, v := range variables {
		if v["name"] == "test_var" {
			found = true
		}
	}
	assert.True(t, found, "expected created variable in list")
}

func TestProductsBlock_ListGroupTypesEmpty(t *testing.T) {
	block, ctx := setupProductsBlock(t)

	msg := waffletest.Retrieve("/ext/products/group-types")
	waffletest.WithAuth(msg, "user-1", "user@test.com")

	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, waffletest.Status(result))

	var groupTypes []any
	waffletest.DecodeResponse(t, result, &groupTypes)
	assert.Empty(t, groupTypes, "expected empty group types with noop seeder")
}

func TestProductsBlock_ListGroupTypesSeeded(t *testing.T) {
	block, ctx := setupProductsBlockSeeded(t)

	msg := waffletest.Retrieve("/ext/products/group-types")
	waffletest.WithAuth(msg, "user-1", "user@test.com")

	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, waffletest.Status(result))

	var groupTypes []map[string]any
	waffletest.DecodeResponse(t, result, &groupTypes)
	assert.NotEmpty(t, groupTypes, "expected seeded group types")
}

func TestProductsBlock_ListProductsRequiresAuth(t *testing.T) {
	block, ctx := setupProductsBlock(t)

	msg := waffletest.Retrieve("/ext/products/products")

	result := block.Handle(ctx, msg)

	assert.Equal(t, 401, waffletest.Status(result))
}

func TestProductsBlock_ListProductsEmpty(t *testing.T) {
	block, ctx := setupProductsBlock(t)

	msg := waffletest.Retrieve("/ext/products/products")
	waffletest.WithAuth(msg, "user-1", "user@test.com")

	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, waffletest.Status(result))

	var products []any
	waffletest.DecodeResponse(t, result, &products)
	assert.Empty(t, products, "expected empty product list")
}

func TestProductsBlock_ListProductTypesEmpty(t *testing.T) {
	block, ctx := setupProductsBlock(t)

	msg := waffletest.Retrieve("/ext/products/product-types")
	waffletest.WithAuth(msg, "user-1", "user@test.com")

	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, waffletest.Status(result))

	var productTypes []any
	waffletest.DecodeResponse(t, result, &productTypes)
	assert.Empty(t, productTypes, "expected empty product types with noop seeder")
}

func TestProductsBlock_CreateProductValidation(t *testing.T) {
	block, ctx := setupProductsBlock(t)

	// Try creating a product without required fields
	msg := waffletest.Create("/ext/products/products", map[string]any{
		"name": "",
	})
	waffletest.WithAuth(msg, "user-1", "user@test.com")

	result := block.Handle(ctx, msg)

	assert.Equal(t, 400, waffletest.Status(result))
}

func TestProductsBlock_DeleteVariableInvalidID(t *testing.T) {
	block, ctx := setupProductsBlock(t)

	msg := waffletest.Delete("/admin/ext/products/variables/{id}")
	waffletest.WithAuth(msg, "admin-1", "admin@test.com")
	waffletest.WithRoles(msg, "admin")
	// Do not set the "id" var -- it stays as "{id}" which is not a valid uint
	// This should return 400 for invalid ID.

	result := block.Handle(ctx, msg)

	assert.Equal(t, 400, waffletest.Status(result))
}

func TestProductsBlock_DeleteVariable(t *testing.T) {
	block, ctx := setupProductsBlock(t)

	// First create a variable
	createMsg := waffletest.Create("/admin/ext/products/variables", map[string]any{
		"name":        "to_delete",
		"displayName": "To Delete",
		"valueType":   "number",
		"type":        "user",
		"status":      "active",
	})
	waffletest.WithAuth(createMsg, "admin-1", "admin@test.com")
	waffletest.WithRoles(createMsg, "admin")
	createResult := block.Handle(ctx, createMsg)
	require.Equal(t, 201, waffletest.Status(createResult))

	var created map[string]any
	waffletest.DecodeResponse(t, createResult, &created)

	// Delete the created variable -- ID is a float64 from JSON
	idFloat, ok := created["id"].(float64)
	require.True(t, ok, "expected numeric id")
	idStr := fmt.Sprintf("%d", int(idFloat))

	deleteMsg := waffletest.Delete("/admin/ext/products/variables/" + idStr)
	waffletest.WithAuth(deleteMsg, "admin-1", "admin@test.com")
	waffletest.WithRoles(deleteMsg, "admin")

	result := block.Handle(ctx, deleteMsg)

	assert.Equal(t, 204, waffletest.Status(result))
}

func TestProductsBlock_ProviderStatus(t *testing.T) {
	block, ctx := setupProductsBlock(t)

	msg := waffletest.Retrieve("/admin/ext/products/provider/status")
	waffletest.WithAuth(msg, "admin-1", "admin@test.com")
	waffletest.WithRoles(msg, "admin")

	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, waffletest.Status(result))

	var resp map[string]any
	waffletest.DecodeResponse(t, result, &resp)
	assert.Contains(t, resp, "configured")
	assert.Contains(t, resp, "provider")
}

func TestProductsBlock_ProductStats(t *testing.T) {
	block, ctx := setupProductsBlock(t)

	msg := waffletest.Retrieve("/admin/ext/products/stats")
	waffletest.WithAuth(msg, "admin-1", "admin@test.com")
	waffletest.WithRoles(msg, "admin")

	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, waffletest.Status(result))

	var resp map[string]any
	waffletest.DecodeResponse(t, result, &resp)
	assert.Contains(t, resp, "totalProducts")
	assert.Contains(t, resp, "totalGroups")
	assert.Contains(t, resp, "activeProducts")
}

func TestProductsBlock_ListPurchasesRequiresAuth(t *testing.T) {
	block, ctx := setupProductsBlock(t)

	msg := waffletest.Retrieve("/ext/products/purchases")

	result := block.Handle(ctx, msg)

	assert.Equal(t, 401, waffletest.Status(result))
}
