package products

import (
	"context"
	"database/sql"
	"encoding/json"
	"strconv"
	"strings"

	"github.com/suppers-ai/solobase/extensions/official/products/models"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
)

// VariableService handles variable operations
type VariableService struct {
	sqlDB   *sql.DB
	queries *db.Queries
}

func NewVariableService(sqlDB *sql.DB) *VariableService {
	return &VariableService{
		sqlDB:   sqlDB,
		queries: db.New(sqlDB),
	}
}

// GetSystemVariables returns hard-coded system variables
func GetSystemVariables() []models.Variable {
	return []models.Variable{
		{
			Name:        "running_total",
			DisplayName: "Running Total",
			ValueType:   "number",
			Type:        "system",
			Description: "Accumulated total from previous calculations",
			Status:      "active",
		},
	}
}

func (s *VariableService) List() ([]models.Variable, error) {
	ctx := context.Background()
	dbVariables, err := s.queries.ListVariables(ctx)
	if err != nil {
		return nil, err
	}

	// Convert to models
	userVariables := make([]models.Variable, len(dbVariables))
	for i, v := range dbVariables {
		userVariables[i] = dbVariableToModel(v)
	}

	// Combine user variables from DB with hard-coded system variables
	allVariables := append(userVariables, GetSystemVariables()...)
	return allVariables, nil
}

func (s *VariableService) Create(variable *models.Variable) error {
	ctx := context.Background()
	now := apptime.NowTime()

	// Convert DefaultValue from interface{} to string
	var defaultValueStr *string
	if variable.DefaultValue != nil {
		if s, ok := variable.DefaultValue.(string); ok {
			defaultValueStr = &s
		}
	}

	dbVar, err := s.queries.CreateVariable(ctx, db.CreateVariableParams{
		Name:         variable.Name,
		DisplayName:  stringPtr(variable.DisplayName),
		ValueType:    stringPtr(variable.ValueType),
		Type:         stringPtr(variable.Type),
		DefaultValue: defaultValueStr,
		Description:  stringPtr(variable.Description),
		Status:       stringPtr(variable.Status),
		CreatedAt:    apptime.Format(now),
		UpdatedAt:    apptime.Format(now),
	})
	if err != nil {
		return err
	}

	variable.ID = uint(dbVar.ID)
	return nil
}

func (s *VariableService) CreateFromField(field models.FieldDefinition) (*models.Variable, error) {
	variable := &models.Variable{
		Name:         field.Name,
		DisplayName:  field.Name,
		ValueType:    field.Type,
		Type:         "user",
		Description:  field.Description,
		DefaultValue: field.Constraints.Default,
		Status:       "active",
	}

	if err := s.Create(variable); err != nil {
		return nil, err
	}
	return variable, nil
}

func (s *VariableService) Update(id uint, variable *models.Variable) error {
	ctx := context.Background()

	// Convert DefaultValue from interface{} to string
	var defaultValueStr *string
	if variable.DefaultValue != nil {
		if str, ok := variable.DefaultValue.(string); ok {
			defaultValueStr = &str
		}
	}

	return s.queries.UpdateVariable(ctx, db.UpdateVariableParams{
		Name:         variable.Name,
		DisplayName:  stringPtr(variable.DisplayName),
		ValueType:    stringPtr(variable.ValueType),
		Type:         stringPtr(variable.Type),
		DefaultValue: defaultValueStr,
		Description:  stringPtr(variable.Description),
		Status:       stringPtr(variable.Status),
		UpdatedAt:   apptime.NowString(),
		ID:           int64(id),
	})
}

func (s *VariableService) Delete(id uint) error {
	ctx := context.Background()
	return s.queries.DeleteVariable(ctx, int64(id))
}

// GroupService handles group operations
type GroupService struct {
	sqlDB           *sql.DB
	queries         *db.Queries
	variableService *VariableService
}

func NewGroupService(sqlDB *sql.DB) *GroupService {
	return &GroupService{
		sqlDB:           sqlDB,
		queries:         db.New(sqlDB),
		variableService: NewVariableService(sqlDB),
	}
}

func (s *GroupService) ListByUser(userID string) ([]models.Group, error) {
	ctx := context.Background()
	dbGroups, err := s.queries.ListGroupsByUserID(ctx, userID)
	if err != nil {
		return nil, err
	}

	groups := make([]models.Group, len(dbGroups))
	for i, g := range dbGroups {
		groups[i] = dbGroupToModel(g)
		// Load group template
		template, err := s.queries.GetGroupTemplateByID(ctx, g.GroupTemplateID)
		if err == nil {
			groups[i].GroupTemplate = dbGroupTemplateToModel(template)
		}
	}
	return groups, nil
}

func (s *GroupService) Create(group *models.Group) error {
	ctx := context.Background()
	tx, err := s.sqlDB.BeginTx(ctx, nil)
	if err != nil {
		return err
	}
	defer tx.Rollback()

	qtx := s.queries.WithTx(tx)
	now := apptime.NowTime()

	// Marshal custom fields
	customFieldsJSON, _ := json.Marshal(group.CustomFields)

	dbGroup, err := qtx.CreateGroup(ctx, db.CreateGroupParams{
		UserID:          group.UserID,
		GroupTemplateID: int64(group.GroupTemplateID),
		Name:            group.Name,
		Description:     stringPtr(group.Description),
		FilterNumeric1:  group.FilterNumeric1,
		FilterNumeric2:  group.FilterNumeric2,
		FilterNumeric3:  group.FilterNumeric3,
		FilterNumeric4:  group.FilterNumeric4,
		FilterNumeric5:  group.FilterNumeric5,
		FilterText1:     group.FilterText1,
		FilterText2:     group.FilterText2,
		FilterText3:     group.FilterText3,
		FilterText4:     group.FilterText4,
		FilterText5:     group.FilterText5,
		FilterBoolean1:  boolPtrToInt64Ptr(group.FilterBoolean1),
		FilterBoolean2:  boolPtrToInt64Ptr(group.FilterBoolean2),
		FilterBoolean3:  boolPtrToInt64Ptr(group.FilterBoolean3),
		FilterBoolean4:  boolPtrToInt64Ptr(group.FilterBoolean4),
		FilterBoolean5:  boolPtrToInt64Ptr(group.FilterBoolean5),
		FilterEnum1:     group.FilterEnum1,
		FilterEnum2:     group.FilterEnum2,
		FilterEnum3:     group.FilterEnum3,
		FilterEnum4:     group.FilterEnum4,
		FilterEnum5:     group.FilterEnum5,
		FilterLocation1: group.FilterLocation1,
		FilterLocation2: group.FilterLocation2,
		FilterLocation3: group.FilterLocation3,
		FilterLocation4: group.FilterLocation4,
		FilterLocation5: group.FilterLocation5,
		CustomFields:    customFieldsJSON,
		CreatedAt:       apptime.Format(now),
		UpdatedAt:       apptime.Format(now),
	})
	if err != nil {
		return err
	}

	group.ID = uint(dbGroup.ID)

	// Load the group template to get field definitions
	groupTemplate, err := qtx.GetGroupTemplateByID(ctx, int64(group.GroupTemplateID))
	if err == nil {
		var filterFields []models.FieldDefinition
		if err := json.Unmarshal(groupTemplate.FilterFieldsSchema, &filterFields); err == nil && len(filterFields) > 0 {
			// Map field values to filter columns based on field IDs
			if customFields, ok := group.CustomFields["fields"].(map[string]interface{}); ok {
				for _, field := range filterFields {
					if value, exists := customFields[field.Name]; exists {
						applyGroupFilterUpdate(ctx, tx, group.ID, field.ID, value)
					}
				}
			}

			// Create variables for each field
			createVariablesFromFieldsTx(ctx, qtx, filterFields)
		}
	}

	return tx.Commit()
}

func (s *GroupService) Update(id uint, userID string, group *models.Group) error {
	ctx := context.Background()

	// Marshal custom fields
	customFieldsJSON, _ := json.Marshal(group.CustomFields)

	// Get current group first to verify ownership
	current, err := s.queries.GetGroupByID(ctx, int64(id))
	if err != nil {
		return err
	}
	if current.UserID != userID {
		return sql.ErrNoRows // User doesn't own this group
	}

	return s.queries.UpdateGroup(ctx, db.UpdateGroupParams{
		Name:            group.Name,
		Description:     stringPtr(group.Description),
		FilterNumeric1:  group.FilterNumeric1,
		FilterNumeric2:  group.FilterNumeric2,
		FilterNumeric3:  group.FilterNumeric3,
		FilterNumeric4:  group.FilterNumeric4,
		FilterNumeric5:  group.FilterNumeric5,
		FilterText1:     group.FilterText1,
		FilterText2:     group.FilterText2,
		FilterText3:     group.FilterText3,
		FilterText4:     group.FilterText4,
		FilterText5:     group.FilterText5,
		FilterBoolean1:  boolPtrToInt64Ptr(group.FilterBoolean1),
		FilterBoolean2:  boolPtrToInt64Ptr(group.FilterBoolean2),
		FilterBoolean3:  boolPtrToInt64Ptr(group.FilterBoolean3),
		FilterBoolean4:  boolPtrToInt64Ptr(group.FilterBoolean4),
		FilterBoolean5:  boolPtrToInt64Ptr(group.FilterBoolean5),
		FilterEnum1:     group.FilterEnum1,
		FilterEnum2:     group.FilterEnum2,
		FilterEnum3:     group.FilterEnum3,
		FilterEnum4:     group.FilterEnum4,
		FilterEnum5:     group.FilterEnum5,
		FilterLocation1: group.FilterLocation1,
		FilterLocation2: group.FilterLocation2,
		FilterLocation3: group.FilterLocation3,
		FilterLocation4: group.FilterLocation4,
		FilterLocation5: group.FilterLocation5,
		CustomFields:    customFieldsJSON,
		UpdatedAt:   apptime.NowString(),
		ID:              int64(id),
	})
}

func (s *GroupService) Delete(id uint, userID string) error {
	ctx := context.Background()

	// Verify ownership first
	current, err := s.queries.GetGroupByID(ctx, int64(id))
	if err != nil {
		return err
	}
	if current.UserID != userID {
		return sql.ErrNoRows
	}

	return s.queries.DeleteGroup(ctx, int64(id))
}

func (s *GroupService) GetByID(id uint, userID string) (*models.Group, error) {
	ctx := context.Background()
	dbGroup, err := s.queries.GetGroupByID(ctx, int64(id))
	if err != nil {
		return nil, err
	}

	if dbGroup.UserID != userID {
		return nil, sql.ErrNoRows
	}

	group := dbGroupToModel(dbGroup)

	// Load group template
	template, err := s.queries.GetGroupTemplateByID(ctx, dbGroup.GroupTemplateID)
	if err == nil {
		group.GroupTemplate = dbGroupTemplateToModel(template)
	}

	return &group, nil
}

// ListAll returns all groups (admin function)
func (s *GroupService) ListAll() ([]models.Group, error) {
	ctx := context.Background()
	dbGroups, err := s.queries.ListGroups(ctx, db.ListGroupsParams{
		Limit:  1000, // Get all groups (reasonable max)
		Offset: 0,
	})
	if err != nil {
		return nil, err
	}

	groups := make([]models.Group, len(dbGroups))
	for i, g := range dbGroups {
		groups[i] = dbGroupToModel(g)
		// Load group template
		template, err := s.queries.GetGroupTemplateByID(ctx, g.GroupTemplateID)
		if err == nil {
			groups[i].GroupTemplate = dbGroupTemplateToModel(template)
		}
	}
	return groups, nil
}

// UpdateAdmin updates a group (admin function - no user check)
func (s *GroupService) UpdateAdmin(id uint, group *models.Group) error {
	ctx := context.Background()

	// Marshal custom fields
	customFieldsJSON, _ := json.Marshal(group.CustomFields)

	return s.queries.UpdateGroup(ctx, db.UpdateGroupParams{
		Name:            group.Name,
		Description:     stringPtr(group.Description),
		FilterNumeric1:  group.FilterNumeric1,
		FilterNumeric2:  group.FilterNumeric2,
		FilterNumeric3:  group.FilterNumeric3,
		FilterNumeric4:  group.FilterNumeric4,
		FilterNumeric5:  group.FilterNumeric5,
		FilterText1:     group.FilterText1,
		FilterText2:     group.FilterText2,
		FilterText3:     group.FilterText3,
		FilterText4:     group.FilterText4,
		FilterText5:     group.FilterText5,
		FilterBoolean1:  boolPtrToInt64Ptr(group.FilterBoolean1),
		FilterBoolean2:  boolPtrToInt64Ptr(group.FilterBoolean2),
		FilterBoolean3:  boolPtrToInt64Ptr(group.FilterBoolean3),
		FilterBoolean4:  boolPtrToInt64Ptr(group.FilterBoolean4),
		FilterBoolean5:  boolPtrToInt64Ptr(group.FilterBoolean5),
		FilterEnum1:     group.FilterEnum1,
		FilterEnum2:     group.FilterEnum2,
		FilterEnum3:     group.FilterEnum3,
		FilterEnum4:     group.FilterEnum4,
		FilterEnum5:     group.FilterEnum5,
		FilterLocation1: group.FilterLocation1,
		FilterLocation2: group.FilterLocation2,
		FilterLocation3: group.FilterLocation3,
		FilterLocation4: group.FilterLocation4,
		FilterLocation5: group.FilterLocation5,
		CustomFields:    customFieldsJSON,
		UpdatedAt:   apptime.NowString(),
		ID:              int64(id),
	})
}

// DeleteAdmin deletes a group (admin function - no user check)
func (s *GroupService) DeleteAdmin(id uint) error {
	ctx := context.Background()
	return s.queries.DeleteGroup(ctx, int64(id))
}

// ProductService handles product operations
type ProductService struct {
	sqlDB           *sql.DB
	queries         *db.Queries
	variableService *VariableService
}

func NewProductService(sqlDB *sql.DB, variableService *VariableService) *ProductService {
	return &ProductService{
		sqlDB:           sqlDB,
		queries:         db.New(sqlDB),
		variableService: variableService,
	}
}

func (s *ProductService) ListByGroup(groupID uint) ([]models.Product, error) {
	ctx := context.Background()
	dbProducts, err := s.queries.ListProductsByGroupID(ctx, int64(groupID))
	if err != nil {
		return nil, err
	}

	products := make([]models.Product, len(dbProducts))
	for i, p := range dbProducts {
		products[i] = dbProductToModel(p)
		// Load product template
		template, err := s.queries.GetProductTemplateByID(ctx, p.ProductTemplateID)
		if err == nil {
			products[i].ProductTemplate = dbProductTemplateToModel(template)
		}
	}
	return products, nil
}

func (s *ProductService) ListByUser(userID string) ([]models.Product, error) {
	ctx := context.Background()
	dbProducts, err := s.queries.ListProductsByUserID(ctx, userID)
	if err != nil {
		return nil, err
	}

	products := make([]models.Product, len(dbProducts))
	for i, p := range dbProducts {
		products[i] = dbProductToModel(p)

		// Load group
		group, err := s.queries.GetGroupByID(ctx, p.GroupID)
		if err == nil {
			grp := dbGroupToModel(group)
			products[i].Group = grp
		}

		// Load product template
		template, err := s.queries.GetProductTemplateByID(ctx, p.ProductTemplateID)
		if err == nil {
			products[i].ProductTemplate = dbProductTemplateToModel(template)
		}
	}
	return products, nil
}

// GetByID retrieves a product by ID
func (s *ProductService) GetByID(id uint) (*models.Product, error) {
	ctx := context.Background()
	dbProduct, err := s.queries.GetProductByID(ctx, int64(id))
	if err != nil {
		return nil, err
	}

	product := dbProductToModel(dbProduct)

	// Load product template
	template, err := s.queries.GetProductTemplateByID(ctx, dbProduct.ProductTemplateID)
	if err == nil {
		product.ProductTemplate = dbProductTemplateToModel(template)
	}

	// Load group
	group, err := s.queries.GetGroupByID(ctx, dbProduct.GroupID)
	if err == nil {
		grp := dbGroupToModel(group)
		product.Group = grp
	}

	return &product, nil
}

func (s *ProductService) Create(product *models.Product) error {
	ctx := context.Background()
	tx, err := s.sqlDB.BeginTx(ctx, nil)
	if err != nil {
		return err
	}
	defer tx.Rollback()

	qtx := s.queries.WithTx(tx)
	now := apptime.NowTime()

	// Marshal custom fields and variables
	customFieldsJSON, _ := json.Marshal(product.CustomFields)
	variablesJSON, _ := json.Marshal(product.Variables)

	// Compute BasePriceCents from BasePrice
	var basePriceCents *int64
	if product.BasePrice > 0 {
		cents := int64(product.BasePrice * 100)
		basePriceCents = &cents
	}

	dbProduct, err := qtx.CreateProduct(ctx, db.CreateProductParams{
		GroupID:           int64(product.GroupID),
		ProductTemplateID: int64(product.ProductTemplateID),
		Name:              product.Name,
		Description:       stringPtr(product.Description),
		BasePrice:         float64Ptr(product.BasePrice),
		BasePriceCents:    basePriceCents,
		Currency:          stringPtr(product.Currency),
		FilterNumeric1:    product.FilterNumeric1,
		FilterNumeric2:    product.FilterNumeric2,
		FilterNumeric3:    product.FilterNumeric3,
		FilterNumeric4:    product.FilterNumeric4,
		FilterNumeric5:    product.FilterNumeric5,
		FilterText1:       product.FilterText1,
		FilterText2:       product.FilterText2,
		FilterText3:       product.FilterText3,
		FilterText4:       product.FilterText4,
		FilterText5:       product.FilterText5,
		FilterBoolean1:    boolPtrToInt64Ptr(product.FilterBoolean1),
		FilterBoolean2:    boolPtrToInt64Ptr(product.FilterBoolean2),
		FilterBoolean3:    boolPtrToInt64Ptr(product.FilterBoolean3),
		FilterBoolean4:    boolPtrToInt64Ptr(product.FilterBoolean4),
		FilterBoolean5:    boolPtrToInt64Ptr(product.FilterBoolean5),
		FilterEnum1:       product.FilterEnum1,
		FilterEnum2:       product.FilterEnum2,
		FilterEnum3:       product.FilterEnum3,
		FilterEnum4:       product.FilterEnum4,
		FilterEnum5:       product.FilterEnum5,
		FilterLocation1:   product.FilterLocation1,
		FilterLocation2:   product.FilterLocation2,
		FilterLocation3:   product.FilterLocation3,
		FilterLocation4:   product.FilterLocation4,
		FilterLocation5:   product.FilterLocation5,
		CustomFields:      customFieldsJSON,
		Variables:         variablesJSON,
		PricingFormula:    stringPtr(product.PricingFormula),
		Active:            boolToInt64Ptr(product.Active),
		CreatedAt:         apptime.Format(now),
		UpdatedAt:         apptime.Format(now),
	})
	if err != nil {
		return err
	}

	product.ID = uint(dbProduct.ID)

	// Load the product template to get field definitions
	productTemplate, err := qtx.GetProductTemplateByID(ctx, int64(product.ProductTemplateID))
	if err == nil {
		var filterFields []models.FieldDefinition
		if err := json.Unmarshal(productTemplate.FilterFieldsSchema, &filterFields); err == nil && len(filterFields) > 0 {
			// Map field values to filter columns based on field IDs
			if customFields, ok := product.CustomFields["fields"].(map[string]interface{}); ok {
				for _, field := range filterFields {
					if value, exists := customFields[field.Name]; exists {
						applyProductFilterUpdate(ctx, tx, product.ID, field.ID, value)
					}
				}
			}

			// Create variables for each field
			createVariablesFromFieldsTx(ctx, qtx, filterFields)
		}
	}

	return tx.Commit()
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

	return s.queries.UpdateProduct(ctx, db.UpdateProductParams{
		Name:            product.Name,
		Description:     stringPtr(product.Description),
		BasePrice:       float64Ptr(product.BasePrice),
		BasePriceCents:  basePriceCents,
		Currency:        stringPtr(product.Currency),
		FilterNumeric1:  product.FilterNumeric1,
		FilterNumeric2:  product.FilterNumeric2,
		FilterNumeric3:  product.FilterNumeric3,
		FilterNumeric4:  product.FilterNumeric4,
		FilterNumeric5:  product.FilterNumeric5,
		FilterText1:     product.FilterText1,
		FilterText2:     product.FilterText2,
		FilterText3:     product.FilterText3,
		FilterText4:     product.FilterText4,
		FilterText5:     product.FilterText5,
		FilterBoolean1:  boolPtrToInt64Ptr(product.FilterBoolean1),
		FilterBoolean2:  boolPtrToInt64Ptr(product.FilterBoolean2),
		FilterBoolean3:  boolPtrToInt64Ptr(product.FilterBoolean3),
		FilterBoolean4:  boolPtrToInt64Ptr(product.FilterBoolean4),
		FilterBoolean5:  boolPtrToInt64Ptr(product.FilterBoolean5),
		FilterEnum1:     product.FilterEnum1,
		FilterEnum2:     product.FilterEnum2,
		FilterEnum3:     product.FilterEnum3,
		FilterEnum4:     product.FilterEnum4,
		FilterEnum5:     product.FilterEnum5,
		FilterLocation1: product.FilterLocation1,
		FilterLocation2: product.FilterLocation2,
		FilterLocation3: product.FilterLocation3,
		FilterLocation4: product.FilterLocation4,
		FilterLocation5: product.FilterLocation5,
		CustomFields:    customFieldsJSON,
		Variables:       variablesJSON,
		PricingFormula:  stringPtr(product.PricingFormula),
		Active:          boolToInt64Ptr(product.Active),
		UpdatedAt:   apptime.NowString(),
		ID:              int64(id),
	})
}

func (s *ProductService) Delete(id uint) error {
	ctx := context.Background()
	return s.queries.DeleteProduct(ctx, int64(id))
}

// ListAll returns all products
func (s *ProductService) ListAll() ([]models.Product, error) {
	ctx := context.Background()
	dbProducts, err := s.queries.ListProducts(ctx, db.ListProductsParams{
		Limit:  1000, // Get all products (reasonable max)
		Offset: 0,
	})
	if err != nil {
		return nil, err
	}

	products := make([]models.Product, len(dbProducts))
	for i, p := range dbProducts {
		products[i] = dbProductToModel(p)
		// Load product template
		template, err := s.queries.GetProductTemplateByID(ctx, p.ProductTemplateID)
		if err == nil {
			products[i].ProductTemplate = dbProductTemplateToModel(template)
		}
	}
	return products, nil
}

// ListActive returns all active products
func (s *ProductService) ListActive() ([]models.Product, error) {
	ctx := context.Background()
	dbProducts, err := s.queries.ListActiveProducts(ctx, db.ListActiveProductsParams{
		Limit:  1000, // Get all active products (reasonable max)
		Offset: 0,
	})
	if err != nil {
		return nil, err
	}

	products := make([]models.Product, len(dbProducts))
	for i, p := range dbProducts {
		products[i] = dbProductToModel(p)
		// Load product template
		template, err := s.queries.GetProductTemplateByID(ctx, p.ProductTemplateID)
		if err == nil {
			products[i].ProductTemplate = dbProductTemplateToModel(template)
		}
	}
	return products, nil
}

// ListTemplates returns all product templates
func (s *ProductService) ListTemplates() ([]models.ProductTemplate, error) {
	ctx := context.Background()
	dbTemplates, err := s.queries.ListProductTemplates(ctx)
	if err != nil {
		return nil, err
	}

	templates := make([]models.ProductTemplate, len(dbTemplates))
	for i, t := range dbTemplates {
		templates[i] = dbProductTemplateToModel(t)
	}
	return templates, nil
}

// GetTemplateByID returns a product template by ID
func (s *ProductService) GetTemplateByID(id uint) (*models.ProductTemplate, error) {
	ctx := context.Background()
	dbTemplate, err := s.queries.GetProductTemplateByID(ctx, int64(id))
	if err != nil {
		return nil, err
	}
	template := dbProductTemplateToModel(dbTemplate)
	return &template, nil
}

// GetTemplateByIDOrName returns a product template by ID (numeric) or name
func (s *ProductService) GetTemplateByIDOrName(idOrName string) (*models.ProductTemplate, error) {
	ctx := context.Background()

	// Try parsing as numeric ID first
	if id, err := strconv.ParseInt(idOrName, 10, 64); err == nil {
		dbTemplate, err := s.queries.GetProductTemplateByID(ctx, id)
		if err != nil {
			return nil, err
		}
		template := dbProductTemplateToModel(dbTemplate)
		return &template, nil
	}

	// Fall back to name lookup
	dbTemplate, err := s.queries.GetProductTemplateByName(ctx, idOrName)
	if err != nil {
		return nil, err
	}
	template := dbProductTemplateToModel(dbTemplate)
	return &template, nil
}

// CreateTemplate creates a new product template
func (s *ProductService) CreateTemplate(template *models.ProductTemplate) error {
	ctx := context.Background()
	now := apptime.NowTime()

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

	dbTemplate, err := s.queries.CreateProductTemplate(ctx, db.CreateProductTemplateParams{
		Name:                          template.Name,
		DisplayName:                   stringPtr(template.DisplayName),
		Description:                   stringPtr(template.Description),
		Category:                      stringPtr(template.Category),
		Icon:                          stringPtr(template.Icon),
		FilterFieldsSchema:            filterFieldsJSON,
		CustomFieldsSchema:            customFieldsJSON,
		PricingTemplates:              pricingTemplatesJSON,
		BillingMode:                   template.BillingMode,
		BillingType:                   template.BillingType,
		BillingRecurringInterval:      template.BillingRecurringInterval,
		BillingRecurringIntervalCount: intervalCount,
		Status:                        stringPtr(template.Status),
		CreatedAt:                     apptime.Format(now),
		UpdatedAt:                     apptime.Format(now),
	})
	if err != nil {
		return err
	}

	template.ID = uint(dbTemplate.ID)
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

	return s.queries.UpdateProductTemplate(ctx, db.UpdateProductTemplateParams{
		Name:                          template.Name,
		DisplayName:                   stringPtr(template.DisplayName),
		Description:                   stringPtr(template.Description),
		Category:                      stringPtr(template.Category),
		Icon:                          stringPtr(template.Icon),
		FilterFieldsSchema:            filterFieldsJSON,
		CustomFieldsSchema:            customFieldsJSON,
		PricingTemplates:              pricingTemplatesJSON,
		BillingMode:                   template.BillingMode,
		BillingType:                   template.BillingType,
		BillingRecurringInterval:      template.BillingRecurringInterval,
		BillingRecurringIntervalCount: intervalCount,
		Status:                        stringPtr(template.Status),
		UpdatedAt:   apptime.NowString(),
		ID:                            int64(template.ID),
	})
}

// DeleteTemplate deletes a product template
func (s *ProductService) DeleteTemplate(id uint) error {
	ctx := context.Background()
	return s.queries.DeleteProductTemplate(ctx, int64(id))
}

// PricingService handles pricing calculations
type PricingService struct {
	sqlDB           *sql.DB
	queries         *db.Queries
	variableService *VariableService
}

func NewPricingService(sqlDB *sql.DB, variableService *VariableService) *PricingService {
	return &PricingService{
		sqlDB:           sqlDB,
		queries:         db.New(sqlDB),
		variableService: variableService,
	}
}

func (s *PricingService) CalculatePrice(productID uint, variables map[string]interface{}) (float64, error) {
	ctx := context.Background()
	product, err := s.queries.GetProductByID(ctx, int64(productID))
	if err != nil {
		return 0, err
	}

	basePrice := float64(0)
	if product.BasePrice != nil {
		basePrice = *product.BasePrice
	}

	if quantity, ok := variables["quantity"].(float64); ok {
		return basePrice * quantity, nil
	}

	return basePrice, nil
}

// ListTemplates returns all pricing templates
func (s *PricingService) ListTemplates() ([]models.PricingTemplate, error) {
	ctx := context.Background()
	dbTemplates, err := s.queries.ListPricingTemplates(ctx)
	if err != nil {
		return nil, err
	}

	templates := make([]models.PricingTemplate, len(dbTemplates))
	for i, t := range dbTemplates {
		templates[i] = dbPricingTemplateToModel(t)
	}
	return templates, nil
}

// CreateTemplate creates a new pricing template
func (s *PricingService) CreateTemplate(template *models.PricingTemplate) error {
	ctx := context.Background()
	now := apptime.NowTime()

	// Marshal variables
	variablesJSON, _ := json.Marshal(template.Variables)

	dbTemplate, err := s.queries.CreatePricingTemplate(ctx, db.CreatePricingTemplateParams{
		Name:             template.Name,
		DisplayName:      stringPtr(template.DisplayName),
		Description:      stringPtr(template.Description),
		PriceFormula:     template.PriceFormula,
		ConditionFormula: stringPtr(template.ConditionFormula),
		Variables:        variablesJSON,
		Category:         stringPtr(template.Category),
		Status:           stringPtr(template.Status),
		CreatedAt:        apptime.Format(now),
		UpdatedAt:        apptime.Format(now),
	})
	if err != nil {
		return err
	}

	template.ID = uint(dbTemplate.ID)
	return nil
}

// UpdateTemplate updates a pricing template
func (s *PricingService) UpdateTemplate(template *models.PricingTemplate) error {
	ctx := context.Background()

	// Marshal variables
	variablesJSON, _ := json.Marshal(template.Variables)

	return s.queries.UpdatePricingTemplate(ctx, db.UpdatePricingTemplateParams{
		Name:             template.Name,
		DisplayName:      stringPtr(template.DisplayName),
		Description:      stringPtr(template.Description),
		PriceFormula:     template.PriceFormula,
		ConditionFormula: stringPtr(template.ConditionFormula),
		Variables:        variablesJSON,
		Category:         stringPtr(template.Category),
		Status:           stringPtr(template.Status),
		UpdatedAt:   apptime.NowString(),
		ID:               int64(template.ID),
	})
}

// DeleteTemplate deletes a pricing template
func (s *PricingService) DeleteTemplate(id uint) error {
	ctx := context.Background()
	return s.queries.DeletePricingTemplate(ctx, int64(id))
}

// dbPricingTemplateToModel converts a database pricing template to model
func dbPricingTemplateToModel(t db.ExtProductsPricingTemplate) models.PricingTemplate {
	m := models.PricingTemplate{
		ID:           uint(t.ID),
		Name:         t.Name,
		PriceFormula: t.PriceFormula,
		CreatedAt:    apptime.NewTime(apptime.MustParse(t.CreatedAt)),
		UpdatedAt:    apptime.NewTime(apptime.MustParse(t.UpdatedAt)),
	}
	if t.DisplayName != nil {
		m.DisplayName = *t.DisplayName
	}
	if t.Description != nil {
		m.Description = *t.Description
	}
	if t.ConditionFormula != nil {
		m.ConditionFormula = *t.ConditionFormula
	}
	if t.Category != nil {
		m.Category = *t.Category
	}
	if t.Status != nil {
		m.Status = *t.Status
	}
	if t.Variables != nil {
		json.Unmarshal(t.Variables, &m.Variables)
	}
	return m
}

// Helper functions

func stringPtr(s string) *string {
	if s == "" {
		return nil
	}
	return &s
}

func float64Ptr(f float64) *float64 {
	if f == 0 {
		return nil
	}
	return &f
}

func float64PtrToPtr(f *float64) *float64 {
	return f
}

func int64Ptr(i int64) *int64 {
	if i == 0 {
		return nil
	}
	return &i
}

func boolToInt64Ptr(b bool) *int64 {
	if !b {
		return nil
	}
	v := int64(1)
	return &v
}

func boolPtrToInt64Ptr(b *bool) *int64 {
	if b == nil || !*b {
		return nil
	}
	v := int64(1)
	return &v
}

func int64PtrToBoolPtr(i *int64) *bool {
	if i == nil {
		return nil
	}
	b := *i == 1
	return &b
}

func dbVariableToModel(v db.ExtProductsVariable) models.Variable {
	m := models.Variable{
		ID:   uint(v.ID),
		Name: v.Name,
	}
	if v.DisplayName != nil {
		m.DisplayName = *v.DisplayName
	}
	if v.ValueType != nil {
		m.ValueType = *v.ValueType
	}
	if v.Type != nil {
		m.Type = *v.Type
	}
	if v.DefaultValue != nil {
		m.DefaultValue = *v.DefaultValue
	}
	if v.Description != nil {
		m.Description = *v.Description
	}
	if v.Status != nil {
		m.Status = *v.Status
	}
	return m
}

func dbGroupToModel(g db.ExtProductsGroup) models.Group {
	m := models.Group{
		ID:              uint(g.ID),
		UserID:          g.UserID,
		GroupTemplateID: uint(g.GroupTemplateID),
		Name:            g.Name,
		CreatedAt:       apptime.NewTime(apptime.MustParse(g.CreatedAt)),
		UpdatedAt:       apptime.NewTime(apptime.MustParse(g.UpdatedAt)),
	}
	if g.Description != nil {
		m.Description = *g.Description
	}
	// Copy pointer fields directly
	m.FilterNumeric1 = g.FilterNumeric1
	m.FilterNumeric2 = g.FilterNumeric2
	m.FilterNumeric3 = g.FilterNumeric3
	m.FilterNumeric4 = g.FilterNumeric4
	m.FilterNumeric5 = g.FilterNumeric5
	m.FilterText1 = g.FilterText1
	m.FilterText2 = g.FilterText2
	m.FilterText3 = g.FilterText3
	m.FilterText4 = g.FilterText4
	m.FilterText5 = g.FilterText5
	// Convert int64 to bool for boolean fields
	m.FilterBoolean1 = int64PtrToBoolPtr(g.FilterBoolean1)
	m.FilterBoolean2 = int64PtrToBoolPtr(g.FilterBoolean2)
	m.FilterBoolean3 = int64PtrToBoolPtr(g.FilterBoolean3)
	m.FilterBoolean4 = int64PtrToBoolPtr(g.FilterBoolean4)
	m.FilterBoolean5 = int64PtrToBoolPtr(g.FilterBoolean5)
	m.FilterEnum1 = g.FilterEnum1
	m.FilterEnum2 = g.FilterEnum2
	m.FilterEnum3 = g.FilterEnum3
	m.FilterEnum4 = g.FilterEnum4
	m.FilterEnum5 = g.FilterEnum5
	m.FilterLocation1 = g.FilterLocation1
	m.FilterLocation2 = g.FilterLocation2
	m.FilterLocation3 = g.FilterLocation3
	m.FilterLocation4 = g.FilterLocation4
	m.FilterLocation5 = g.FilterLocation5
	if g.CustomFields != nil {
		json.Unmarshal(g.CustomFields, &m.CustomFields)
	}
	return m
}

func dbGroupTemplateToModel(t db.ExtProductsGroupTemplate) models.GroupTemplate {
	m := models.GroupTemplate{
		ID:        uint(t.ID),
		Name:      t.Name,
		CreatedAt: apptime.NewTime(apptime.MustParse(t.CreatedAt)),
		UpdatedAt: apptime.NewTime(apptime.MustParse(t.UpdatedAt)),
	}
	if t.DisplayName != nil {
		m.DisplayName = *t.DisplayName
	}
	if t.Description != nil {
		m.Description = *t.Description
	}
	if t.Icon != nil {
		m.Icon = *t.Icon
	}
	if t.Status != nil {
		m.Status = *t.Status
	}
	if t.FilterFieldsSchema != nil {
		json.Unmarshal(t.FilterFieldsSchema, &m.FilterFieldsSchema)
	}
	return m
}

func dbProductToModel(p db.ExtProductsProduct) models.Product {
	m := models.Product{
		ID:                uint(p.ID),
		GroupID:           uint(p.GroupID),
		ProductTemplateID: uint(p.ProductTemplateID),
		Name:              p.Name,
		CreatedAt:         apptime.NewTime(apptime.MustParse(p.CreatedAt)),
		UpdatedAt:         apptime.NewTime(apptime.MustParse(p.UpdatedAt)),
	}
	if p.Description != nil {
		m.Description = *p.Description
	}
	if p.BasePrice != nil {
		m.BasePrice = *p.BasePrice
	}
	// Note: BasePriceCents is in DB but not in model, skip it
	if p.Currency != nil {
		m.Currency = *p.Currency
	}
	// Copy pointer fields directly
	m.FilterNumeric1 = p.FilterNumeric1
	m.FilterNumeric2 = p.FilterNumeric2
	m.FilterNumeric3 = p.FilterNumeric3
	m.FilterNumeric4 = p.FilterNumeric4
	m.FilterNumeric5 = p.FilterNumeric5
	m.FilterText1 = p.FilterText1
	m.FilterText2 = p.FilterText2
	m.FilterText3 = p.FilterText3
	m.FilterText4 = p.FilterText4
	m.FilterText5 = p.FilterText5
	// Convert int64 to bool for boolean fields
	m.FilterBoolean1 = int64PtrToBoolPtr(p.FilterBoolean1)
	m.FilterBoolean2 = int64PtrToBoolPtr(p.FilterBoolean2)
	m.FilterBoolean3 = int64PtrToBoolPtr(p.FilterBoolean3)
	m.FilterBoolean4 = int64PtrToBoolPtr(p.FilterBoolean4)
	m.FilterBoolean5 = int64PtrToBoolPtr(p.FilterBoolean5)
	m.FilterEnum1 = p.FilterEnum1
	m.FilterEnum2 = p.FilterEnum2
	m.FilterEnum3 = p.FilterEnum3
	m.FilterEnum4 = p.FilterEnum4
	m.FilterEnum5 = p.FilterEnum5
	m.FilterLocation1 = p.FilterLocation1
	m.FilterLocation2 = p.FilterLocation2
	m.FilterLocation3 = p.FilterLocation3
	m.FilterLocation4 = p.FilterLocation4
	m.FilterLocation5 = p.FilterLocation5
	if p.PricingFormula != nil {
		m.PricingFormula = *p.PricingFormula
	}
	if p.Active != nil && *p.Active == 1 {
		m.Active = true
	}
	if p.CustomFields != nil {
		json.Unmarshal(p.CustomFields, &m.CustomFields)
	}
	if p.Variables != nil {
		json.Unmarshal(p.Variables, &m.Variables)
	}
	return m
}

func dbProductTemplateToModel(t db.ExtProductsProductTemplate) models.ProductTemplate {
	m := models.ProductTemplate{
		ID:          uint(t.ID),
		Name:        t.Name,
		BillingMode: t.BillingMode,
		BillingType: t.BillingType,
		CreatedAt:   apptime.NewTime(apptime.MustParse(t.CreatedAt)),
		UpdatedAt:   apptime.NewTime(apptime.MustParse(t.UpdatedAt)),
	}
	if t.DisplayName != nil {
		m.DisplayName = *t.DisplayName
	}
	if t.Description != nil {
		m.Description = *t.Description
	}
	if t.Category != nil {
		m.Category = *t.Category
	}
	if t.Icon != nil {
		m.Icon = *t.Icon
	}
	if t.Status != nil {
		m.Status = *t.Status
	}
	// BillingRecurringInterval - both are *string
	m.BillingRecurringInterval = t.BillingRecurringInterval
	// BillingRecurringIntervalCount - convert *int64 to *int
	if t.BillingRecurringIntervalCount != nil {
		count := int(*t.BillingRecurringIntervalCount)
		m.BillingRecurringIntervalCount = &count
	}
	if t.FilterFieldsSchema != nil {
		json.Unmarshal(t.FilterFieldsSchema, &m.FilterFieldsSchema)
	}
	if t.CustomFieldsSchema != nil {
		json.Unmarshal(t.CustomFieldsSchema, &m.CustomFieldsSchema)
	}
	if t.PricingTemplates != nil {
		json.Unmarshal(t.PricingTemplates, &m.PricingTemplates)
	}
	return m
}

// applyGroupFilterUpdate applies filter column updates to a group
func applyGroupFilterUpdate(ctx context.Context, tx *sql.Tx, id uint, fieldID string, value interface{}) {
	parts := strings.Split(fieldID, "_")
	if len(parts) != 3 || parts[0] != "filter" {
		return
	}

	fieldType := parts[1]
	index := parts[2]

	switch fieldType {
	case "numeric":
		if v, ok := value.(float64); ok {
			tx.ExecContext(ctx, "UPDATE ext_products_groups SET filter_numeric_"+index+" = ? WHERE id = ?", v, id)
		}
	case "text":
		if v, ok := value.(string); ok {
			tx.ExecContext(ctx, "UPDATE ext_products_groups SET filter_text_"+index+" = ? WHERE id = ?", v, id)
		}
	case "boolean":
		if v, ok := value.(bool); ok {
			val := 0
			if v {
				val = 1
			}
			tx.ExecContext(ctx, "UPDATE ext_products_groups SET filter_boolean_"+index+" = ? WHERE id = ?", val, id)
		}
	case "enum":
		if v, ok := value.(string); ok {
			tx.ExecContext(ctx, "UPDATE ext_products_groups SET filter_enum_"+index+" = ? WHERE id = ?", v, id)
		}
	case "location":
		if v, ok := value.(string); ok {
			tx.ExecContext(ctx, "UPDATE ext_products_groups SET filter_location_"+index+" = ? WHERE id = ?", v, id)
		}
	}
}

// applyProductFilterUpdate applies filter column updates to a product
func applyProductFilterUpdate(ctx context.Context, tx *sql.Tx, id uint, fieldID string, value interface{}) {
	parts := strings.Split(fieldID, "_")
	if len(parts) != 3 || parts[0] != "filter" {
		return
	}

	fieldType := parts[1]
	index := parts[2]

	switch fieldType {
	case "numeric":
		if v, ok := value.(float64); ok {
			tx.ExecContext(ctx, "UPDATE ext_products_products SET filter_numeric_"+index+" = ? WHERE id = ?", v, id)
		}
	case "text":
		if v, ok := value.(string); ok {
			tx.ExecContext(ctx, "UPDATE ext_products_products SET filter_text_"+index+" = ? WHERE id = ?", v, id)
		}
	case "boolean":
		if v, ok := value.(bool); ok {
			val := 0
			if v {
				val = 1
			}
			tx.ExecContext(ctx, "UPDATE ext_products_products SET filter_boolean_"+index+" = ? WHERE id = ?", val, id)
		}
	case "enum":
		if v, ok := value.(string); ok {
			tx.ExecContext(ctx, "UPDATE ext_products_products SET filter_enum_"+index+" = ? WHERE id = ?", v, id)
		}
	case "location":
		if v, ok := value.(string); ok {
			tx.ExecContext(ctx, "UPDATE ext_products_products SET filter_location_"+index+" = ? WHERE id = ?", v, id)
		}
	}
}

// createVariablesFromFieldsTx creates variables for each field definition within a transaction
func createVariablesFromFieldsTx(ctx context.Context, qtx *db.Queries, fields []models.FieldDefinition) {
	now := apptime.NowTime()
	for _, field := range fields {
		// Convert interface{} Default value to string
		var defaultValue *string
		if field.Constraints.Default != nil {
			if s, ok := field.Constraints.Default.(string); ok {
				defaultValue = &s
			}
		}
		qtx.CreateVariable(ctx, db.CreateVariableParams{
			Name:         field.ID,
			DisplayName:  stringPtr(field.Name),
			ValueType:    stringPtr(field.Type),
			Type:         stringPtr("user"),
			Description:  stringPtr(field.Description),
			DefaultValue: defaultValue,
			Status:       stringPtr("active"),
			CreatedAt:    apptime.Format(now),
			UpdatedAt:    apptime.Format(now),
		})
	}
}
