package products

import (
	"strconv"

	"github.com/suppers-ai/solobase/blocks/products/models"
	waffle "github.com/suppers-ai/waffle-go"
)

func (b *ProductsWaffleBlock) handleListProductTypes(_ waffle.Context, msg *waffle.Message) waffle.Result {
	productTemplates, err := b.productService.ListTemplates()
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 200, productTemplates)
}

func (b *ProductsWaffleBlock) handleCreateProductType(_ waffle.Context, msg *waffle.Message) waffle.Result {
	var productTemplate models.ProductTemplate
	if err := msg.Decode(&productTemplate); err != nil {
		return waffle.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	if err := b.productService.CreateTemplate(&productTemplate); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 201, productTemplate)
}

func (b *ProductsWaffleBlock) handleUpdateProductType(_ waffle.Context, msg *waffle.Message) waffle.Result {
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return waffle.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	var productTemplate models.ProductTemplate
	if err := msg.Decode(&productTemplate); err != nil {
		return waffle.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	productTemplate.ID = uint(id)
	if err := b.productService.UpdateTemplate(&productTemplate); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 200, productTemplate)
}

func (b *ProductsWaffleBlock) handleDeleteProductType(_ waffle.Context, msg *waffle.Message) waffle.Result {
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return waffle.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	if err := b.productService.DeleteTemplate(uint(id)); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.Respond(msg, 204, nil, "")
}
