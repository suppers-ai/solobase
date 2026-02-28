package products

import (
	"context"
	"strconv"
	"strings"

	"github.com/suppers-ai/solobase/blocks/products/models"
	"github.com/suppers-ai/waffle-go/services/database"
	waffle "github.com/suppers-ai/waffle-go"
)

func (b *ProductsWaffleBlock) handleListMyProducts(_ waffle.Context, msg *waffle.Message) waffle.Result {
	userID := msg.UserID()
	if userID == "" {
		return waffle.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	products, err := b.productService.ListByUser(userID)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 200, products)
}

func (b *ProductsWaffleBlock) handleCreateProduct(_ waffle.Context, msg *waffle.Message) waffle.Result {
	userID := msg.UserID()
	if userID == "" {
		return waffle.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	var product models.Product
	if err := msg.Decode(&product); err != nil {
		return waffle.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	if product.Name == "" {
		return waffle.Error(msg, 400, "validation_error", "Name is required")
	}
	if product.GroupID == 0 {
		return waffle.Error(msg, 400, "validation_error", "Group is required")
	}
	if product.ProductTemplateID == 0 {
		return waffle.Error(msg, 400, "validation_error", "Product type is required")
	}

	// Verify user owns the group
	if _, err := b.groupService.GetByID(product.GroupID, userID); err != nil {
		if err == database.ErrNotFound {
			return waffle.Error(msg, 403, "forbidden", "You don't own this group")
		}
		return waffle.Error(msg, 500, "internal_error", "Failed to verify group ownership")
	}

	// Get the product template to validate required fields
	productTemplate, err := b.productService.GetTemplateByID(product.ProductTemplateID)
	if err != nil {
		return waffle.Error(msg, 400, "validation_error", "Product template not found")
	}

	// Validate required filter fields
	validationErrors := validateRequiredFields(&product, productTemplate)
	if len(validationErrors) > 0 {
		return waffle.Error(msg, 400, "validation_error", "Validation failed: "+strings.Join(validationErrors, ", "))
	}

	if err := b.productService.Create(&product); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 201, product)
}

func (b *ProductsWaffleBlock) handleUpdateProduct(_ waffle.Context, msg *waffle.Message) waffle.Result {
	userID := msg.UserID()
	if userID == "" {
		return waffle.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return waffle.Error(msg, 400, "invalid_id", "Invalid product ID")
	}

	existingProduct, err := b.productService.GetByID(uint(id))
	if err != nil {
		return waffle.Error(msg, 404, "not_found", "Product not found")
	}

	// Verify user owns the product's group
	if _, err := b.groupService.GetByID(existingProduct.GroupID, userID); err != nil {
		if err == database.ErrNotFound {
			return waffle.Error(msg, 403, "forbidden", "You don't own this product")
		}
		return waffle.Error(msg, 500, "internal_error", "Failed to verify ownership")
	}

	var product models.Product
	if err := msg.Decode(&product); err != nil {
		return waffle.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	if product.Name == "" {
		return waffle.Error(msg, 400, "validation_error", "Name is required")
	}

	// Get the product template to check field constraints
	productTemplate, err := b.productService.GetTemplateByID(existingProduct.ProductTemplateID)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Product template not found")
	}

	// Preserve non-editable fields
	models.PreserveNonEditableFields(&product, existingProduct, productTemplate)

	// Validate required fields
	validationErrors := validateRequiredFields(&product, productTemplate)
	if len(validationErrors) > 0 {
		return waffle.Error(msg, 400, "validation_error", "Validation failed: "+strings.Join(validationErrors, ", "))
	}

	product.ID = uint(id)
	product.GroupID = existingProduct.GroupID // Prevent changing group

	if err := b.productService.Update(uint(id), &product); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 200, product)
}

func (b *ProductsWaffleBlock) handleDeleteProduct(_ waffle.Context, msg *waffle.Message) waffle.Result {
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return waffle.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	if err := b.productService.Delete(uint(id)); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.Respond(msg, 204, nil, "")
}

func (b *ProductsWaffleBlock) handleProductStats(_ waffle.Context, msg *waffle.Message) waffle.Result {
	ctx := context.Background()

	groupCount, _ := b.db.Count(ctx, "ext_products_groups", nil)
	productCount, _ := b.db.Count(ctx, "ext_products_products", nil)
	activeProductCount, _ := b.db.Count(ctx, "ext_products_products", []database.Filter{
		{Field: "active", Operator: database.OpEqual, Value: 1},
	})

	// Total revenue from paid purchases
	revenueRecords, _ := b.db.QueryRaw(ctx,
		"SELECT COALESCE(SUM(total_cents), 0) / 100.0 as total_revenue FROM ext_products_purchases WHERE status IN (?, ?)",
		models.PurchaseStatusPaid, models.PurchaseStatusPaidPendingApproval)
	var totalRevenue float64
	if len(revenueRecords) > 0 {
		totalRevenue = toFloat64Val(revenueRecords[0].Data["total_revenue"])
	}

	// Average price
	avgRecords, _ := b.db.QueryRaw(ctx,
		"SELECT COALESCE(AVG(base_price_cents), 0) / 100.0 as avg_price FROM ext_products_products")
	var avgPrice float64
	if len(avgRecords) > 0 {
		avgPrice = toFloat64Val(avgRecords[0].Data["avg_price"])
	}

	stats := map[string]interface{}{
		"totalProducts":  productCount,
		"totalGroups":    groupCount,
		"activeProducts": activeProductCount,
		"totalRevenue":   totalRevenue,
		"avgPrice":       avgPrice,
	}
	return waffle.JSONRespond(msg, 200, stats)
}

func (b *ProductsWaffleBlock) handleProviderStatus(_ waffle.Context, msg *waffle.Message) waffle.Result {
	return waffle.JSONRespond(msg, 200, b.GetProviderStatus())
}

// validateRequiredFields validates that all required fields editable by users are filled.
func validateRequiredFields(product *models.Product, template *models.ProductTemplate) []string {
	var errs []string

	for _, field := range template.FilterFieldsSchema {
		if field.Required && field.Constraints.EditableByUser {
			structFieldName, ok := models.FilterFieldMapping[field.ID]
			if !ok {
				continue
			}
			if models.IsFilterFieldEmpty(product, structFieldName) {
				if field.Constraints.Default == nil {
					errs = append(errs, field.Name+" is required")
				} else {
					models.SetFilterFieldFromDefault(product, structFieldName, field.Constraints.Default)
				}
			}
		}
	}

	for _, field := range template.CustomFieldsSchema {
		if field.Required && field.Constraints.EditableByUser {
			needsValue := false
			if product.CustomFields == nil {
				product.CustomFields = make(map[string]interface{})
				needsValue = true
			} else if val, exists := product.CustomFields[field.ID]; !exists || val == nil || val == "" {
				needsValue = true
			}

			if needsValue {
				if field.Constraints.Default != nil {
					product.CustomFields[field.ID] = field.Constraints.Default
				} else {
					errs = append(errs, field.Name+" is required")
				}
			}
		}
	}

	return errs
}
