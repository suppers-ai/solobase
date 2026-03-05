package products

import (
	"strconv"

	"github.com/suppers-ai/solobase/blocks/products/models"
	wafer "github.com/wafer-run/wafer-go"
)

func (b *ProductsBlock) handleListProductTypes(_ wafer.Context, msg *wafer.Message) wafer.Result {
	productTemplates, err := b.productService.ListTemplates()
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, productTemplates)
}

func (b *ProductsBlock) handleCreateProductType(_ wafer.Context, msg *wafer.Message) wafer.Result {
	var productTemplate models.ProductTemplate
	if err := msg.Decode(&productTemplate); err != nil {
		return wafer.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	if err := b.productService.CreateTemplate(&productTemplate); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 201, productTemplate)
}

func (b *ProductsBlock) handleUpdateProductType(_ wafer.Context, msg *wafer.Message) wafer.Result {
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return wafer.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	var productTemplate models.ProductTemplate
	if err := msg.Decode(&productTemplate); err != nil {
		return wafer.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	productTemplate.ID = uint(id)
	if err := b.productService.UpdateTemplate(&productTemplate); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, productTemplate)
}

func (b *ProductsBlock) handleDeleteProductType(_ wafer.Context, msg *wafer.Message) wafer.Result {
	id, err := strconv.ParseUint(msg.Var("id"), 10, 32)
	if err != nil {
		return wafer.Error(msg, 400, "invalid_id", "Invalid ID")
	}

	if err := b.productService.DeleteTemplate(uint(id)); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.Respond(msg, 204, nil, "")
}
