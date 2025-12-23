-- Variable queries

-- name: CreateVariable :one
INSERT INTO ext_products_variables (
    name, display_name, value_type, type, default_value, description, status, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetVariableByID :one
SELECT * FROM ext_products_variables WHERE id = ? LIMIT 1;

-- name: GetVariableByName :one
SELECT * FROM ext_products_variables WHERE name = ? LIMIT 1;

-- name: ListVariables :many
SELECT * FROM ext_products_variables ORDER BY name;

-- name: ListActiveVariables :many
SELECT * FROM ext_products_variables WHERE status = 'active' ORDER BY name;

-- name: UpdateVariable :exec
UPDATE ext_products_variables SET
    name = ?, display_name = ?, value_type = ?, type = ?,
    default_value = ?, description = ?, status = ?, updated_at = ?
WHERE id = ?;

-- name: DeleteVariable :exec
DELETE FROM ext_products_variables WHERE id = ?;

-- Group Template queries

-- name: CreateGroupTemplate :one
INSERT INTO ext_products_group_templates (
    name, display_name, description, icon, filter_fields_schema, status, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetGroupTemplateByID :one
SELECT * FROM ext_products_group_templates WHERE id = ? LIMIT 1;

-- name: GetGroupTemplateByName :one
SELECT * FROM ext_products_group_templates WHERE name = ? LIMIT 1;

-- name: ListGroupTemplates :many
SELECT * FROM ext_products_group_templates ORDER BY name;

-- name: ListActiveGroupTemplates :many
SELECT * FROM ext_products_group_templates WHERE status = 'active' ORDER BY name;

-- name: UpdateGroupTemplate :exec
UPDATE ext_products_group_templates SET
    name = ?, display_name = ?, description = ?, icon = ?,
    filter_fields_schema = ?, status = ?, updated_at = ?
WHERE id = ?;

-- name: DeleteGroupTemplate :exec
DELETE FROM ext_products_group_templates WHERE id = ?;

-- Group queries

-- name: CreateGroup :one
INSERT INTO ext_products_groups (
    user_id, group_template_id, name, description,
    filter_numeric_1, filter_numeric_2, filter_numeric_3, filter_numeric_4, filter_numeric_5,
    filter_text_1, filter_text_2, filter_text_3, filter_text_4, filter_text_5,
    filter_boolean_1, filter_boolean_2, filter_boolean_3, filter_boolean_4, filter_boolean_5,
    filter_enum_1, filter_enum_2, filter_enum_3, filter_enum_4, filter_enum_5,
    filter_location_1, filter_location_2, filter_location_3, filter_location_4, filter_location_5,
    custom_fields, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetGroupByID :one
SELECT * FROM ext_products_groups WHERE id = ? LIMIT 1;

-- name: ListGroups :many
SELECT * FROM ext_products_groups ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListGroupsByUserID :many
SELECT * FROM ext_products_groups WHERE user_id = ? ORDER BY created_at DESC;

-- name: ListGroupsByTemplateID :many
SELECT * FROM ext_products_groups WHERE group_template_id = ? ORDER BY created_at DESC;

-- name: CountGroupsByUserID :one
SELECT COUNT(*) FROM ext_products_groups WHERE user_id = ?;

-- name: UpdateGroup :exec
UPDATE ext_products_groups SET
    name = ?, description = ?,
    filter_numeric_1 = ?, filter_numeric_2 = ?, filter_numeric_3 = ?, filter_numeric_4 = ?, filter_numeric_5 = ?,
    filter_text_1 = ?, filter_text_2 = ?, filter_text_3 = ?, filter_text_4 = ?, filter_text_5 = ?,
    filter_boolean_1 = ?, filter_boolean_2 = ?, filter_boolean_3 = ?, filter_boolean_4 = ?, filter_boolean_5 = ?,
    filter_enum_1 = ?, filter_enum_2 = ?, filter_enum_3 = ?, filter_enum_4 = ?, filter_enum_5 = ?,
    filter_location_1 = ?, filter_location_2 = ?, filter_location_3 = ?, filter_location_4 = ?, filter_location_5 = ?,
    custom_fields = ?, updated_at = ?
WHERE id = ?;

-- name: DeleteGroup :exec
DELETE FROM ext_products_groups WHERE id = ?;

-- Product Template queries

-- name: CreateProductTemplate :one
INSERT INTO ext_products_product_templates (
    name, display_name, description, category, icon,
    filter_fields_schema, custom_fields_schema, pricing_templates,
    billing_mode, billing_type, billing_recurring_interval, billing_recurring_interval_count,
    status, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetProductTemplateByID :one
SELECT * FROM ext_products_product_templates WHERE id = ? LIMIT 1;

-- name: GetProductTemplateByName :one
SELECT * FROM ext_products_product_templates WHERE name = ? LIMIT 1;

-- name: ListProductTemplates :many
SELECT * FROM ext_products_product_templates ORDER BY name;

-- name: ListActiveProductTemplates :many
SELECT * FROM ext_products_product_templates WHERE status = 'active' ORDER BY name;

-- name: ListProductTemplatesByCategory :many
SELECT * FROM ext_products_product_templates WHERE category = ? ORDER BY name;

-- name: UpdateProductTemplate :exec
UPDATE ext_products_product_templates SET
    name = ?, display_name = ?, description = ?, category = ?, icon = ?,
    filter_fields_schema = ?, custom_fields_schema = ?, pricing_templates = ?,
    billing_mode = ?, billing_type = ?, billing_recurring_interval = ?, billing_recurring_interval_count = ?,
    status = ?, updated_at = ?
WHERE id = ?;

-- name: DeleteProductTemplate :exec
DELETE FROM ext_products_product_templates WHERE id = ?;

-- Product queries

-- name: CreateProduct :one
INSERT INTO ext_products_products (
    group_id, product_template_id, name, description, base_price, base_price_cents, currency,
    filter_numeric_1, filter_numeric_2, filter_numeric_3, filter_numeric_4, filter_numeric_5,
    filter_text_1, filter_text_2, filter_text_3, filter_text_4, filter_text_5,
    filter_boolean_1, filter_boolean_2, filter_boolean_3, filter_boolean_4, filter_boolean_5,
    filter_enum_1, filter_enum_2, filter_enum_3, filter_enum_4, filter_enum_5,
    filter_location_1, filter_location_2, filter_location_3, filter_location_4, filter_location_5,
    custom_fields, variables, pricing_formula, active, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetProductByID :one
SELECT * FROM ext_products_products WHERE id = ? LIMIT 1;

-- name: ListProducts :many
SELECT * FROM ext_products_products ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListActiveProducts :many
SELECT * FROM ext_products_products WHERE active = 1 ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListProductsByGroupID :many
SELECT * FROM ext_products_products WHERE group_id = ? ORDER BY created_at DESC;

-- name: ListProductsByTemplateID :many
SELECT * FROM ext_products_products WHERE product_template_id = ? ORDER BY created_at DESC;

-- name: ListProductsByUserID :many
SELECT p.* FROM ext_products_products p
JOIN ext_products_groups g ON p.group_id = g.id
WHERE g.user_id = ?
ORDER BY p.created_at DESC;

-- name: CountProducts :one
SELECT COUNT(*) FROM ext_products_products;

-- name: CountActiveProducts :one
SELECT COUNT(*) FROM ext_products_products WHERE active = 1;

-- name: CountProductsByGroupID :one
SELECT COUNT(*) FROM ext_products_products WHERE group_id = ?;

-- name: UpdateProduct :exec
UPDATE ext_products_products SET
    name = ?, description = ?, base_price = ?, base_price_cents = ?, currency = ?,
    filter_numeric_1 = ?, filter_numeric_2 = ?, filter_numeric_3 = ?, filter_numeric_4 = ?, filter_numeric_5 = ?,
    filter_text_1 = ?, filter_text_2 = ?, filter_text_3 = ?, filter_text_4 = ?, filter_text_5 = ?,
    filter_boolean_1 = ?, filter_boolean_2 = ?, filter_boolean_3 = ?, filter_boolean_4 = ?, filter_boolean_5 = ?,
    filter_enum_1 = ?, filter_enum_2 = ?, filter_enum_3 = ?, filter_enum_4 = ?, filter_enum_5 = ?,
    filter_location_1 = ?, filter_location_2 = ?, filter_location_3 = ?, filter_location_4 = ?, filter_location_5 = ?,
    custom_fields = ?, variables = ?, pricing_formula = ?, active = ?, updated_at = ?
WHERE id = ?;

-- name: UpdateProductActiveStatus :exec
UPDATE ext_products_products SET active = ?, updated_at = ? WHERE id = ?;

-- name: DeleteProduct :exec
DELETE FROM ext_products_products WHERE id = ?;

-- name: DeleteProductsByGroupID :exec
DELETE FROM ext_products_products WHERE group_id = ?;

-- Pricing Template queries

-- name: CreatePricingTemplate :one
INSERT INTO ext_products_pricing_templates (
    name, display_name, description, price_formula, condition_formula,
    variables, category, status, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetPricingTemplateByID :one
SELECT * FROM ext_products_pricing_templates WHERE id = ? LIMIT 1;

-- name: GetPricingTemplateByName :one
SELECT * FROM ext_products_pricing_templates WHERE name = ? LIMIT 1;

-- name: ListPricingTemplates :many
SELECT * FROM ext_products_pricing_templates ORDER BY name;

-- name: ListActivePricingTemplates :many
SELECT * FROM ext_products_pricing_templates WHERE status = 'active' ORDER BY name;

-- name: ListPricingTemplatesByCategory :many
SELECT * FROM ext_products_pricing_templates WHERE category = ? ORDER BY name;

-- name: UpdatePricingTemplate :exec
UPDATE ext_products_pricing_templates SET
    name = ?, display_name = ?, description = ?, price_formula = ?,
    condition_formula = ?, variables = ?, category = ?, status = ?, updated_at = ?
WHERE id = ?;

-- name: DeletePricingTemplate :exec
DELETE FROM ext_products_pricing_templates WHERE id = ?;

-- Purchase queries

-- name: CreatePurchase :one
INSERT INTO ext_products_purchases (
    user_id, provider, provider_session_id, provider_payment_intent_id, provider_subscription_id,
    line_items, product_metadata, tax_items, amount_cents, tax_cents, total_cents, currency,
    status, requires_approval, success_url, cancel_url, customer_email, customer_name,
    billing_address, shipping_address, payment_method_types, expires_at, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetPurchaseByID :one
SELECT * FROM ext_products_purchases WHERE id = ? LIMIT 1;

-- name: GetPurchaseBySessionID :one
SELECT * FROM ext_products_purchases WHERE provider_session_id = ? LIMIT 1;

-- name: GetPurchaseByPaymentIntentID :one
SELECT * FROM ext_products_purchases WHERE provider_payment_intent_id = ? LIMIT 1;

-- name: GetPurchaseBySubscriptionID :one
SELECT * FROM ext_products_purchases WHERE provider_subscription_id = ? LIMIT 1;

-- name: ListPurchases :many
SELECT * FROM ext_products_purchases ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListPurchasesByUserID :many
SELECT * FROM ext_products_purchases WHERE user_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListPurchasesByStatus :many
SELECT * FROM ext_products_purchases WHERE status = ? ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListPendingApprovalPurchases :many
SELECT * FROM ext_products_purchases WHERE requires_approval = 1 AND approved_at IS NULL AND status = 'paid' ORDER BY created_at DESC;

-- name: CountPurchases :one
SELECT COUNT(*) FROM ext_products_purchases;

-- name: CountPurchasesByUserID :one
SELECT COUNT(*) FROM ext_products_purchases WHERE user_id = ?;

-- name: CountPurchasesByStatus :one
SELECT COUNT(*) FROM ext_products_purchases WHERE status = ?;

-- name: SumPurchasesByUserID :one
SELECT COALESCE(SUM(total_cents), 0) FROM ext_products_purchases WHERE user_id = ? AND status IN ('paid', 'paid_pending_approval');

-- name: UpdatePurchaseStatus :exec
UPDATE ext_products_purchases SET status = ?, updated_at = ? WHERE id = ?;

-- name: UpdatePurchaseProviderIDs :exec
UPDATE ext_products_purchases SET
    provider_session_id = ?,
    provider_payment_intent_id = ?,
    provider_subscription_id = ?,
    updated_at = ?
WHERE id = ?;

-- name: ApprovePurchase :exec
UPDATE ext_products_purchases SET approved_at = ?, approved_by = ?, updated_at = ? WHERE id = ?;

-- name: RefundPurchase :exec
UPDATE ext_products_purchases SET
    refunded_at = ?, refund_reason = ?, refund_amount = ?, status = 'refunded', updated_at = ?
WHERE id = ?;

-- name: CancelPurchase :exec
UPDATE ext_products_purchases SET cancelled_at = ?, cancel_reason = ?, status = 'cancelled', updated_at = ? WHERE id = ?;

-- name: DeletePurchase :exec
DELETE FROM ext_products_purchases WHERE id = ?;
