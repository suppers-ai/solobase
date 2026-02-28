package products

import (
	"context"
	"encoding/json"
	"strconv"

	"github.com/suppers-ai/solobase/blocks/products/models"
	waffle "github.com/suppers-ai/waffle-go"
	"github.com/suppers-ai/waffle-go/services/database"
)

func (b *ProductsWaffleBlock) handleListGroupTypes(_ waffle.Context, msg *waffle.Message) waffle.Result {
	ctx := context.Background()
	result, err := b.db.List(ctx, "ext_products_group_templates", &database.ListOptions{
		Sort:  []database.SortField{{Field: "id"}},
		Limit: 10000,
	})
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}

	var groupTemplates []models.GroupTemplate
	for _, r := range result.Records {
		groupTemplates = append(groupTemplates, *recordToGroupTemplate(r))
	}
	return waffle.JSONRespond(msg, 200, groupTemplates)
}

func (b *ProductsWaffleBlock) handleCreateGroupType(_ waffle.Context, msg *waffle.Message) waffle.Result {
	ctx := context.Background()
	var groupTemplate models.GroupTemplate
	if err := msg.Decode(&groupTemplate); err != nil {
		return waffle.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	fieldsSchema, _ := json.Marshal(groupTemplate.FilterFieldsSchema)
	_, err := b.db.ExecRaw(ctx, "INSERT INTO ext_products_group_templates (name, description, fields_schema) VALUES (?, ?, ?)",
		groupTemplate.Name, groupTemplate.Description, fieldsSchema)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	id, _ := getLastInsertedID(ctx, b.db, "ext_products_group_templates")
	groupTemplate.ID = id
	return waffle.JSONRespond(msg, 201, groupTemplate)
}

func (b *ProductsWaffleBlock) handleUpdateGroupType(_ waffle.Context, msg *waffle.Message) waffle.Result {
	ctx := context.Background()
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return waffle.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	var groupTemplate models.GroupTemplate
	if err := msg.Decode(&groupTemplate); err != nil {
		return waffle.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	fieldsSchema, _ := json.Marshal(groupTemplate.FilterFieldsSchema)
	_, err = b.db.ExecRaw(ctx, "UPDATE ext_products_group_templates SET name = ?, description = ?, fields_schema = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
		groupTemplate.Name, groupTemplate.Description, fieldsSchema, id)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}

	groupTemplate.ID = uint(id)
	return waffle.JSONRespond(msg, 200, groupTemplate)
}

func (b *ProductsWaffleBlock) handleDeleteGroupType(_ waffle.Context, msg *waffle.Message) waffle.Result {
	ctx := context.Background()
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return waffle.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	if _, err := b.db.ExecRaw(ctx, "DELETE FROM ext_products_group_templates WHERE id = ?", id); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.Respond(msg, 204, nil, "")
}
