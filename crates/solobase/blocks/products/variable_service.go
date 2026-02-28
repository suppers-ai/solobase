package products

import (
	"context"

	"github.com/suppers-ai/solobase/blocks/products/models"
	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/waffle-go/services/database"
)

// VariableService handles variable operations
type VariableService struct {
	db database.Service
}

func NewVariableService(db database.Service) *VariableService {
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
	ctx := context.Background()
	result, err := s.db.List(ctx, "ext_products_variables", &database.ListOptions{
		Sort:  []database.SortField{{Field: "id"}},
		Limit: 10000,
	})
	if err != nil {
		return nil, err
	}

	var userVariables []models.Variable
	for _, r := range result.Records {
		userVariables = append(userVariables, *recordToVariable(r))
	}

	// Combine user variables from DB with hard-coded system variables
	return append(userVariables, GetSystemVariables()...), nil
}

func (s *VariableService) Create(variable *models.Variable) error {
	ctx := context.Background()
	now := apptime.NowString()

	// Convert DefaultValue from interface{} to string
	var defaultValueStr *string
	if variable.DefaultValue != nil {
		if str, ok := variable.DefaultValue.(string); ok {
			defaultValueStr = &str
		}
	}

	_, err := s.db.ExecRaw(ctx, `
		INSERT INTO ext_products_variables (name, display_name, value_type, type, default_value, description, status, created_at, updated_at)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		variable.Name,
		stringPtr(variable.DisplayName),
		stringPtr(variable.ValueType),
		stringPtr(variable.Type),
		defaultValueStr,
		stringPtr(variable.Description),
		stringPtr(variable.Status),
		now, now)
	if err != nil {
		return err
	}

	id, err := getLastInsertedID(ctx, s.db, "ext_products_variables")
	if err != nil {
		return err
	}
	variable.ID = id
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

	_, err := s.db.ExecRaw(ctx, `
		UPDATE ext_products_variables SET
			name = ?, display_name = ?, value_type = ?, type = ?,
			default_value = ?, description = ?, status = ?, updated_at = ?
		WHERE id = ?`,
		variable.Name,
		stringPtr(variable.DisplayName),
		stringPtr(variable.ValueType),
		stringPtr(variable.Type),
		defaultValueStr,
		stringPtr(variable.Description),
		stringPtr(variable.Status),
		apptime.NowString(),
		id)
	return err
}

func (s *VariableService) Delete(id uint) error {
	ctx := context.Background()
	_, err := s.db.ExecRaw(ctx, "DELETE FROM ext_products_variables WHERE id = ?", id)
	return err
}
