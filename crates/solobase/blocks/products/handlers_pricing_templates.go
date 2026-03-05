package products

import (
	"strconv"

	"github.com/suppers-ai/solobase/blocks/products/models"
	wafer "github.com/wafer-run/wafer-go"
)

func (b *ProductsBlock) handleListPricingTemplates(_ wafer.Context, msg *wafer.Message) wafer.Result {
	templates, err := b.pricingService.ListTemplates()
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, templates)
}

func (b *ProductsBlock) handleCreatePricingTemplate(_ wafer.Context, msg *wafer.Message) wafer.Result {
	var template models.PricingTemplate
	if err := msg.Decode(&template); err != nil {
		return wafer.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	if err := b.pricingService.CreateTemplate(&template); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 201, template)
}

func (b *ProductsBlock) handleUpdatePricingTemplate(_ wafer.Context, msg *wafer.Message) wafer.Result {
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return wafer.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	var template models.PricingTemplate
	if err := msg.Decode(&template); err != nil {
		return wafer.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	template.ID = uint(id)
	if err := b.pricingService.UpdateTemplate(&template); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, template)
}

func (b *ProductsBlock) handleDeletePricingTemplate(_ wafer.Context, msg *wafer.Message) wafer.Result {
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return wafer.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	if err := b.pricingService.DeleteTemplate(uint(id)); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.Respond(msg, 204, nil, "")
}
