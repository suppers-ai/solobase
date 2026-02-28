package products

import (
	"strconv"

	"github.com/suppers-ai/solobase/blocks/products/models"
	waffle "github.com/suppers-ai/waffle-go"
)

func (b *ProductsWaffleBlock) handleListMyGroups(_ waffle.Context, msg *waffle.Message) waffle.Result {
	userID := msg.UserID()
	if userID == "" {
		return waffle.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	groups, err := b.groupService.ListByUser(userID)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 200, groups)
}

func (b *ProductsWaffleBlock) handleUserCreateGroup(_ waffle.Context, msg *waffle.Message) waffle.Result {
	userID := msg.UserID()
	if userID == "" {
		return waffle.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	var group models.Group
	if err := msg.Decode(&group); err != nil {
		return waffle.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	group.UserID = userID
	if err := b.groupService.Create(&group); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 201, group)
}

func (b *ProductsWaffleBlock) handleGetGroup(_ waffle.Context, msg *waffle.Message) waffle.Result {
	userID := msg.UserID()
	if userID == "" {
		return waffle.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return waffle.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	group, err := b.groupService.GetByID(uint(id), userID)
	if err != nil {
		return waffle.Error(msg, 404, "not_found", err.Error())
	}
	return waffle.JSONRespond(msg, 200, group)
}

func (b *ProductsWaffleBlock) handleUpdateGroup(_ waffle.Context, msg *waffle.Message) waffle.Result {
	userID := msg.UserID()
	if userID == "" {
		return waffle.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return waffle.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	var group models.Group
	if err := msg.Decode(&group); err != nil {
		return waffle.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	if err := b.groupService.Update(uint(id), userID, &group); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 200, group)
}

func (b *ProductsWaffleBlock) handleDeleteGroup(_ waffle.Context, msg *waffle.Message) waffle.Result {
	userID := msg.UserID()
	if userID == "" {
		return waffle.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return waffle.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	if err := b.groupService.Delete(uint(id), userID); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.Respond(msg, 204, nil, "")
}

func (b *ProductsWaffleBlock) handleListGroupProducts(_ waffle.Context, msg *waffle.Message) waffle.Result {
	groupID, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return waffle.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	products, err := b.productService.ListByGroup(uint(groupID))
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 200, products)
}

