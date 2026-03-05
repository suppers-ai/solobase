package products

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/suppers-ai/solobase/blocks/products/formulaengine"
	"github.com/suppers-ai/solobase/blocks/products/models"
	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/wafer-run/wafer-go/services/database"
)

// PricingService handles pricing calculations
type PricingService struct {
	db              database.Service
	variableService *VariableService
}

func NewPricingService(db database.Service, variableService *VariableService) *PricingService {
	return &PricingService{
		db:              db,
		variableService: variableService,
	}
}

// CalculatePrice evaluates the price for a product using the formula engine.
// Priority: product.PricingFormula > product template pricing templates > fallback (basePrice * quantity).
func (s *PricingService) CalculatePrice(productID uint, variables map[string]interface{}) (float64, error) {
	ctx := context.Background()

	// Load product core fields
	records, err := s.db.QueryRaw(ctx,
		"SELECT base_price, pricing_formula, variables, product_template_id FROM ext_products_products WHERE id = ?",
		productID,
	)
	if err != nil {
		return 0, err
	}
	if len(records) == 0 {
		return 0, database.ErrNotFound
	}
	d := records[0].Data

	price := toFloat64Val(d["base_price"])
	pricingFormula := stringVal(d["pricing_formula"])
	variablesJSON := stringVal(d["variables"])
	productTemplateID := toInt64Val(d["product_template_id"])

	// Parse product-level variables
	var productVars map[string]interface{}
	if variablesJSON != "" {
		json.Unmarshal([]byte(variablesJSON), &productVars)
	}

	// Always inject base_price into request variables
	if variables == nil {
		variables = make(map[string]interface{})
	}
	if _, ok := variables["base_price"]; !ok {
		variables["base_price"] = price
	}

	// Build resolver
	resolver := newProductVariableResolverFromService(variables, productVars, s.variableService)
	engine := formulaengine.NewEngine()

	// Path 1: Product has its own pricing formula
	if pricingFormula != "" {
		result, err := engine.Calculate(ctx, pricingFormula, resolver)
		if err != nil {
			return 0, fmt.Errorf("formula evaluation error: %w", err)
		}
		return result, nil
	}

	// Path 2: Product template has pricing templates -> evaluate as rules
	if productTemplateID > 0 {
		ptRecords, _ := s.db.QueryRaw(ctx,
			"SELECT pricing_templates FROM ext_products_product_templates WHERE id = ?",
			productTemplateID,
		)

		if len(ptRecords) > 0 {
			pricingTemplatesJSON := stringVal(ptRecords[0].Data["pricing_templates"])
			if pricingTemplatesJSON != "" {
				var templateIDs []uint
				if err := json.Unmarshal([]byte(pricingTemplatesJSON), &templateIDs); err == nil && len(templateIDs) > 0 {
					// Load pricing templates and convert to rules
					var rules []formulaengine.Rule
					for _, tid := range templateIDs {
						tidRecords, err := s.db.QueryRaw(ctx,
							"SELECT name, price_formula, condition_formula FROM ext_products_pricing_templates WHERE id = ? AND status = 'active'",
							tid,
						)
						if err != nil || len(tidRecords) == 0 {
							continue
						}
						td := tidRecords[0].Data
						priceFormula := stringVal(td["price_formula"])
						conditionFormula := stringVal(td["condition_formula"])
						condition := "true" // Default: always matches
						if conditionFormula != "" {
							condition = conditionFormula
						}
						rules = append(rules, formulaengine.Rule{
							Condition:   condition,
							Calculation: priceFormula,
						})
					}

					if len(rules) > 0 {
						result, err := engine.EvaluateRules(ctx, rules, resolver)
						if err == nil {
							return result.Value, nil
						}
						// If no rule matched, fall through to fallback
					}
				}
			}
		}
	}

	// Path 3: Fallback -- basePrice * quantity
	if quantity, ok := variables["quantity"].(float64); ok {
		return price * quantity, nil
	}
	return price, nil
}

// PriceBreakdown provides a detailed result of price calculation.
type PriceBreakdown struct {
	Price       float64                `json:"price"`
	Formula     string                 `json:"formula,omitempty"`
	Variables   map[string]interface{} `json:"variables,omitempty"`
	RuleApplied *formulaengine.Rule    `json:"ruleApplied,omitempty"`
	Method      string                 `json:"method"` // "formula", "rules", "fallback"
}

// CalculatePriceWithBreakdown returns a detailed pricing result.
func (s *PricingService) CalculatePriceWithBreakdown(productID uint, variables map[string]interface{}) (*PriceBreakdown, error) {
	ctx := context.Background()

	records, err := s.db.QueryRaw(ctx,
		"SELECT base_price, pricing_formula, variables, product_template_id FROM ext_products_products WHERE id = ?",
		productID,
	)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, database.ErrNotFound
	}
	d := records[0].Data

	price := toFloat64Val(d["base_price"])
	pricingFormula := stringVal(d["pricing_formula"])
	variablesJSON := stringVal(d["variables"])
	productTemplateID := toInt64Val(d["product_template_id"])

	var productVars map[string]interface{}
	if variablesJSON != "" {
		json.Unmarshal([]byte(variablesJSON), &productVars)
	}

	if variables == nil {
		variables = make(map[string]interface{})
	}
	if _, ok := variables["base_price"]; !ok {
		variables["base_price"] = price
	}

	resolver := newProductVariableResolverFromService(variables, productVars, s.variableService)
	engine := formulaengine.NewEngine()

	// Path 1: Product formula
	if pricingFormula != "" {
		result, err := engine.Calculate(ctx, pricingFormula, resolver)
		if err != nil {
			return nil, fmt.Errorf("formula evaluation error: %w", err)
		}
		allVars, _ := resolver.GetAllVariables(ctx)
		return &PriceBreakdown{
			Price:     result,
			Formula:   pricingFormula,
			Variables: allVars,
			Method:    "formula",
		}, nil
	}

	// Path 2: Template rules
	if productTemplateID > 0 {
		ptRecords, _ := s.db.QueryRaw(ctx,
			"SELECT pricing_templates FROM ext_products_product_templates WHERE id = ?",
			productTemplateID,
		)

		if len(ptRecords) > 0 {
			pricingTemplatesJSON := stringVal(ptRecords[0].Data["pricing_templates"])
			if pricingTemplatesJSON != "" {
				var templateIDs []uint
				if err := json.Unmarshal([]byte(pricingTemplatesJSON), &templateIDs); err == nil && len(templateIDs) > 0 {
					var rules []formulaengine.Rule
					for _, tid := range templateIDs {
						tidRecords, err := s.db.QueryRaw(ctx,
							"SELECT name, price_formula, condition_formula FROM ext_products_pricing_templates WHERE id = ? AND status = 'active'",
							tid,
						)
						if err != nil || len(tidRecords) == 0 {
							continue
						}
						td := tidRecords[0].Data
						priceFormula := stringVal(td["price_formula"])
						conditionFormula := stringVal(td["condition_formula"])
						condition := "true"
						if conditionFormula != "" {
							condition = conditionFormula
						}
						rules = append(rules, formulaengine.Rule{
							Condition:   condition,
							Calculation: priceFormula,
						})
					}

					if len(rules) > 0 {
						calcResult, err := engine.EvaluateRules(ctx, rules, resolver)
						if err == nil {
							return &PriceBreakdown{
								Price:       calcResult.Value,
								Formula:     calcResult.Formula,
								Variables:   calcResult.Variables,
								RuleApplied: calcResult.RuleApplied,
								Method:      "rules",
							}, nil
						}
					}
				}
			}
		}
	}

	// Path 3: Fallback
	finalPrice := price
	if quantity, ok := variables["quantity"].(float64); ok {
		finalPrice = price * quantity
	}
	return &PriceBreakdown{
		Price:     finalPrice,
		Variables: variables,
		Method:    "fallback",
	}, nil
}

// TestFormula validates and evaluates a formula with given variables.
func (s *PricingService) TestFormula(formula string, variables map[string]interface{}) (*formulaengine.CalculationResult, error) {
	engine := formulaengine.NewEngine()
	if err := engine.ValidateFormula(formula); err != nil {
		return nil, fmt.Errorf("invalid formula: %w", err)
	}

	resolver := newProductVariableResolverFromService(variables, nil, s.variableService)
	ctx := context.Background()
	result, err := engine.Calculate(ctx, formula, resolver)
	if err != nil {
		return nil, fmt.Errorf("calculation error: %w", err)
	}

	allVars, _ := resolver.GetAllVariables(ctx)
	return &formulaengine.CalculationResult{
		Value:     result,
		Formula:   formula,
		Variables: allVars,
	}, nil
}

// ListTemplates returns all pricing templates
func (s *PricingService) ListTemplates() ([]models.PricingTemplate, error) {
	ctx := context.Background()
	result, err := s.db.List(ctx, "ext_products_pricing_templates", &database.ListOptions{
		Sort:  []database.SortField{{Field: "id"}},
		Limit: 10000,
	})
	if err != nil {
		return nil, err
	}

	var templates []models.PricingTemplate
	for _, r := range result.Records {
		templates = append(templates, *recordToPricingTemplate(r))
	}
	return templates, nil
}

// CreateTemplate creates a new pricing template
func (s *PricingService) CreateTemplate(template *models.PricingTemplate) error {
	ctx := context.Background()
	now := apptime.NowString()

	// Marshal variables
	variablesJSON, _ := json.Marshal(template.Variables)

	_, err := s.db.ExecRaw(ctx, `
		INSERT INTO ext_products_pricing_templates (
			name, display_name, description, price_formula, condition_formula,
			variables, category, status, created_at, updated_at
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		template.Name, stringPtr(template.DisplayName), stringPtr(template.Description),
		template.PriceFormula, stringPtr(template.ConditionFormula),
		variablesJSON, stringPtr(template.Category), stringPtr(template.Status),
		now, now)
	if err != nil {
		return err
	}

	id, err := getLastInsertedID(ctx, s.db, "ext_products_pricing_templates")
	if err != nil {
		return err
	}
	template.ID = id
	return nil
}

// UpdateTemplate updates a pricing template
func (s *PricingService) UpdateTemplate(template *models.PricingTemplate) error {
	ctx := context.Background()

	// Marshal variables
	variablesJSON, _ := json.Marshal(template.Variables)

	_, err := s.db.ExecRaw(ctx, `
		UPDATE ext_products_pricing_templates SET
			name = ?, display_name = ?, description = ?, price_formula = ?, condition_formula = ?,
			variables = ?, category = ?, status = ?, updated_at = ?
		WHERE id = ?`,
		template.Name, stringPtr(template.DisplayName), stringPtr(template.Description),
		template.PriceFormula, stringPtr(template.ConditionFormula),
		variablesJSON, stringPtr(template.Category), stringPtr(template.Status),
		apptime.NowString(), template.ID)
	return err
}

// DeleteTemplate deletes a pricing template
func (s *PricingService) DeleteTemplate(id uint) error {
	ctx := context.Background()
	_, err := s.db.ExecRaw(ctx, "DELETE FROM ext_products_pricing_templates WHERE id = ?", id)
	return err
}
