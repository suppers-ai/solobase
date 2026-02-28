package products

import (
	"strconv"

	"github.com/suppers-ai/solobase/blocks/products/models"
	waffle "github.com/suppers-ai/waffle-go"
)

func (b *ProductsWaffleBlock) handleListPricingTemplates(_ waffle.Context, msg *waffle.Message) waffle.Result {
	templates, err := b.pricingService.ListTemplates()
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 200, templates)
}

func (b *ProductsWaffleBlock) handleCreatePricingTemplate(_ waffle.Context, msg *waffle.Message) waffle.Result {
	var template models.PricingTemplate
	if err := msg.Decode(&template); err != nil {
		return waffle.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	if err := b.pricingService.CreateTemplate(&template); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 201, template)
}

func (b *ProductsWaffleBlock) handleUpdatePricingTemplate(_ waffle.Context, msg *waffle.Message) waffle.Result {
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return waffle.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	var template models.PricingTemplate
	if err := msg.Decode(&template); err != nil {
		return waffle.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	template.ID = uint(id)
	if err := b.pricingService.UpdateTemplate(&template); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 200, template)
}

func (b *ProductsWaffleBlock) handleDeletePricingTemplate(_ waffle.Context, msg *waffle.Message) waffle.Result {
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return waffle.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	if err := b.pricingService.DeleteTemplate(uint(id)); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.Respond(msg, 204, nil, "")
}
