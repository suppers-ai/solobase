package products

import (
	"context"
	"fmt"

	"github.com/suppers-ai/solobase/blocks/products/formulaengine"
	waffle "github.com/suppers-ai/waffle-go"
)

// ProductVariableResolver implements formulaengine.VariableResolver by merging
// multiple variable sources in priority order:
// 1. Request-time variables (from API caller)
// 2. Product-level variables (from product.Variables JSON)
// 3. DB variables (from ext_products_variables table)
// 4. System variables (from GetSystemVariables())
type ProductVariableResolver struct {
	requestVars map[string]interface{}
	productVars map[string]interface{}
	dbVars      map[string]interface{}
	systemVars  map[string]interface{}
}

// NewProductVariableResolver creates a resolver that merges variables from multiple sources.
func NewProductVariableResolver(
	requestVars map[string]interface{},
	productVars map[string]interface{},
	dbVars []interface{},
	systemVars []interface{},
) *ProductVariableResolver {
	// Convert DB variables list to map
	dbMap := make(map[string]interface{})
	for _, v := range dbVars {
		if m, ok := v.(map[string]interface{}); ok {
			if name, ok := m["name"].(string); ok {
				if def, ok := m["defaultValue"]; ok {
					dbMap[name] = def
				}
			}
		}
	}

	// Convert system variables list to map
	sysMap := make(map[string]interface{})
	for _, v := range systemVars {
		if m, ok := v.(map[string]interface{}); ok {
			if name, ok := m["name"].(string); ok {
				if def, ok := m["defaultValue"]; ok {
					sysMap[name] = def
				}
			}
		}
	}

	return &ProductVariableResolver{
		requestVars: requestVars,
		productVars: productVars,
		dbVars:      dbMap,
		systemVars:  sysMap,
	}
}

// newProductVariableResolverFromService creates a resolver using the variable service for DB/system vars.
func newProductVariableResolverFromService(
	requestVars map[string]interface{},
	productVars map[string]interface{},
	variableService *VariableService,
) *ProductVariableResolver {
	dbMap := make(map[string]interface{})
	sysMap := make(map[string]interface{})

	// Load DB variables
	if variableService != nil {
		if vars, err := variableService.List(); err == nil {
			for _, v := range vars {
				if v.Type == "system" {
					sysMap[v.Name] = v.DefaultValue
				} else {
					dbMap[v.Name] = v.DefaultValue
				}
			}
		}
	}

	return &ProductVariableResolver{
		requestVars: requestVars,
		productVars: productVars,
		dbVars:      dbMap,
		systemVars:  sysMap,
	}
}

func (r *ProductVariableResolver) GetVariable(_ context.Context, name string) (interface{}, error) {
	// Priority 1: Request-time variables
	if val, ok := r.requestVars[name]; ok {
		return val, nil
	}
	// Priority 2: Product-level variables
	if val, ok := r.productVars[name]; ok {
		return val, nil
	}
	// Priority 3: DB variables
	if val, ok := r.dbVars[name]; ok {
		return val, nil
	}
	// Priority 4: System variables
	if val, ok := r.systemVars[name]; ok {
		return val, nil
	}
	return nil, fmt.Errorf("variable '%s' not found", name)
}

func (r *ProductVariableResolver) HasVariable(_ context.Context, name string) bool {
	if _, ok := r.requestVars[name]; ok {
		return true
	}
	if _, ok := r.productVars[name]; ok {
		return true
	}
	if _, ok := r.dbVars[name]; ok {
		return true
	}
	if _, ok := r.systemVars[name]; ok {
		return true
	}
	return false
}

func (r *ProductVariableResolver) GetAllVariables(_ context.Context) (map[string]interface{}, error) {
	result := make(map[string]interface{})
	// Merge in reverse priority order so higher priority overwrites
	for k, v := range r.systemVars {
		result[k] = v
	}
	for k, v := range r.dbVars {
		result[k] = v
	}
	for k, v := range r.productVars {
		result[k] = v
	}
	for k, v := range r.requestVars {
		result[k] = v
	}
	return result, nil
}

// Ensure interface compliance
var _ formulaengine.VariableResolver = (*ProductVariableResolver)(nil)

// handleTestFormula handles the admin formula test sandbox endpoint.
func (b *ProductsWaffleBlock) handleTestFormula(_ waffle.Context, msg *waffle.Message) waffle.Result {
	var req struct {
		Formula   string                 `json:"formula"`
		Variables map[string]interface{} `json:"variables"`
	}
	if err := msg.Decode(&req); err != nil {
		return waffle.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	if req.Formula == "" {
		return waffle.Error(msg, 400, "validation_error", "Formula is required")
	}

	engine := formulaengine.NewEngine()

	// Validate formula syntax first
	if err := engine.ValidateFormula(req.Formula); err != nil {
		return waffle.JSONRespond(msg, 200, map[string]interface{}{
			"success": false,
			"error":   fmt.Sprintf("Invalid formula: %v", err),
		})
	}

	// Build resolver from request variables + DB variables
	resolver := newProductVariableResolverFromService(req.Variables, nil, b.variableService)

	ctx := context.Background()
	result, err := engine.Calculate(ctx, req.Formula, resolver)
	if err != nil {
		return waffle.JSONRespond(msg, 200, map[string]interface{}{
			"success": false,
			"error":   fmt.Sprintf("Calculation error: %v", err),
		})
	}

	allVars, _ := resolver.GetAllVariables(ctx)
	return waffle.JSONRespond(msg, 200, map[string]interface{}{
		"success":   true,
		"result":    result,
		"formula":   req.Formula,
		"variables": allVars,
	})
}
