package products

import (
	"strconv"

	"github.com/suppers-ai/solobase/blocks/products/models"
	wafer "github.com/wafer-run/wafer-go"
)

func (b *ProductsBlock) handleListVariables(_ wafer.Context, msg *wafer.Message) wafer.Result {
	variables, err := b.variableService.List()
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, variables)
}

func (b *ProductsBlock) handleCreateVariable(_ wafer.Context, msg *wafer.Message) wafer.Result {
	var variable models.Variable
	if err := msg.Decode(&variable); err != nil {
		return wafer.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	if err := b.variableService.Create(&variable); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 201, variable)
}

func (b *ProductsBlock) handleUpdateVariable(_ wafer.Context, msg *wafer.Message) wafer.Result {
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return wafer.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	var variable models.Variable
	if err := msg.Decode(&variable); err != nil {
		return wafer.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	if err := b.variableService.Update(uint(id), &variable); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, variable)
}

func (b *ProductsBlock) handleDeleteVariable(_ wafer.Context, msg *wafer.Message) wafer.Result {
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return wafer.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	if err := b.variableService.Delete(uint(id)); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.Respond(msg, 204, nil, "")
}
