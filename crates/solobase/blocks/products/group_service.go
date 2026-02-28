package products

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/suppers-ai/solobase/blocks/products/models"
	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/waffle-go/services/database"
)

// GroupService handles group operations
type GroupService struct {
	db              database.Service
	variableService *VariableService
}

func NewGroupService(db database.Service) *GroupService {
	return &GroupService{
		db:              db,
		variableService: NewVariableService(db),
	}
}

func (s *GroupService) ListByUser(userID string) ([]models.Group, error) {
	ctx := context.Background()
	result, err := s.db.List(ctx, "ext_products_groups", &database.ListOptions{
		Filters: []database.Filter{{Field: "user_id", Operator: database.OpEqual, Value: userID}},
		Sort:    []database.SortField{{Field: "id"}},
		Limit:   10000,
	})
	if err != nil {
		return nil, err
	}

	var groups []models.Group
	for _, r := range result.Records {
		g := recordToGroup(r)
		// Load group template
		template, err := s.getGroupTemplateByID(ctx, g.GroupTemplateID)
		if err == nil {
			g.GroupTemplate = *template
		}
		groups = append(groups, *g)
	}
	return groups, nil
}

func (s *GroupService) Create(group *models.Group) error {
	ctx := context.Background()
	now := apptime.NowString()

	// Marshal custom fields
	customFieldsJSON, _ := json.Marshal(group.CustomFields)

	_, err := s.db.ExecRaw(ctx, `
		INSERT INTO ext_products_groups (
			user_id, group_template_id, name, description,
			filter_numeric_1, filter_numeric_2, filter_numeric_3, filter_numeric_4, filter_numeric_5,
			filter_text_1, filter_text_2, filter_text_3, filter_text_4, filter_text_5,
			filter_boolean_1, filter_boolean_2, filter_boolean_3, filter_boolean_4, filter_boolean_5,
			filter_enum_1, filter_enum_2, filter_enum_3, filter_enum_4, filter_enum_5,
			filter_location_1, filter_location_2, filter_location_3, filter_location_4, filter_location_5,
			custom_fields, created_at, updated_at
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		group.UserID, group.GroupTemplateID, group.Name, stringPtr(group.Description),
		group.FilterNumeric1, group.FilterNumeric2, group.FilterNumeric3, group.FilterNumeric4, group.FilterNumeric5,
		group.FilterText1, group.FilterText2, group.FilterText3, group.FilterText4, group.FilterText5,
		boolPtrToInt64Ptr(group.FilterBoolean1), boolPtrToInt64Ptr(group.FilterBoolean2),
		boolPtrToInt64Ptr(group.FilterBoolean3), boolPtrToInt64Ptr(group.FilterBoolean4),
		boolPtrToInt64Ptr(group.FilterBoolean5),
		group.FilterEnum1, group.FilterEnum2, group.FilterEnum3, group.FilterEnum4, group.FilterEnum5,
		group.FilterLocation1, group.FilterLocation2, group.FilterLocation3, group.FilterLocation4, group.FilterLocation5,
		customFieldsJSON, now, now)
	if err != nil {
		return err
	}

	id, err := getLastInsertedID(ctx, s.db, "ext_products_groups")
	if err != nil {
		return err
	}
	group.ID = id

	// Load the group template to get field definitions
	groupTemplate, err := s.getGroupTemplateByIDWithJSON(ctx, group.GroupTemplateID)
	if err == nil {
		var filterFields []models.FieldDefinition
		if err := json.Unmarshal(groupTemplate.FilterFieldsSchemaJSON, &filterFields); err == nil && len(filterFields) > 0 {
			// Map field values to filter columns based on field IDs
			if customFields, ok := group.CustomFields["fields"].(map[string]interface{}); ok {
				for _, field := range filterFields {
					if value, exists := customFields[field.Name]; exists {
						applyGroupFilterUpdate(ctx, s.db, group.ID, field.ID, value)
					}
				}
			}

			// Create variables for each field
			createVariablesFromFields(ctx, s.db, filterFields)
		}
	}

	return nil
}

func (s *GroupService) Update(id uint, userID string, group *models.Group) error {
	ctx := context.Background()

	// Marshal custom fields
	customFieldsJSON, _ := json.Marshal(group.CustomFields)

	// Verify ownership first
	records, err := s.db.QueryRaw(ctx, "SELECT user_id FROM ext_products_groups WHERE id = ?", id)
	if err != nil || len(records) == 0 {
		return fmt.Errorf("group not found")
	}
	ownerID := stringVal(records[0].Data["user_id"])
	if ownerID != userID {
		return database.ErrNotFound // User doesn't own this group
	}

	_, err = s.db.ExecRaw(ctx, `
		UPDATE ext_products_groups SET
			name = ?, description = ?,
			filter_numeric_1 = ?, filter_numeric_2 = ?, filter_numeric_3 = ?, filter_numeric_4 = ?, filter_numeric_5 = ?,
			filter_text_1 = ?, filter_text_2 = ?, filter_text_3 = ?, filter_text_4 = ?, filter_text_5 = ?,
			filter_boolean_1 = ?, filter_boolean_2 = ?, filter_boolean_3 = ?, filter_boolean_4 = ?, filter_boolean_5 = ?,
			filter_enum_1 = ?, filter_enum_2 = ?, filter_enum_3 = ?, filter_enum_4 = ?, filter_enum_5 = ?,
			filter_location_1 = ?, filter_location_2 = ?, filter_location_3 = ?, filter_location_4 = ?, filter_location_5 = ?,
			custom_fields = ?, updated_at = ?
		WHERE id = ?`,
		group.Name, stringPtr(group.Description),
		group.FilterNumeric1, group.FilterNumeric2, group.FilterNumeric3, group.FilterNumeric4, group.FilterNumeric5,
		group.FilterText1, group.FilterText2, group.FilterText3, group.FilterText4, group.FilterText5,
		boolPtrToInt64Ptr(group.FilterBoolean1), boolPtrToInt64Ptr(group.FilterBoolean2),
		boolPtrToInt64Ptr(group.FilterBoolean3), boolPtrToInt64Ptr(group.FilterBoolean4),
		boolPtrToInt64Ptr(group.FilterBoolean5),
		group.FilterEnum1, group.FilterEnum2, group.FilterEnum3, group.FilterEnum4, group.FilterEnum5,
		group.FilterLocation1, group.FilterLocation2, group.FilterLocation3, group.FilterLocation4, group.FilterLocation5,
		customFieldsJSON, apptime.NowString(), id)
	return err
}

func (s *GroupService) Delete(id uint, userID string) error {
	ctx := context.Background()

	// Verify ownership first
	records, err := s.db.QueryRaw(ctx, "SELECT user_id FROM ext_products_groups WHERE id = ?", id)
	if err != nil || len(records) == 0 {
		return fmt.Errorf("group not found")
	}
	ownerID := stringVal(records[0].Data["user_id"])
	if ownerID != userID {
		return database.ErrNotFound
	}

	_, err = s.db.ExecRaw(ctx, "DELETE FROM ext_products_groups WHERE id = ?", id)
	return err
}

func (s *GroupService) GetByID(id uint, userID string) (*models.Group, error) {
	ctx := context.Background()
	records, err := s.db.QueryRaw(ctx, "SELECT "+groupColumns+" FROM ext_products_groups WHERE id = ?", id)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, database.ErrNotFound
	}

	g := recordToGroup(records[0])
	if g.UserID != userID {
		return nil, database.ErrNotFound
	}

	// Load group template
	template, err := s.getGroupTemplateByID(ctx, g.GroupTemplateID)
	if err == nil {
		g.GroupTemplate = *template
	}

	return g, nil
}

// ListAll returns all groups (admin function)
func (s *GroupService) ListAll() ([]models.Group, error) {
	ctx := context.Background()
	result, err := s.db.List(ctx, "ext_products_groups", &database.ListOptions{
		Sort:  []database.SortField{{Field: "id"}},
		Limit: 1000,
	})
	if err != nil {
		return nil, err
	}

	var groups []models.Group
	for _, r := range result.Records {
		g := recordToGroup(r)
		// Load group template
		template, err := s.getGroupTemplateByID(ctx, g.GroupTemplateID)
		if err == nil {
			g.GroupTemplate = *template
		}
		groups = append(groups, *g)
	}
	return groups, nil
}

// UpdateAdmin updates a group (admin function - no user check)
func (s *GroupService) UpdateAdmin(id uint, group *models.Group) error {
	ctx := context.Background()

	// Marshal custom fields
	customFieldsJSON, _ := json.Marshal(group.CustomFields)

	_, err := s.db.ExecRaw(ctx, `
		UPDATE ext_products_groups SET
			name = ?, description = ?,
			filter_numeric_1 = ?, filter_numeric_2 = ?, filter_numeric_3 = ?, filter_numeric_4 = ?, filter_numeric_5 = ?,
			filter_text_1 = ?, filter_text_2 = ?, filter_text_3 = ?, filter_text_4 = ?, filter_text_5 = ?,
			filter_boolean_1 = ?, filter_boolean_2 = ?, filter_boolean_3 = ?, filter_boolean_4 = ?, filter_boolean_5 = ?,
			filter_enum_1 = ?, filter_enum_2 = ?, filter_enum_3 = ?, filter_enum_4 = ?, filter_enum_5 = ?,
			filter_location_1 = ?, filter_location_2 = ?, filter_location_3 = ?, filter_location_4 = ?, filter_location_5 = ?,
			custom_fields = ?, updated_at = ?
		WHERE id = ?`,
		group.Name, stringPtr(group.Description),
		group.FilterNumeric1, group.FilterNumeric2, group.FilterNumeric3, group.FilterNumeric4, group.FilterNumeric5,
		group.FilterText1, group.FilterText2, group.FilterText3, group.FilterText4, group.FilterText5,
		boolPtrToInt64Ptr(group.FilterBoolean1), boolPtrToInt64Ptr(group.FilterBoolean2),
		boolPtrToInt64Ptr(group.FilterBoolean3), boolPtrToInt64Ptr(group.FilterBoolean4),
		boolPtrToInt64Ptr(group.FilterBoolean5),
		group.FilterEnum1, group.FilterEnum2, group.FilterEnum3, group.FilterEnum4, group.FilterEnum5,
		group.FilterLocation1, group.FilterLocation2, group.FilterLocation3, group.FilterLocation4, group.FilterLocation5,
		customFieldsJSON, apptime.NowString(), id)
	return err
}

// DeleteAdmin deletes a group (admin function - no user check)
func (s *GroupService) DeleteAdmin(id uint) error {
	ctx := context.Background()
	_, err := s.db.ExecRaw(ctx, "DELETE FROM ext_products_groups WHERE id = ?", id)
	return err
}

// Helper to get group template by ID
func (s *GroupService) getGroupTemplateByID(ctx context.Context, id uint) (*models.GroupTemplate, error) {
	records, err := s.db.QueryRaw(ctx, "SELECT "+groupTemplateColumns+" FROM ext_products_group_templates WHERE id = ?", id)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, database.ErrNotFound
	}
	return recordToGroupTemplate(records[0]), nil
}

// Helper to get group template by ID returning the raw JSON variant
func (s *GroupService) getGroupTemplateByIDWithJSON(ctx context.Context, id uint) (*groupTemplateWithJSON, error) {
	records, err := s.db.QueryRaw(ctx, "SELECT "+groupTemplateColumns+" FROM ext_products_group_templates WHERE id = ?", id)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, database.ErrNotFound
	}
	return recordToGroupTemplateWithJSON(records[0]), nil
}
