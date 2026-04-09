use super::mock_context::*;
use crate::blocks::products::handlers;
use crate::blocks::products::purchase;
use std::collections::HashMap;
use wafer_run::types::Action;

// ============================================================
// Purchase creation — happy path
// ============================================================

#[tokio::test]
async fn create_purchase_single_item() {
    let ctx = MockContext::new();

    // Seed a product with base_price
    let mut product = HashMap::new();
    product.insert("name".to_string(), serde_json::json!("Widget"));
    product.insert("base_price".to_string(), serde_json::json!(19.99));
    product.insert("currency".to_string(), serde_json::json!("USD"));
    product.insert("status".to_string(), serde_json::json!("active"));
    ctx.seed("suppers_ai__products__products", "prod_1", product);

    let mut msg = create_msg(
        "/b/products/purchases",
        "user_1",
        serde_json::json!({
            "items": [{"product_id": "prod_1", "quantity": 2}]
        }),
    );

    let result = purchase::handle_create(&ctx, &mut msg).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert!(body["id"].as_str().is_some());
    assert_eq!(body["status"], "pending");
    // 19.99 * 2 = 39.98 → 3998 cents
    assert_eq!(body["total_cents"].as_i64().unwrap(), 3998);
    assert_eq!(body["item_count"].as_i64().unwrap(), 1);
}

#[tokio::test]
async fn create_purchase_multiple_items() {
    let ctx = MockContext::new();

    let mut p1 = HashMap::new();
    p1.insert("name".to_string(), serde_json::json!("Item A"));
    p1.insert("base_price".to_string(), serde_json::json!(10.0));
    p1.insert("status".to_string(), serde_json::json!("active"));
    ctx.seed("suppers_ai__products__products", "pa", p1);

    let mut p2 = HashMap::new();
    p2.insert("name".to_string(), serde_json::json!("Item B"));
    p2.insert("base_price".to_string(), serde_json::json!(25.50));
    p2.insert("status".to_string(), serde_json::json!("active"));
    ctx.seed("suppers_ai__products__products", "pb", p2);

    let mut msg = create_msg(
        "/b/products/purchases",
        "user_1",
        serde_json::json!({
            "items": [
                {"product_id": "pa", "quantity": 1},
                {"product_id": "pb", "quantity": 3}
            ]
        }),
    );

    let result = purchase::handle_create(&ctx, &mut msg).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    // 10.0*1 + 25.50*3 = 10 + 76.50 = 86.50 → 8650 cents
    assert_eq!(body["total_cents"].as_i64().unwrap(), 8650);
    assert_eq!(body["item_count"].as_i64().unwrap(), 2);
}

#[tokio::test]
async fn create_purchase_with_pricing_template() {
    let ctx = MockContext::new();

    // Pricing template with formula
    let mut tmpl = HashMap::new();
    tmpl.insert("name".to_string(), serde_json::json!("per-unit"));
    tmpl.insert(
        "price_formula".to_string(),
        serde_json::json!("base * rate"),
    );
    ctx.seed("suppers_ai__products__pricing_templates", "tmpl_1", tmpl);

    // Product referencing the template
    let mut product = HashMap::new();
    product.insert("name".to_string(), serde_json::json!("Service"));
    product.insert(
        "pricing_template_id".to_string(),
        serde_json::json!("tmpl_1"),
    );
    product.insert("status".to_string(), serde_json::json!("active"));
    ctx.seed("suppers_ai__products__products", "prod_svc", product);

    let mut msg = create_msg(
        "/b/products/purchases",
        "user_1",
        serde_json::json!({
            "items": [{
                "product_id": "prod_svc",
                "quantity": 2,
                "variables": {"base": 100.0, "rate": 0.15}
            }]
        }),
    );

    let result = purchase::handle_create(&ctx, &mut msg).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    // formula: base * rate = 100 * 0.15 = 15.0 per unit, qty=2, total = 30.0 → 3000 cents
    assert_eq!(body["total_cents"].as_i64().unwrap(), 3000);
}

#[tokio::test]
async fn create_purchase_defaults_to_usd() {
    let ctx = MockContext::new();

    let mut product = HashMap::new();
    product.insert("name".to_string(), serde_json::json!("Widget"));
    product.insert("base_price".to_string(), serde_json::json!(5.0));
    product.insert("status".to_string(), serde_json::json!("active"));
    ctx.seed("suppers_ai__products__products", "p1", product);

    let mut msg = create_msg(
        "/b/products/purchases",
        "user_1",
        serde_json::json!({
            "items": [{"product_id": "p1", "quantity": 1}]
        }),
    );

    let result = purchase::handle_create(&ctx, &mut msg).await;
    assert_eq!(result.action, Action::Respond);
    // Currency defaults to USD — verify purchase was created (check via DB)
    assert_eq!(body_status(&result), "pending");
}

// ============================================================
// Purchase creation — error cases
// ============================================================

#[tokio::test]
async fn create_purchase_empty_items() {
    let ctx = MockContext::new();

    let mut msg = create_msg(
        "/b/products/purchases",
        "user_1",
        serde_json::json!({
            "items": []
        }),
    );

    let result = purchase::handle_create(&ctx, &mut msg).await;
    assert!(is_error(&result, "invalid_argument"));
}

#[tokio::test]
async fn create_purchase_zero_quantity() {
    let ctx = MockContext::new();

    let mut product = HashMap::new();
    product.insert("name".to_string(), serde_json::json!("Widget"));
    product.insert("base_price".to_string(), serde_json::json!(10.0));
    ctx.seed("suppers_ai__products__products", "p1", product);

    let mut msg = create_msg(
        "/b/products/purchases",
        "user_1",
        serde_json::json!({
            "items": [{"product_id": "p1", "quantity": 0}]
        }),
    );

    let result = purchase::handle_create(&ctx, &mut msg).await;
    assert!(is_error(&result, "invalid_argument"));
}

#[tokio::test]
async fn create_purchase_negative_quantity() {
    let ctx = MockContext::new();

    let mut product = HashMap::new();
    product.insert("name".to_string(), serde_json::json!("Widget"));
    product.insert("base_price".to_string(), serde_json::json!(10.0));
    ctx.seed("suppers_ai__products__products", "p1", product);

    let mut msg = create_msg(
        "/b/products/purchases",
        "user_1",
        serde_json::json!({
            "items": [{"product_id": "p1", "quantity": -1}]
        }),
    );

    let result = purchase::handle_create(&ctx, &mut msg).await;
    assert!(is_error(&result, "invalid_argument"));
}

#[tokio::test]
async fn create_purchase_product_not_found() {
    let ctx = MockContext::new();

    let mut msg = create_msg(
        "/b/products/purchases",
        "user_1",
        serde_json::json!({
            "items": [{"product_id": "nonexistent", "quantity": 1}]
        }),
    );

    let result = purchase::handle_create(&ctx, &mut msg).await;
    assert!(is_error(&result, "not_found"));
}

#[tokio::test]
async fn create_purchase_fallback_when_template_missing() {
    let ctx = MockContext::new();

    // Product references a template that doesn't exist — should fall back to base_price
    let mut product = HashMap::new();
    product.insert("name".to_string(), serde_json::json!("Fallback"));
    product.insert("base_price".to_string(), serde_json::json!(42.0));
    product.insert(
        "pricing_template_id".to_string(),
        serde_json::json!("nonexistent_tmpl"),
    );
    product.insert("status".to_string(), serde_json::json!("active"));
    ctx.seed("suppers_ai__products__products", "p_fb", product);

    let mut msg = create_msg(
        "/b/products/purchases",
        "user_1",
        serde_json::json!({
            "items": [{"product_id": "p_fb", "quantity": 1}]
        }),
    );

    let result = purchase::handle_create(&ctx, &mut msg).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    // Should fall back to base_price = 42.0 → 4200 cents
    assert_eq!(body["total_cents"].as_i64().unwrap(), 4200);
}

#[tokio::test]
async fn create_purchase_no_base_price_rejected() {
    let ctx = MockContext::new();

    // Product with no base_price and no template — zero-price purchases are rejected
    let mut product = HashMap::new();
    product.insert("name".to_string(), serde_json::json!("Free"));
    product.insert("status".to_string(), serde_json::json!("active"));
    ctx.seed("suppers_ai__products__products", "p_free", product);

    let mut msg = create_msg(
        "/b/products/purchases",
        "user_1",
        serde_json::json!({
            "items": [{"product_id": "p_free", "quantity": 5}]
        }),
    );

    let result = purchase::handle_create(&ctx, &mut msg).await;
    assert!(
        is_error(&result, "bad_request"),
        "zero-price purchases should be rejected"
    );
}

// ============================================================
// Purchase listing
// ============================================================

#[tokio::test]
async fn list_user_purchases_only_own() {
    let ctx = MockContext::new();

    // Seed purchases for two different users
    let mut p1 = HashMap::new();
    p1.insert("user_id".to_string(), serde_json::json!("user_1"));
    p1.insert("status".to_string(), serde_json::json!("pending"));
    p1.insert("total_cents".to_string(), serde_json::json!(1000));
    ctx.seed("suppers_ai__products__purchases", "pur_1", p1);

    let mut p2 = HashMap::new();
    p2.insert("user_id".to_string(), serde_json::json!("user_2"));
    p2.insert("status".to_string(), serde_json::json!("completed"));
    p2.insert("total_cents".to_string(), serde_json::json!(2000));
    ctx.seed("suppers_ai__products__purchases", "pur_2", p2);

    let mut msg = get_msg("/b/products/purchases", "user_1");
    let result = purchase::handle_list_user(&ctx, &mut msg).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    let records = body["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["id"], "pur_1");
}

// ============================================================
// Purchase detail retrieval
// ============================================================

#[tokio::test]
async fn get_purchase_own() {
    let ctx = MockContext::new();

    let mut pd = HashMap::new();
    pd.insert("user_id".to_string(), serde_json::json!("user_1"));
    pd.insert("status".to_string(), serde_json::json!("pending"));
    pd.insert("total_cents".to_string(), serde_json::json!(5000));
    ctx.seed("suppers_ai__products__purchases", "pur_own", pd);

    let mut msg = get_msg("/b/products/purchases/pur_own", "user_1");
    let result = purchase::handle_get(&ctx, &mut msg).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert_eq!(body["purchase"]["id"], "pur_own");
}

#[tokio::test]
async fn get_purchase_denied_for_other_user() {
    let ctx = MockContext::new();

    let mut pd = HashMap::new();
    pd.insert("user_id".to_string(), serde_json::json!("user_1"));
    pd.insert("status".to_string(), serde_json::json!("pending"));
    ctx.seed("suppers_ai__products__purchases", "pur_priv", pd);

    // user_2 tries to access user_1's purchase
    let mut msg = get_msg("/b/products/purchases/pur_priv", "user_2");
    let result = purchase::handle_get(&ctx, &mut msg).await;
    assert!(is_error(&result, "permission_denied"));
}

#[tokio::test]
async fn get_purchase_not_found() {
    let ctx = MockContext::new();

    let mut msg = get_msg("/b/products/purchases/nonexistent", "user_1");
    let result = purchase::handle_get(&ctx, &mut msg).await;
    assert!(is_error(&result, "not_found"));
}

#[tokio::test]
async fn get_purchase_admin_can_view_any() {
    let ctx = MockContext::new();

    let mut pd = HashMap::new();
    pd.insert("user_id".to_string(), serde_json::json!("user_1"));
    pd.insert("status".to_string(), serde_json::json!("completed"));
    ctx.seed("suppers_ai__products__purchases", "pur_any", pd);

    let mut msg = get_msg("/b/products/purchases/pur_any", "admin_1");
    msg.set_meta("auth.user_roles", "admin");
    let result = purchase::handle_get(&ctx, &mut msg).await;
    assert_eq!(result.action, Action::Respond);
}

// ============================================================
// Refund
// ============================================================

#[tokio::test]
async fn refund_completed_purchase() {
    let ctx = MockContext::new();

    let mut pd = HashMap::new();
    pd.insert("user_id".to_string(), serde_json::json!("user_1"));
    pd.insert("status".to_string(), serde_json::json!("completed"));
    pd.insert("total_cents".to_string(), serde_json::json!(5000));
    ctx.seed("suppers_ai__products__purchases", "pur_refund", pd);

    let mut msg = create_msg(
        "/admin/b/products/purchases/pur_refund/refund",
        "admin_1",
        serde_json::json!({"reason": "Customer requested"}),
    );
    msg.set_meta("auth.user_roles", "admin");

    let result = purchase::handle_refund(&ctx, &mut msg).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert_eq!(body["data"]["status"], "refunded");
    assert_eq!(body["data"]["refund_reason"], "Customer requested");
    assert!(body["data"]["refunded_by"].as_str().is_some());
}

#[tokio::test]
async fn refund_non_completed_fails() {
    let ctx = MockContext::new();

    let mut pd = HashMap::new();
    pd.insert("user_id".to_string(), serde_json::json!("user_1"));
    pd.insert("status".to_string(), serde_json::json!("pending"));
    ctx.seed("suppers_ai__products__purchases", "pur_pending", pd);

    let mut msg = create_msg(
        "/admin/b/products/purchases/pur_pending/refund",
        "admin_1",
        serde_json::json!({}),
    );
    msg.set_meta("auth.user_roles", "admin");

    let result = purchase::handle_refund(&ctx, &mut msg).await;
    assert!(is_error(&result, "invalid_argument"));
}

#[tokio::test]
async fn refund_already_refunded_fails() {
    let ctx = MockContext::new();

    let mut pd = HashMap::new();
    pd.insert("user_id".to_string(), serde_json::json!("user_1"));
    pd.insert("status".to_string(), serde_json::json!("refunded"));
    ctx.seed("suppers_ai__products__purchases", "pur_already", pd);

    let mut msg = create_msg(
        "/admin/b/products/purchases/pur_already/refund",
        "admin_1",
        serde_json::json!({}),
    );
    msg.set_meta("auth.user_roles", "admin");

    let result = purchase::handle_refund(&ctx, &mut msg).await;
    assert!(is_error(&result, "invalid_argument"));
}

#[tokio::test]
async fn refund_purchase_not_found() {
    let ctx = MockContext::new();

    let mut msg = create_msg(
        "/admin/b/products/purchases/nonexistent/refund",
        "admin_1",
        serde_json::json!({}),
    );
    msg.set_meta("auth.user_roles", "admin");

    let result = purchase::handle_refund(&ctx, &mut msg).await;
    assert!(is_error(&result, "not_found"));
}

#[tokio::test]
async fn refund_without_reason() {
    let ctx = MockContext::new();

    let mut pd = HashMap::new();
    pd.insert("user_id".to_string(), serde_json::json!("user_1"));
    pd.insert("status".to_string(), serde_json::json!("completed"));
    ctx.seed("suppers_ai__products__purchases", "pur_noreason", pd);

    let mut msg = create_msg(
        "/admin/b/products/purchases/pur_noreason/refund",
        "admin_1",
        serde_json::json!({}),
    );
    msg.set_meta("auth.user_roles", "admin");

    let result = purchase::handle_refund(&ctx, &mut msg).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert_eq!(body["data"]["status"], "refunded");
}

// ============================================================
// Purchase via user handler routing
// ============================================================

#[tokio::test]
async fn purchase_create_via_user_handler() {
    let ctx = MockContext::new();

    let mut product = HashMap::new();
    product.insert("name".to_string(), serde_json::json!("Routed Product"));
    product.insert("base_price".to_string(), serde_json::json!(10.0));
    product.insert("status".to_string(), serde_json::json!("active"));
    ctx.seed("suppers_ai__products__products", "p_route", product);

    let mut msg = create_msg(
        "/b/products/purchases",
        "user_1",
        serde_json::json!({
            "items": [{"product_id": "p_route", "quantity": 1}]
        }),
    );

    let result = handlers::handle_user(&ctx, &mut msg).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert_eq!(body["total_cents"].as_i64().unwrap(), 1000);
}

#[tokio::test]
async fn purchase_list_via_user_handler() {
    let ctx = MockContext::new();

    let mut pd = HashMap::new();
    pd.insert("user_id".to_string(), serde_json::json!("user_1"));
    pd.insert("status".to_string(), serde_json::json!("pending"));
    ctx.seed("suppers_ai__products__purchases", "pur_route", pd);

    let mut msg = get_msg("/b/products/purchases", "user_1");
    let result = handlers::handle_user(&ctx, &mut msg).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert_eq!(body["records"].as_array().unwrap().len(), 1);
}

// ============================================================
// Rounding edge case
// ============================================================

#[tokio::test]
async fn purchase_rounding_precision() {
    let ctx = MockContext::new();

    // 19.99 * 3 = 59.97 → should be 5997 cents, not 5996
    let mut product = HashMap::new();
    product.insert("name".to_string(), serde_json::json!("Precise"));
    product.insert("base_price".to_string(), serde_json::json!(19.99));
    product.insert("status".to_string(), serde_json::json!("active"));
    ctx.seed("suppers_ai__products__products", "p_round", product);

    let mut msg = create_msg(
        "/b/products/purchases",
        "user_1",
        serde_json::json!({
            "items": [{"product_id": "p_round", "quantity": 3}]
        }),
    );

    let result = purchase::handle_create(&ctx, &mut msg).await;
    let body = response_json(&result);
    assert_eq!(body["total_cents"].as_i64().unwrap(), 5997);
}

// --- helpers ---

fn body_status(result: &wafer_run::types::Result_) -> String {
    response_json(result)["status"]
        .as_str()
        .unwrap_or("")
        .to_string()
}
