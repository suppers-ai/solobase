package products

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"

	"github.com/suppers-ai/solobase/blocks/products/models"
	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/waffle-go/services/database"
)

func stringPtr(s string) *string {
	if s == "" {
		return nil
	}
	return &s
}

func float64Ptr(f float64) *float64 {
	if f == 0 {
		return nil
	}
	return &f
}

func boolToInt64Ptr(b bool) *int64 {
	if !b {
		return nil
	}
	v := int64(1)
	return &v
}

func boolPtrToInt64Ptr(b *bool) *int64 {
	if b == nil || !*b {
		return nil
	}
	v := int64(1)
	return &v
}

// stringVal extracts a string from any value.
func stringVal(v any) string {
	if v == nil {
		return ""
	}
	switch val := v.(type) {
	case string:
		return val
	case []byte:
		return string(val)
	default:
		return fmt.Sprintf("%v", val)
	}
}

// toInt64Val extracts an int64 from any value.
func toInt64Val(v any) int64 {
	if v == nil {
		return 0
	}
	switch val := v.(type) {
	case int64:
		return val
	case int:
		return int64(val)
	case float64:
		return int64(val)
	case string:
		return 0
	default:
		return 0
	}
}

// toFloat64Val extracts a float64 from any value.
func toFloat64Val(v any) float64 {
	if v == nil {
		return 0
	}
	switch val := v.(type) {
	case float64:
		return val
	case int64:
		return float64(val)
	case int:
		return float64(val)
	default:
		return 0
	}
}

// toBoolVal extracts a bool from any value.
func toBoolVal(v any) bool {
	if v == nil {
		return false
	}
	switch val := v.(type) {
	case bool:
		return val
	case int64:
		return val == 1
	case float64:
		return val == 1
	default:
		return false
	}
}

// toFloat64Ptr extracts a *float64 from a possibly-nil value.
func toFloat64Ptr(v any) *float64 {
	if v == nil {
		return nil
	}
	f := toFloat64Val(v)
	return &f
}

// toStringPtr extracts a *string from a possibly-nil value.
func toStringPtr(v any) *string {
	if v == nil {
		return nil
	}
	s := stringVal(v)
	if s == "" {
		return nil
	}
	return &s
}

// toBoolPtr extracts a *bool from a possibly-nil value (int64 -> bool).
func toBoolPtr(v any) *bool {
	if v == nil {
		return nil
	}
	b := toBoolVal(v)
	return &b
}

// toUint extracts a uint from any value.
func toUint(v any) uint {
	return uint(toInt64Val(v))
}

// idStr converts an integer-like value to a string ID.
func idStr(id uint) string {
	return fmt.Sprintf("%d", id)
}

// recordToVariable converts a database.Record to a models.Variable.
func recordToVariable(r *database.Record) *models.Variable {
	d := r.Data
	v := &models.Variable{
		ID:          toUint(d["id"]),
		Name:        stringVal(d["name"]),
		DisplayName: stringVal(d["display_name"]),
		ValueType:   stringVal(d["value_type"]),
		Type:        stringVal(d["type"]),
		Description: stringVal(d["description"]),
		Status:      stringVal(d["status"]),
	}
	if d["default_value"] != nil {
		v.DefaultValue = stringVal(d["default_value"])
	}
	if ca := stringVal(d["created_at"]); ca != "" {
		v.CreatedAt = apptime.NewTime(apptime.MustParse(ca))
	}
	if ua := stringVal(d["updated_at"]); ua != "" {
		v.UpdatedAt = apptime.NewTime(apptime.MustParse(ua))
	}
	return v
}

// recordToGroupTemplate converts a database.Record to a models.GroupTemplate.
func recordToGroupTemplate(r *database.Record) *models.GroupTemplate {
	d := r.Data
	t := &models.GroupTemplate{
		ID:          toUint(d["id"]),
		Name:        stringVal(d["name"]),
		DisplayName: stringVal(d["display_name"]),
		Description: stringVal(d["description"]),
		Icon:        stringVal(d["icon"]),
		Status:      stringVal(d["status"]),
	}
	if raw := stringVal(d["filter_fields_schema"]); raw != "" {
		json.Unmarshal([]byte(raw), &t.FilterFieldsSchema)
	}
	if ca := stringVal(d["created_at"]); ca != "" {
		t.CreatedAt = apptime.NewTime(apptime.MustParse(ca))
	}
	if ua := stringVal(d["updated_at"]); ua != "" {
		t.UpdatedAt = apptime.NewTime(apptime.MustParse(ua))
	}
	return t
}

// recordToGroupTemplateWithJSON converts a database.Record to a groupTemplateWithJSON.
func recordToGroupTemplateWithJSON(r *database.Record) *groupTemplateWithJSON {
	d := r.Data
	t := &groupTemplateWithJSON{}
	t.ID = toUint(d["id"])
	t.Name = stringVal(d["name"])
	t.DisplayName = stringVal(d["display_name"])
	t.Description = stringVal(d["description"])
	t.Icon = stringVal(d["icon"])
	t.Status = stringVal(d["status"])
	if raw := stringVal(d["filter_fields_schema"]); raw != "" {
		t.FilterFieldsSchemaJSON = []byte(raw)
		json.Unmarshal(t.FilterFieldsSchemaJSON, &t.FilterFieldsSchema)
	}
	if ca := stringVal(d["created_at"]); ca != "" {
		t.CreatedAt = apptime.NewTime(apptime.MustParse(ca))
	}
	if ua := stringVal(d["updated_at"]); ua != "" {
		t.UpdatedAt = apptime.NewTime(apptime.MustParse(ua))
	}
	return t
}

// recordToGroup converts a database.Record to a models.Group.
func recordToGroup(r *database.Record) *models.Group {
	d := r.Data
	g := &models.Group{
		ID:              toUint(d["id"]),
		UserID:          stringVal(d["user_id"]),
		GroupTemplateID: toUint(d["group_template_id"]),
		Name:            stringVal(d["name"]),
		Description:     stringVal(d["description"]),
		FilterNumeric1:  toFloat64Ptr(d["filter_numeric_1"]),
		FilterNumeric2:  toFloat64Ptr(d["filter_numeric_2"]),
		FilterNumeric3:  toFloat64Ptr(d["filter_numeric_3"]),
		FilterNumeric4:  toFloat64Ptr(d["filter_numeric_4"]),
		FilterNumeric5:  toFloat64Ptr(d["filter_numeric_5"]),
		FilterText1:     toStringPtr(d["filter_text_1"]),
		FilterText2:     toStringPtr(d["filter_text_2"]),
		FilterText3:     toStringPtr(d["filter_text_3"]),
		FilterText4:     toStringPtr(d["filter_text_4"]),
		FilterText5:     toStringPtr(d["filter_text_5"]),
		FilterBoolean1:  toBoolPtr(d["filter_boolean_1"]),
		FilterBoolean2:  toBoolPtr(d["filter_boolean_2"]),
		FilterBoolean3:  toBoolPtr(d["filter_boolean_3"]),
		FilterBoolean4:  toBoolPtr(d["filter_boolean_4"]),
		FilterBoolean5:  toBoolPtr(d["filter_boolean_5"]),
		FilterEnum1:     toStringPtr(d["filter_enum_1"]),
		FilterEnum2:     toStringPtr(d["filter_enum_2"]),
		FilterEnum3:     toStringPtr(d["filter_enum_3"]),
		FilterEnum4:     toStringPtr(d["filter_enum_4"]),
		FilterEnum5:     toStringPtr(d["filter_enum_5"]),
		FilterLocation1: toStringPtr(d["filter_location_1"]),
		FilterLocation2: toStringPtr(d["filter_location_2"]),
		FilterLocation3: toStringPtr(d["filter_location_3"]),
		FilterLocation4: toStringPtr(d["filter_location_4"]),
		FilterLocation5: toStringPtr(d["filter_location_5"]),
	}
	if raw := stringVal(d["custom_fields"]); raw != "" {
		json.Unmarshal([]byte(raw), &g.CustomFields)
	}
	if ca := stringVal(d["created_at"]); ca != "" {
		g.CreatedAt = apptime.NewTime(apptime.MustParse(ca))
	}
	if ua := stringVal(d["updated_at"]); ua != "" {
		g.UpdatedAt = apptime.NewTime(apptime.MustParse(ua))
	}
	return g
}

// recordToProductTemplate converts a database.Record to a models.ProductTemplate.
func recordToProductTemplate(r *database.Record) *models.ProductTemplate {
	d := r.Data
	t := &models.ProductTemplate{
		ID:          toUint(d["id"]),
		Name:        stringVal(d["name"]),
		DisplayName: stringVal(d["display_name"]),
		Description: stringVal(d["description"]),
		Category:    stringVal(d["category"]),
		Icon:        stringVal(d["icon"]),
		BillingMode: stringVal(d["billing_mode"]),
		BillingType: stringVal(d["billing_type"]),
		Status:      stringVal(d["status"]),
	}
	if ri := stringVal(d["billing_recurring_interval"]); ri != "" {
		t.BillingRecurringInterval = &ri
	}
	if ric := d["billing_recurring_interval_count"]; ric != nil {
		count := int(toInt64Val(ric))
		t.BillingRecurringIntervalCount = &count
	}
	if raw := stringVal(d["filter_fields_schema"]); raw != "" {
		json.Unmarshal([]byte(raw), &t.FilterFieldsSchema)
	}
	if raw := stringVal(d["custom_fields_schema"]); raw != "" {
		json.Unmarshal([]byte(raw), &t.CustomFieldsSchema)
	}
	if raw := stringVal(d["pricing_templates"]); raw != "" {
		json.Unmarshal([]byte(raw), &t.PricingTemplates)
	}
	if ca := stringVal(d["created_at"]); ca != "" {
		t.CreatedAt = apptime.NewTime(apptime.MustParse(ca))
	}
	if ua := stringVal(d["updated_at"]); ua != "" {
		t.UpdatedAt = apptime.NewTime(apptime.MustParse(ua))
	}
	return t
}

// recordToProductTemplateWithJSON converts a database.Record to a productTemplateWithJSON.
func recordToProductTemplateWithJSON(r *database.Record) *productTemplateWithJSON {
	d := r.Data
	t := &productTemplateWithJSON{}
	t.ID = toUint(d["id"])
	t.Name = stringVal(d["name"])
	t.DisplayName = stringVal(d["display_name"])
	t.Description = stringVal(d["description"])
	t.Category = stringVal(d["category"])
	t.Icon = stringVal(d["icon"])
	t.BillingMode = stringVal(d["billing_mode"])
	t.BillingType = stringVal(d["billing_type"])
	t.Status = stringVal(d["status"])
	if ri := stringVal(d["billing_recurring_interval"]); ri != "" {
		t.BillingRecurringInterval = &ri
	}
	if ric := d["billing_recurring_interval_count"]; ric != nil {
		count := int(toInt64Val(ric))
		t.BillingRecurringIntervalCount = &count
	}
	if raw := stringVal(d["filter_fields_schema"]); raw != "" {
		t.FilterFieldsSchemaJSON = []byte(raw)
		json.Unmarshal(t.FilterFieldsSchemaJSON, &t.FilterFieldsSchema)
	}
	if raw := stringVal(d["custom_fields_schema"]); raw != "" {
		json.Unmarshal([]byte(raw), &t.CustomFieldsSchema)
	}
	if raw := stringVal(d["pricing_templates"]); raw != "" {
		json.Unmarshal([]byte(raw), &t.PricingTemplates)
	}
	if ca := stringVal(d["created_at"]); ca != "" {
		t.CreatedAt = apptime.NewTime(apptime.MustParse(ca))
	}
	if ua := stringVal(d["updated_at"]); ua != "" {
		t.UpdatedAt = apptime.NewTime(apptime.MustParse(ua))
	}
	return t
}

// recordToProduct converts a database.Record to a models.Product.
func recordToProduct(r *database.Record) *models.Product {
	d := r.Data
	p := &models.Product{
		ID:                toUint(d["id"]),
		GroupID:           toUint(d["group_id"]),
		ProductTemplateID: toUint(d["product_template_id"]),
		Name:              stringVal(d["name"]),
		Description:       stringVal(d["description"]),
		BasePrice:         toFloat64Val(d["base_price"]),
		Currency:          stringVal(d["currency"]),
		PricingFormula:    stringVal(d["pricing_formula"]),
		Active:            toBoolVal(d["active"]),
		FilterNumeric1:    toFloat64Ptr(d["filter_numeric_1"]),
		FilterNumeric2:    toFloat64Ptr(d["filter_numeric_2"]),
		FilterNumeric3:    toFloat64Ptr(d["filter_numeric_3"]),
		FilterNumeric4:    toFloat64Ptr(d["filter_numeric_4"]),
		FilterNumeric5:    toFloat64Ptr(d["filter_numeric_5"]),
		FilterText1:       toStringPtr(d["filter_text_1"]),
		FilterText2:       toStringPtr(d["filter_text_2"]),
		FilterText3:       toStringPtr(d["filter_text_3"]),
		FilterText4:       toStringPtr(d["filter_text_4"]),
		FilterText5:       toStringPtr(d["filter_text_5"]),
		FilterBoolean1:    toBoolPtr(d["filter_boolean_1"]),
		FilterBoolean2:    toBoolPtr(d["filter_boolean_2"]),
		FilterBoolean3:    toBoolPtr(d["filter_boolean_3"]),
		FilterBoolean4:    toBoolPtr(d["filter_boolean_4"]),
		FilterBoolean5:    toBoolPtr(d["filter_boolean_5"]),
		FilterEnum1:       toStringPtr(d["filter_enum_1"]),
		FilterEnum2:       toStringPtr(d["filter_enum_2"]),
		FilterEnum3:       toStringPtr(d["filter_enum_3"]),
		FilterEnum4:       toStringPtr(d["filter_enum_4"]),
		FilterEnum5:       toStringPtr(d["filter_enum_5"]),
		FilterLocation1:   toStringPtr(d["filter_location_1"]),
		FilterLocation2:   toStringPtr(d["filter_location_2"]),
		FilterLocation3:   toStringPtr(d["filter_location_3"]),
		FilterLocation4:   toStringPtr(d["filter_location_4"]),
		FilterLocation5:   toStringPtr(d["filter_location_5"]),
	}
	if raw := stringVal(d["custom_fields"]); raw != "" {
		json.Unmarshal([]byte(raw), &p.CustomFields)
	}
	if raw := stringVal(d["variables"]); raw != "" {
		json.Unmarshal([]byte(raw), &p.Variables)
	}
	if ca := stringVal(d["created_at"]); ca != "" {
		p.CreatedAt = apptime.NewTime(apptime.MustParse(ca))
	}
	if ua := stringVal(d["updated_at"]); ua != "" {
		p.UpdatedAt = apptime.NewTime(apptime.MustParse(ua))
	}
	return p
}

// recordToPricingTemplate converts a database.Record to a models.PricingTemplate.
func recordToPricingTemplate(r *database.Record) *models.PricingTemplate {
	d := r.Data
	t := &models.PricingTemplate{
		ID:               toUint(d["id"]),
		Name:             stringVal(d["name"]),
		DisplayName:      stringVal(d["display_name"]),
		Description:      stringVal(d["description"]),
		PriceFormula:     stringVal(d["price_formula"]),
		ConditionFormula: stringVal(d["condition_formula"]),
		Category:         stringVal(d["category"]),
		Status:           stringVal(d["status"]),
	}
	if raw := stringVal(d["variables"]); raw != "" {
		json.Unmarshal([]byte(raw), &t.Variables)
	}
	if ca := stringVal(d["created_at"]); ca != "" {
		t.CreatedAt = apptime.NewTime(apptime.MustParse(ca))
	}
	if ua := stringVal(d["updated_at"]); ua != "" {
		t.UpdatedAt = apptime.NewTime(apptime.MustParse(ua))
	}
	return t
}

// recordToPurchase converts a database.Record to a models.Purchase.
func recordToPurchase(r *database.Record) *models.Purchase {
	d := r.Data
	p := &models.Purchase{
		ID:                       toUint(d["id"]),
		UserID:                   stringVal(d["user_id"]),
		Provider:                 stringVal(d["provider"]),
		AmountCents:              toInt64Val(d["amount_cents"]),
		TotalCents:               toInt64Val(d["total_cents"]),
		TaxCents:                 toInt64Val(d["tax_cents"]),
		Currency:                 stringVal(d["currency"]),
		Status:                   stringVal(d["status"]),
		RequiresApproval:         toBoolVal(d["requires_approval"]),
		ProviderSessionID:        stringVal(d["provider_session_id"]),
		ProviderPaymentIntentID:  stringVal(d["provider_payment_intent_id"]),
		ProviderSubscriptionID:   stringVal(d["provider_subscription_id"]),
		SuccessURL:               stringVal(d["success_url"]),
		CancelURL:                stringVal(d["cancel_url"]),
		CustomerEmail:            stringVal(d["customer_email"]),
		RefundAmount:             toInt64Val(d["refund_amount"]),
		RefundReason:             stringVal(d["refund_reason"]),
		CancelReason:             stringVal(d["cancel_reason"]),
	}

	// Unmarshal JSON fields
	if raw := stringVal(d["line_items"]); raw != "" {
		json.Unmarshal([]byte(raw), &p.LineItems)
	}
	if raw := stringVal(d["product_metadata"]); raw != "" {
		json.Unmarshal([]byte(raw), &p.ProductMetadata)
	}
	if raw := stringVal(d["tax_items"]); raw != "" {
		json.Unmarshal([]byte(raw), &p.TaxItems)
	}
	if raw := stringVal(d["payment_method_types"]); raw != "" {
		json.Unmarshal([]byte(raw), &p.PaymentMethodTypes)
	}

	// Handle nullable timestamp fields
	if at := stringVal(d["approved_at"]); at != "" {
		p.ApprovedAt = apptime.NewNullTime(apptime.MustParse(at))
	}
	if ab := stringVal(d["approved_by"]); ab != "" {
		p.ApprovedBy = &ab
	}
	if rat := stringVal(d["refunded_at"]); rat != "" {
		p.RefundedAt = apptime.NewNullTime(apptime.MustParse(rat))
	}
	if cat := stringVal(d["cancelled_at"]); cat != "" {
		p.CancelledAt = apptime.NewNullTime(apptime.MustParse(cat))
	}

	if ca := stringVal(d["created_at"]); ca != "" {
		p.CreatedAt = apptime.NewTime(apptime.MustParse(ca))
	}
	if ua := stringVal(d["updated_at"]); ua != "" {
		p.UpdatedAt = apptime.NewTime(apptime.MustParse(ua))
	}

	return p
}

// applyGroupFilterUpdate applies filter column updates to a group using ExecRaw.
func applyGroupFilterUpdate(ctx context.Context, db database.Service, id uint, fieldID string, value interface{}) {
	parts := strings.Split(fieldID, "_")
	if len(parts) != 3 || parts[0] != "filter" {
		return
	}

	fieldType := parts[1]
	index := parts[2]

	switch fieldType {
	case "numeric":
		if v, ok := value.(float64); ok {
			db.ExecRaw(ctx, "UPDATE ext_products_groups SET filter_numeric_"+index+" = ? WHERE id = ?", v, id)
		}
	case "text":
		if v, ok := value.(string); ok {
			db.ExecRaw(ctx, "UPDATE ext_products_groups SET filter_text_"+index+" = ? WHERE id = ?", v, id)
		}
	case "boolean":
		if v, ok := value.(bool); ok {
			val := 0
			if v {
				val = 1
			}
			db.ExecRaw(ctx, "UPDATE ext_products_groups SET filter_boolean_"+index+" = ? WHERE id = ?", val, id)
		}
	case "enum":
		if v, ok := value.(string); ok {
			db.ExecRaw(ctx, "UPDATE ext_products_groups SET filter_enum_"+index+" = ? WHERE id = ?", v, id)
		}
	case "location":
		if v, ok := value.(string); ok {
			db.ExecRaw(ctx, "UPDATE ext_products_groups SET filter_location_"+index+" = ? WHERE id = ?", v, id)
		}
	}
}

// applyProductFilterUpdate applies filter column updates to a product using ExecRaw.
func applyProductFilterUpdate(ctx context.Context, db database.Service, id uint, fieldID string, value interface{}) {
	parts := strings.Split(fieldID, "_")
	if len(parts) != 3 || parts[0] != "filter" {
		return
	}

	fieldType := parts[1]
	index := parts[2]

	switch fieldType {
	case "numeric":
		if v, ok := value.(float64); ok {
			db.ExecRaw(ctx, "UPDATE ext_products_products SET filter_numeric_"+index+" = ? WHERE id = ?", v, id)
		}
	case "text":
		if v, ok := value.(string); ok {
			db.ExecRaw(ctx, "UPDATE ext_products_products SET filter_text_"+index+" = ? WHERE id = ?", v, id)
		}
	case "boolean":
		if v, ok := value.(bool); ok {
			val := 0
			if v {
				val = 1
			}
			db.ExecRaw(ctx, "UPDATE ext_products_products SET filter_boolean_"+index+" = ? WHERE id = ?", val, id)
		}
	case "enum":
		if v, ok := value.(string); ok {
			db.ExecRaw(ctx, "UPDATE ext_products_products SET filter_enum_"+index+" = ? WHERE id = ?", v, id)
		}
	case "location":
		if v, ok := value.(string); ok {
			db.ExecRaw(ctx, "UPDATE ext_products_products SET filter_location_"+index+" = ? WHERE id = ?", v, id)
		}
	}
}

// createVariablesFromFields creates variables for each field definition using ExecRaw.
func createVariablesFromFields(ctx context.Context, db database.Service, fields []models.FieldDefinition) {
	now := apptime.NowString()
	for _, field := range fields {
		// Convert interface{} Default value to string
		var defaultValue *string
		if field.Constraints.Default != nil {
			if s, ok := field.Constraints.Default.(string); ok {
				defaultValue = &s
			}
		}
		db.ExecRaw(ctx, `
			INSERT INTO ext_products_variables (name, display_name, value_type, type, description, default_value, status, created_at, updated_at)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
			field.ID,
			stringPtr(field.Name),
			stringPtr(field.Type),
			stringPtr("user"),
			stringPtr(field.Description),
			defaultValue,
			stringPtr("active"),
			now, now)
	}
}

// getLastInsertedID returns the last auto-increment ID inserted into a table.
func getLastInsertedID(ctx context.Context, db database.Service, collection string) (uint, error) {
	records, err := db.QueryRaw(ctx, fmt.Sprintf("SELECT last_insert_rowid() as id FROM %s LIMIT 1", collection))
	if err != nil || len(records) == 0 {
		// Fallback: query the max ID
		records2, err2 := db.QueryRaw(ctx, fmt.Sprintf("SELECT MAX(id) as id FROM %s", collection))
		if err2 != nil || len(records2) == 0 {
			return 0, fmt.Errorf("failed to get last insert ID")
		}
		return toUint(records2[0].Data["id"]), nil
	}
	return toUint(records[0].Data["id"]), nil
}
