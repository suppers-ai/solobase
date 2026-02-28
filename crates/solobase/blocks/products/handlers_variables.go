package products

import (
	"strconv"

	"github.com/suppers-ai/solobase/blocks/products/models"
	waffle "github.com/suppers-ai/waffle-go"
)

func (b *ProductsWaffleBlock) handleListVariables(_ waffle.Context, msg *waffle.Message) waffle.Result {
	variables, err := b.variableService.List()
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 200, variables)
}

func (b *ProductsWaffleBlock) handleCreateVariable(_ waffle.Context, msg *waffle.Message) waffle.Result {
	var variable models.Variable
	if err := msg.Decode(&variable); err != nil {
		return waffle.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	if err := b.variableService.Create(&variable); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 201, variable)
}

func (b *ProductsWaffleBlock) handleUpdateVariable(_ waffle.Context, msg *waffle.Message) waffle.Result {
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return waffle.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	var variable models.Variable
	if err := msg.Decode(&variable); err != nil {
		return waffle.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	if err := b.variableService.Update(uint(id), &variable); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 200, variable)
}

func (b *ProductsWaffleBlock) handleDeleteVariable(_ waffle.Context, msg *waffle.Message) waffle.Result {
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return waffle.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	if err := b.variableService.Delete(uint(id)); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.Respond(msg, 204, nil, "")
}
