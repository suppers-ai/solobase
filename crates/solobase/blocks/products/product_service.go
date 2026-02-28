package products

import (
	"context"
	"encoding/json"
	"strconv"

	"github.com/suppers-ai/solobase/blocks/products/models"
	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/waffle-go/services/database"
)

// ProductService handles product operations
type ProductService struct {
	db              database.Service
	variableService *VariableService
}

func NewProductService(db database.Service, variableService *VariableService) *ProductService {
	return &ProductService{
		db:              db,
		variableService: variableService,
	}
}

func (s *ProductService) ListByGroup(groupID uint) ([]models.Product, error) {
	ctx := context.Background()
	result, err := s.db.List(ctx, "ext_products_products", &database.ListOptions{
		Filters: []database.Filter{{Field: "group_id", Operator: database.OpEqual, Value: groupID}},
		Sort:    []database.SortField{{Field: "id"}},
		Limit:   10000,
	})
	if err != nil {
		return nil, err
	}

	var products []models.Product
	for _, r := range result.Records {
		p := recordToProduct(r)
		// Load product template
		template, err := s.getProductTemplateByID(ctx, p.ProductTemplateID)
		if err == nil {
			p.ProductTemplate = *template
		}
		products = append(products, *p)
	}
	return products, nil
}

func (s *ProductService) ListByUser(userID string) ([]models.Product, error) {
	ctx := context.Background()
	records, err := s.db.QueryRaw(ctx, `
		SELECT p.id, p.group_id, p.product_template_id, p.name, p.description, p.base_price, p.base_price_cents, p.currency,
			p.filter_numeric_1, p.filter_numeric_2, p.filter_numeric_3, p.filter_numeric_4, p.filter_numeric_5,
			p.filter_text_1, p.filter_text_2, p.filter_text_3, p.filter_text_4, p.filter_text_5,
			p.filter_boolean_1, p.filter_boolean_2, p.filter_boolean_3, p.filter_boolean_4, p.filter_boolean_5,
			p.filter_enum_1, p.filter_enum_2, p.filter_enum_3, p.filter_enum_4, p.filter_enum_5,
			p.filter_location_1, p.filter_location_2, p.filter_location_3, p.filter_location_4, p.filter_location_5,
			p.custom_fields, p.variables, p.pricing_formula, p.active, p.created_at, p.updated_at
		FROM ext_products_products p
		JOIN ext_products_groups g ON p.group_id = g.id
		WHERE g.user_id = ?
		ORDER BY p.id`, userID)
	if err != nil {
		return nil, err
	}

	var products []models.Product
	for _, r := range records {
		p := recordToProduct(r)

		// Load group
		group, err := s.getGroupByID(ctx, p.GroupID)
		if err == nil {
			p.Group = *group
		}

		// Load product template
		template, err := s.getProductTemplateByID(ctx, p.ProductTemplateID)
		if err == nil {
			p.ProductTemplate = *template
		}
		products = append(products, *p)
	}
	return products, nil
}

// GetByID retrieves a product by ID
func (s *ProductService) GetByID(id uint) (*models.Product, error) {
	ctx := context.Background()
	records, err := s.db.QueryRaw(ctx, "SELECT "+productColumns+" FROM ext_products_products WHERE id = ?", id)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, database.ErrNotFound
	}

	p := recordToProduct(records[0])

	// Load product template
	template, err := s.getProductTemplateByID(ctx, p.ProductTemplateID)
	if err == nil {
		p.ProductTemplate = *template
	}

	// Load group
	group, err := s.getGroupByID(ctx, p.GroupID)
	if err == nil {
		p.Group = *group
	}

	return p, nil
}

func (s *ProductService) Create(product *models.Product) error {
	ctx := context.Background()
	now := apptime.NowString()

	// Marshal custom fields and variables
	customFieldsJSON, _ := json.Marshal(product.CustomFields)
	variablesJSON, _ := json.Marshal(product.Variables)

	// Compute BasePriceCents from BasePrice
	var basePriceCents *int64
	if product.BasePrice > 0 {
		cents := int64(product.BasePrice * 100)
		basePriceCents = &cents
	}

	_, err := s.db.ExecRaw(ctx, `
		INSERT INTO ext_products_products (
			group_id, product_template_id, name, description, base_price, base_price_cents, currency,
			filter_numeric_1, filter_numeric_2, filter_numeric_3, filter_numeric_4, filter_numeric_5,
			filter_text_1, filter_text_2, filter_text_3, filter_text_4, filter_text_5,
			filter_boolean_1, filter_boolean_2, filter_boolean_3, filter_boolean_4, filter_boolean_5,
			filter_enum_1, filter_enum_2, filter_enum_3, filter_enum_4, filter_enum_5,
			filter_location_1, filter_location_2, filter_location_3, filter_location_4, filter_location_5,
			custom_fields, variables, pricing_formula, active, created_at, updated_at
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		product.GroupID, product.ProductTemplateID, product.Name, stringPtr(product.Description),
		float64Ptr(product.BasePrice), basePriceCents, stringPtr(product.Currency),
		product.FilterNumeric1, product.FilterNumeric2, product.FilterNumeric3, product.FilterNumeric4, product.FilterNumeric5,
		product.FilterText1, product.FilterText2, product.FilterText3, product.FilterText4, product.FilterText5,
		boolPtrToInt64Ptr(product.FilterBoolean1), boolPtrToInt64Ptr(product.FilterBoolean2),
		boolPtrToInt64Ptr(product.FilterBoolean3), boolPtrToInt64Ptr(product.FilterBoolean4),
		boolPtrToInt64Ptr(product.FilterBoolean5),
		product.FilterEnum1, product.FilterEnum2, product.FilterEnum3, product.FilterEnum4, product.FilterEnum5,
		product.FilterLocation1, product.FilterLocation2, product.FilterLocation3, product.FilterLocation4, product.FilterLocation5,
		customFieldsJSON, variablesJSON, stringPtr(product.PricingFormula), boolToInt64Ptr(product.Active),
		now, now)
	if err != nil {
		return err
	}

	id, err := getLastInsertedID(ctx, s.db, "ext_products_products")
	if err != nil {
		return err
	}
	product.ID = id

	// Load the product template to get field definitions
	productTemplate, err := s.getProductTemplateByIDWithJSON(ctx, product.ProductTemplateID)
	if err == nil {
		var filterFields []models.FieldDefinition
		if err := json.Unmarshal(productTemplate.FilterFieldsSchemaJSON, &filterFields); err == nil && len(filterFields) > 0 {
			// Map field values to filter columns based on field IDs
			if customFields, ok := product.CustomFields["fields"].(map[string]interface{}); ok {
				for _, field := range filterFields {
					if value, exists := customFields[field.Name]; exists {
						applyProductFilterUpdate(ctx, s.db, product.ID, field.ID, value)
					}
				}
			}

			// Create variables for each field
			createVariablesFromFields(ctx, s.db, filterFields)
		}
	}

	return nil
}

func (s *ProductService) Update(id uint, product *models.Product) error {
	ctx := context.Background()

	// Marshal custom fields and variables
	customFieldsJSON, _ := json.Marshal(product.CustomFields)
	variablesJSON, _ := json.Marshal(product.Variables)

	// Compute BasePriceCents from BasePrice
	var basePriceCents *int64
	if product.BasePrice > 0 {
		cents := int64(product.BasePrice * 100)
		basePriceCents = &cents
	}

	_, err := s.db.ExecRaw(ctx, `
		UPDATE ext_products_products SET
			name = ?, description = ?, base_price = ?, base_price_cents = ?, currency = ?,
			filter_numeric_1 = ?, filter_numeric_2 = ?, filter_numeric_3 = ?, filter_numeric_4 = ?, filter_numeric_5 = ?,
			filter_text_1 = ?, filter_text_2 = ?, filter_text_3 = ?, filter_text_4 = ?, filter_text_5 = ?,
			filter_boolean_1 = ?, filter_boolean_2 = ?, filter_boolean_3 = ?, filter_boolean_4 = ?, filter_boolean_5 = ?,
			filter_enum_1 = ?, filter_enum_2 = ?, filter_enum_3 = ?, filter_enum_4 = ?, filter_enum_5 = ?,
			filter_location_1 = ?, filter_location_2 = ?, filter_location_3 = ?, filter_location_4 = ?, filter_location_5 = ?,
			custom_fields = ?, variables = ?, pricing_formula = ?, active = ?, updated_at = ?
		WHERE id = ?`,
		product.Name, stringPtr(product.Description), float64Ptr(product.BasePrice), basePriceCents, stringPtr(product.Currency),
		product.FilterNumeric1, product.FilterNumeric2, product.FilterNumeric3, product.FilterNumeric4, product.FilterNumeric5,
		product.FilterText1, product.FilterText2, product.FilterText3, product.FilterText4, product.FilterText5,
		boolPtrToInt64Ptr(product.FilterBoolean1), boolPtrToInt64Ptr(product.FilterBoolean2),
		boolPtrToInt64Ptr(product.FilterBoolean3), boolPtrToInt64Ptr(product.FilterBoolean4),
		boolPtrToInt64Ptr(product.FilterBoolean5),
		product.FilterEnum1, product.FilterEnum2, product.FilterEnum3, product.FilterEnum4, product.FilterEnum5,
		product.FilterLocation1, product.FilterLocation2, product.FilterLocation3, product.FilterLocation4, product.FilterLocation5,
		customFieldsJSON, variablesJSON, stringPtr(product.PricingFormula), boolToInt64Ptr(product.Active),
		apptime.NowString(), id)
	return err
}

func (s *ProductService) Delete(id uint) error {
	ctx := context.Background()
	_, err := s.db.ExecRaw(ctx, "DELETE FROM ext_products_products WHERE id = ?", id)
	return err
}

// ListAll returns all products
func (s *ProductService) ListAll() ([]models.Product, error) {
	ctx := context.Background()
	result, err := s.db.List(ctx, "ext_products_products", &database.ListOptions{
		Sort:  []database.SortField{{Field: "id"}},
		Limit: 1000,
	})
	if err != nil {
		return nil, err
	}

	var products []models.Product
	for _, r := range result.Records {
		p := recordToProduct(r)
		// Load product template
		template, err := s.getProductTemplateByID(ctx, p.ProductTemplateID)
		if err == nil {
			p.ProductTemplate = *template
		}
		products = append(products, *p)
	}
	return products, nil
}

// ListActive returns all active products
func (s *ProductService) ListActive() ([]models.Product, error) {
	ctx := context.Background()
	result, err := s.db.List(ctx, "ext_products_products", &database.ListOptions{
		Filters: []database.Filter{{Field: "active", Operator: database.OpEqual, Value: 1}},
		Sort:    []database.SortField{{Field: "id"}},
		Limit:   1000,
	})
	if err != nil {
		return nil, err
	}

	var products []models.Product
	for _, r := range result.Records {
		p := recordToProduct(r)
		// Load product template
		template, err := s.getProductTemplateByID(ctx, p.ProductTemplateID)
		if err == nil {
			p.ProductTemplate = *template
		}
		products = append(products, *p)
	}
	return products, nil
}

// ListTemplates returns all product templates
func (s *ProductService) ListTemplates() ([]models.ProductTemplate, error) {
	ctx := context.Background()
	result, err := s.db.List(ctx, "ext_products_product_templates", &database.ListOptions{
		Sort:  []database.SortField{{Field: "id"}},
		Limit: 10000,
	})
	if err != nil {
		return nil, err
	}

	var templates []models.ProductTemplate
	for _, r := range result.Records {
		templates = append(templates, *recordToProductTemplate(r))
	}
	return templates, nil
}

// GetTemplateByID returns a product template by ID
func (s *ProductService) GetTemplateByID(id uint) (*models.ProductTemplate, error) {
	ctx := context.Background()
	return s.getProductTemplateByID(ctx, id)
}

// GetTemplateByIDOrName returns a product template by ID (numeric) or name
func (s *ProductService) GetTemplateByIDOrName(idOrName string) (*models.ProductTemplate, error) {
	ctx := context.Background()

	// Try parsing as numeric ID first
	if id, err := strconv.ParseInt(idOrName, 10, 64); err == nil {
		return s.getProductTemplateByID(ctx, uint(id))
	}

	// Fall back to name lookup
	records, err := s.db.QueryRaw(ctx, "SELECT "+productTemplateColumns+" FROM ext_products_product_templates WHERE name = ?", idOrName)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, database.ErrNotFound
	}
	return recordToProductTemplate(records[0]), nil
}

// CreateTemplate creates a new product template
func (s *ProductService) CreateTemplate(template *models.ProductTemplate) error {
	ctx := context.Background()
	now := apptime.NowString()

	// Marshal JSON fields
	filterFieldsJSON, _ := json.Marshal(template.FilterFieldsSchema)
	customFieldsJSON, _ := json.Marshal(template.CustomFieldsSchema)
	pricingTemplatesJSON, _ := json.Marshal(template.PricingTemplates)

	// Convert BillingRecurringIntervalCount from *int to *int64
	var intervalCount *int64
	if template.BillingRecurringIntervalCount != nil {
		count := int64(*template.BillingRecurringIntervalCount)
		intervalCount = &count
	}

	_, err := s.db.ExecRaw(ctx, `
		INSERT INTO ext_products_product_templates (
			name, display_name, description, category, icon,
			filter_fields_schema, custom_fields_schema, pricing_templates,
			billing_mode, billing_type, billing_recurring_interval, billing_recurring_interval_count,
			status, created_at, updated_at
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		template.Name, stringPtr(template.DisplayName), stringPtr(template.Description),
		stringPtr(template.Category), stringPtr(template.Icon),
		filterFieldsJSON, customFieldsJSON, pricingTemplatesJSON,
		template.BillingMode, template.BillingType, template.BillingRecurringInterval, intervalCount,
		stringPtr(template.Status), now, now)
	if err != nil {
		return err
	}

	id, err := getLastInsertedID(ctx, s.db, "ext_products_product_templates")
	if err != nil {
		return err
	}
	template.ID = id
	return nil
}

// UpdateTemplate updates a product template
func (s *ProductService) UpdateTemplate(template *models.ProductTemplate) error {
	ctx := context.Background()

	// Marshal JSON fields
	filterFieldsJSON, _ := json.Marshal(template.FilterFieldsSchema)
	customFieldsJSON, _ := json.Marshal(template.CustomFieldsSchema)
	pricingTemplatesJSON, _ := json.Marshal(template.PricingTemplates)

	// Convert BillingRecurringIntervalCount from *int to *int64
	var intervalCount *int64
	if template.BillingRecurringIntervalCount != nil {
		count := int64(*template.BillingRecurringIntervalCount)
		intervalCount = &count
	}

	_, err := s.db.ExecRaw(ctx, `
		UPDATE ext_products_product_templates SET
			name = ?, display_name = ?, description = ?, category = ?, icon = ?,
			filter_fields_schema = ?, custom_fields_schema = ?, pricing_templates = ?,
			billing_mode = ?, billing_type = ?, billing_recurring_interval = ?, billing_recurring_interval_count = ?,
			status = ?, updated_at = ?
		WHERE id = ?`,
		template.Name, stringPtr(template.DisplayName), stringPtr(template.Description),
		stringPtr(template.Category), stringPtr(template.Icon),
		filterFieldsJSON, customFieldsJSON, pricingTemplatesJSON,
		template.BillingMode, template.BillingType, template.BillingRecurringInterval, intervalCount,
		stringPtr(template.Status), apptime.NowString(), template.ID)
	return err
}

// DeleteTemplate deletes a product template
func (s *ProductService) DeleteTemplate(id uint) error {
	ctx := context.Background()
	_, err := s.db.ExecRaw(ctx, "DELETE FROM ext_products_product_templates WHERE id = ?", id)
	return err
}

// Helper functions
func (s *ProductService) getProductTemplateByID(ctx context.Context, id uint) (*models.ProductTemplate, error) {
	records, err := s.db.QueryRaw(ctx, "SELECT "+productTemplateColumns+" FROM ext_products_product_templates WHERE id = ?", id)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, database.ErrNotFound
	}
	return recordToProductTemplate(records[0]), nil
}

func (s *ProductService) getProductTemplateByIDWithJSON(ctx context.Context, id uint) (*productTemplateWithJSON, error) {
	records, err := s.db.QueryRaw(ctx, "SELECT "+productTemplateColumns+" FROM ext_products_product_templates WHERE id = ?", id)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, database.ErrNotFound
	}
	return recordToProductTemplateWithJSON(records[0]), nil
}

func (s *ProductService) getGroupByID(ctx context.Context, id uint) (*models.Group, error) {
	records, err := s.db.QueryRaw(ctx, "SELECT "+groupColumns+" FROM ext_products_groups WHERE id = ?", id)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, database.ErrNotFound
	}
	return recordToGroup(records[0]), nil
}
