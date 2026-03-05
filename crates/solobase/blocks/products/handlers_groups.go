package products

import (
	"strconv"

	"github.com/suppers-ai/solobase/blocks/products/models"
	wafer "github.com/wafer-run/wafer-go"
)

func (b *ProductsBlock) handleListMyGroups(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	groups, err := b.groupService.ListByUser(userID)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, groups)
}

func (b *ProductsBlock) handleUserCreateGroup(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	var group models.Group
	if err := msg.Decode(&group); err != nil {
		return wafer.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	group.UserID = userID
	if err := b.groupService.Create(&group); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 201, group)
}

func (b *ProductsBlock) handleGetGroup(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return wafer.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	group, err := b.groupService.GetByID(uint(id), userID)
	if err != nil {
		return wafer.Error(msg, 404, "not_found", err.Error())
	}
	return wafer.JSONRespond(msg, 200, group)
}

func (b *ProductsBlock) handleUpdateGroup(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return wafer.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	var group models.Group
	if err := msg.Decode(&group); err != nil {
		return wafer.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	if err := b.groupService.Update(uint(id), userID, &group); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, group)
}

func (b *ProductsBlock) handleDeleteGroup(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return wafer.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	if err := b.groupService.Delete(uint(id), userID); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.Respond(msg, 204, nil, "")
}

func (b *ProductsBlock) handleListGroupProducts(_ wafer.Context, msg *wafer.Message) wafer.Result {
	groupID, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return wafer.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	products, err := b.productService.ListByGroup(uint(groupID))
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, products)
}

