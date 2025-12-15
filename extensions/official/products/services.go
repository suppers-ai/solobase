package products

import (
	"strings"

	"github.com/suppers-ai/solobase/extensions/official/products/models"
	"gorm.io/gorm"
)

// VariableService handles variable operations
type VariableService struct {
	db *gorm.DB
}

func NewVariableService(db *gorm.DB) *VariableService {
	return &VariableService{db: db}
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
	var userVariables []models.Variable
	err := s.db.Find(&userVariables).Error
	if err != nil {
		return nil, err
	}

	// Combine user variables from DB with hard-coded system variables
	allVariables := append(userVariables, GetSystemVariables()...)
	return allVariables, nil
}

func (s *VariableService) Create(variable *models.Variable) error {
	return s.db.Create(variable).Error
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

	if err := s.db.Create(variable).Error; err != nil {
		return nil, err
	}
	return variable, nil
}

func (s *VariableService) Update(id uint, variable *models.Variable) error {
	return s.db.Model(&models.Variable{}).Where("id = ?", id).Updates(variable).Error
}

func (s *VariableService) Delete(id uint) error {
	return s.db.Delete(&models.Variable{}, id).Error
}

// DeleteBySource is no longer needed as we removed source tracking from variables
// Variables are now standalone and not tied to specific groups/products

// GroupService handles group operations
type GroupService struct {
	db              *gorm.DB
	variableService *VariableService
}

func NewGroupService(db *gorm.DB) *GroupService {
	return &GroupService{
		db:              db,
		variableService: NewVariableService(db),
	}
}

func (s *GroupService) ListByUser(userID string) ([]models.Group, error) {
	var groups []models.Group
	err := s.db.Preload("GroupTemplate").Where("user_id = ?", userID).Find(&groups).Error
	return groups, err
}

func (s *GroupService) Create(group *models.Group) error {
	tx := s.db.Begin()

	if err := tx.Create(group).Error; err != nil {
		tx.Rollback()
		return err
	}

	// Load the group template to get field definitions
	var groupTemplate models.GroupTemplate
	if err := tx.First(&groupTemplate, group.GroupTemplateID).Error; err == nil && len(groupTemplate.FilterFieldsSchema) > 0 {
		// Map field values to filter columns based on field IDs
		if customFields, ok := group.CustomFields["fields"].(map[string]interface{}); ok {
			for _, field := range groupTemplate.FilterFieldsSchema {
				if value, exists := customFields[field.Name]; exists {
					applyFilterColumnUpdates[models.Group](tx, group.ID, field.ID, value)
				}
			}
		}

		// Create variables for each field
		createVariablesFromFields(tx, groupTemplate.FilterFieldsSchema)
	}

	tx.Commit()
	return nil
}

// mapFilterColumnUpdates parses a filter field ID and value into a map of column updates
// Returns nil if the field ID is invalid or value type doesn't match
func mapFilterColumnUpdates(fieldID string, value interface{}) map[string]interface{} {
	parts := strings.Split(fieldID, "_")
	if len(parts) != 3 || parts[0] != "filter" {
		return nil
	}

	fieldType := parts[1]
	index := parts[2]
	updates := map[string]interface{}{}

	switch fieldType {
	case "numeric":
		if v, ok := value.(float64); ok {
			updates["filter_numeric_"+index] = v
		}
	case "text":
		if v, ok := value.(string); ok {
			updates["filter_text_"+index] = v
		}
	case "boolean":
		if v, ok := value.(bool); ok {
			updates["filter_boolean_"+index] = v
		}
	case "enum":
		if v, ok := value.(string); ok {
			updates["filter_enum_"+index] = v
		}
	case "location":
		if v, ok := value.(string); ok {
			updates["filter_location_"+index] = v
		}
	}

	if len(updates) == 0 {
		return nil
	}
	return updates
}

// applyFilterColumnUpdates applies filter column updates to a model
func applyFilterColumnUpdates[T any](tx *gorm.DB, id uint, fieldID string, value interface{}) {
	if updates := mapFilterColumnUpdates(fieldID, value); updates != nil {
		tx.Model(new(T)).Where("id = ?", id).Updates(updates)
	}
}

// createVariablesFromFields creates variables for each field definition
func createVariablesFromFields(tx *gorm.DB, fields []models.FieldDefinition) {
	for _, field := range fields {
		variable := &models.Variable{
			Name:         field.ID,
			DisplayName:  field.Name,
			ValueType:    field.Type,
			Type:         "user",
			Description:  field.Description,
			DefaultValue: field.Constraints.Default,
			Status:       "active",
		}
		tx.Create(variable)
	}
}

func (s *GroupService) Update(id uint, userID string, group *models.Group) error {
	return s.db.Model(&models.Group{}).Where("id = ? AND user_id = ?", id, userID).Updates(group).Error
}

func (s *GroupService) Delete(id uint, userID string) error {
	return s.db.Where("id = ? AND user_id = ?", id, userID).Delete(&models.Group{}).Error
}

func (s *GroupService) GetByID(id uint, userID string) (*models.Group, error) {
	var group models.Group
	err := s.db.Preload("GroupTemplate").Where("id = ? AND user_id = ?", id, userID).First(&group).Error
	if err != nil {
		return nil, err
	}
	return &group, nil
}

// ProductService handles product operations
type ProductService struct {
	db              *gorm.DB
	variableService *VariableService
}

func NewProductService(db *gorm.DB, variableService *VariableService) *ProductService {
	return &ProductService{
		db:              db,
		variableService: variableService,
	}
}

func (s *ProductService) ListByGroup(groupID uint) ([]models.Product, error) {
	var products []models.Product
	err := s.db.Preload("ProductTemplate").Where("group_id = ?", groupID).Find(&products).Error
	return products, err
}

func (s *ProductService) ListByUser(userID string) ([]models.Product, error) {
	var products []models.Product

	// First, get all group IDs for the user
	var groupIDs []uint
	if err := s.db.Model(&models.Group{}).Where("user_id = ?", userID).Pluck("id", &groupIDs).Error; err != nil {
		return nil, err
	}

	// Then get products for those groups
	if len(groupIDs) > 0 {
		err := s.db.Preload("Group").Preload("ProductTemplate").
			Where("group_id IN ?", groupIDs).
			Find(&products).Error
		return products, err
	}

	return products, nil
}

// GetByID retrieves a product by ID
func (s *ProductService) GetByID(id uint) (*models.Product, error) {
	var product models.Product
	if err := s.db.Preload("ProductTemplate").Preload("Group").First(&product, id).Error; err != nil {
		return nil, err
	}
	return &product, nil
}

func (s *ProductService) Create(product *models.Product) error {
	tx := s.db.Begin()

	if err := tx.Create(product).Error; err != nil {
		tx.Rollback()
		return err
	}

	// Load the product template to get field definitions
	var productTemplate models.ProductTemplate
	if err := tx.First(&productTemplate, product.ProductTemplateID).Error; err == nil && len(productTemplate.FilterFieldsSchema) > 0 {
		// Map field values to filter columns based on field IDs
		if customFields, ok := product.CustomFields["fields"].(map[string]interface{}); ok {
			for _, field := range productTemplate.FilterFieldsSchema {
				if value, exists := customFields[field.Name]; exists {
					applyFilterColumnUpdates[models.Product](tx, product.ID, field.ID, value)
				}
			}
		}

		// Create variables for each field
		createVariablesFromFields(tx, productTemplate.FilterFieldsSchema)
	}

	tx.Commit()
	return nil
}

func (s *ProductService) Update(id uint, product *models.Product) error {
	return s.db.Model(&models.Product{}).Where("id = ?", id).Updates(product).Error
}

func (s *ProductService) Delete(id uint) error {
	return s.db.Delete(&models.Product{}, id).Error
}

// PricingService handles pricing calculations
type PricingService struct {
	db              *gorm.DB
	variableService *VariableService
}

func NewPricingService(db *gorm.DB, variableService *VariableService) *PricingService {
	return &PricingService{
		db:              db,
		variableService: variableService,
	}
}

func (s *PricingService) CalculatePrice(productID uint, variables map[string]interface{}) (float64, error) {
	// For now, return a simple calculation
	// TODO: Implement formula engine integration
	var product models.Product
	if err := s.db.First(&product, productID).Error; err != nil {
		return 0, err
	}

	// Simple calculation for now
	basePrice := product.BasePrice
	if quantity, ok := variables["quantity"].(float64); ok {
		return basePrice * quantity, nil
	}

	return basePrice, nil
}
