use std::collections::HashMap;
use wafer_run::types::Action;
use super::mock_context::*;
use crate::blocks::products::handlers;

// ============================================================
// Admin Product CRUD
// ============================================================

#[tokio::test]
async fn admin_create_product() {
    let ctx = MockContext::new();
    let mut msg = admin_create_msg("/admin/b/products/products", serde_json::json!({
        "name": "Cloud Hosting",
        "description": "Managed hosting",
        "base_price": 29.99,
        "currency": "USD"
    }));

    let result = handlers::handle_admin(&ctx, &mut msg).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert!(body["id"].as_str().is_some());
    assert_eq!(body["data"]["name"], "Cloud Hosting");
    assert_eq!(body["data"]["status"], "draft");
    assert_eq!(body["data"]["created_by"], "admin_1");
}

#[tokio::test]
async fn admin_list_products() {
    let ctx = MockContext::new();

    // Create two products
    let mut msg1 = admin_create_msg("/admin/b/products/products", serde_json::json!({
        "name": "Product A", "base_price": 10
    }));
    handlers::handle_admin(&ctx, &mut msg1).await;
    let mut msg2 = admin_create_msg("/admin/b/products/products", serde_json::json!({
        "name": "Product B", "base_price": 20
    }));
    handlers::handle_admin(&ctx, &mut msg2).await;

    let mut list_msg = admin_get_msg("/admin/b/products/products");
    let result = handlers::handle_admin(&ctx, &mut list_msg).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert!(body["records"].as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn admin_get_product() {
    let ctx = MockContext::new();

    let mut create_msg_data = admin_create_msg("/admin/b/products/products", serde_json::json!({
        "name": "Widget", "base_price": 5.0
    }));
    let create_result = handlers::handle_admin(&ctx, &mut create_msg_data).await;
    let id = response_json(&create_result)["id"].as_str().unwrap().to_string();

    let mut get_msg_data = admin_get_msg(&format!("/admin/b/products/products/{}", id));
    let result = handlers::handle_admin(&ctx, &mut get_msg_data).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert_eq!(body["data"]["name"], "Widget");
}

#[tokio::test]
async fn admin_update_product() {
    let ctx = MockContext::new();

    let mut create = admin_create_msg("/admin/b/products/products", serde_json::json!({
        "name": "Old Name", "base_price": 10
    }));
    let create_result = handlers::handle_admin(&ctx, &mut create).await;
    let id = response_json(&create_result)["id"].as_str().unwrap().to_string();

    let mut update = request_msg("update", &format!("/admin/b/products/products/{}", id), "admin_1", serde_json::json!({
        "name": "New Name", "base_price": 20
    }));
    update.set_meta("auth.user_roles", "admin");
    let result = handlers::handle_admin(&ctx, &mut update).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert_eq!(body["data"]["name"], "New Name");
}

#[tokio::test]
async fn admin_delete_product() {
    let ctx = MockContext::new();

    let mut create = admin_create_msg("/admin/b/products/products", serde_json::json!({
        "name": "To Delete"
    }));
    let create_result = handlers::handle_admin(&ctx, &mut create).await;
    let id = response_json(&create_result)["id"].as_str().unwrap().to_string();

    let mut del = delete_msg(&format!("/admin/b/products/products/{}", id), "admin_1");
    del.set_meta("auth.user_roles", "admin");
    let result = handlers::handle_admin(&ctx, &mut del).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert_eq!(body["deleted"], true);

    // Verify it's gone
    let mut get = admin_get_msg(&format!("/admin/b/products/products/{}", id));
    let result = handlers::handle_admin(&ctx, &mut get).await;
    assert!(is_error(&result, "not_found"));
}

// ============================================================
// Admin Group CRUD
// ============================================================

#[tokio::test]
async fn admin_create_and_list_groups() {
    let ctx = MockContext::new();

    let mut create = admin_create_msg("/admin/b/products/groups", serde_json::json!({
        "name": "Electronics"
    }));
    let result = handlers::handle_admin(&ctx, &mut create).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert_eq!(body["data"]["name"], "Electronics");
    assert_eq!(body["data"]["user_id"], "admin_1");

    let mut list = admin_get_msg("/admin/b/products/groups");
    let list_result = handlers::handle_admin(&ctx, &mut list).await;
    let list_body = response_json(&list_result);
    assert_eq!(list_body["records"].as_array().unwrap().len(), 1);
}

// ============================================================
// Admin Types CRUD
// ============================================================

#[tokio::test]
async fn admin_create_and_list_types() {
    let ctx = MockContext::new();

    let mut create = admin_create_msg("/admin/b/products/types", serde_json::json!({
        "name": "subscription", "display_name": "Subscription"
    }));
    handlers::handle_admin(&ctx, &mut create).await;

    let mut list = admin_get_msg("/admin/b/products/types");
    let result = handlers::handle_admin(&ctx, &mut list).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert_eq!(body["records"].as_array().unwrap().len(), 1);
}

// ============================================================
// Admin Pricing Templates
// ============================================================

#[tokio::test]
async fn admin_pricing_template_crud() {
    let ctx = MockContext::new();

    // Create
    let mut create = admin_create_msg("/admin/b/products/pricing", serde_json::json!({
        "name": "volume-discount",
        "price_formula": "base * quantity * 0.9",
        "conditions": [{"field": "quantity", "operator": ">", "value": 10, "formula": "base * quantity * 0.8"}]
    }));
    let create_result = handlers::handle_admin(&ctx, &mut create).await;
    assert_eq!(create_result.action, Action::Respond);
    let id = response_json(&create_result)["id"].as_str().unwrap().to_string();

    // Update
    let mut update = request_msg("update", &format!("/admin/b/products/pricing/{}", id), "admin_1", serde_json::json!({
        "price_formula": "base * quantity * 0.85"
    }));
    update.set_meta("auth.user_roles", "admin");
    let update_result = handlers::handle_admin(&ctx, &mut update).await;
    assert_eq!(update_result.action, Action::Respond);
    assert_eq!(response_json(&update_result)["data"]["price_formula"], "base * quantity * 0.85");

    // Delete
    let mut del = delete_msg(&format!("/admin/b/products/pricing/{}", id), "admin_1");
    del.set_meta("auth.user_roles", "admin");
    let del_result = handlers::handle_admin(&ctx, &mut del).await;
    assert_eq!(response_json(&del_result)["deleted"], true);
}

// ============================================================
// Admin Stats
// ============================================================

#[tokio::test]
async fn admin_stats() {
    let ctx = MockContext::new();

    // Seed some products
    let mut data = HashMap::new();
    data.insert("name".to_string(), serde_json::json!("Active Product"));
    data.insert("status".to_string(), serde_json::json!("active"));
    ctx.seed("block_products_products", "p1", data);

    let mut data2 = HashMap::new();
    data2.insert("name".to_string(), serde_json::json!("Draft Product"));
    data2.insert("status".to_string(), serde_json::json!("draft"));
    ctx.seed("block_products_products", "p2", data2);

    // Seed a completed purchase
    let mut purchase_data = HashMap::new();
    purchase_data.insert("status".to_string(), serde_json::json!("completed"));
    purchase_data.insert("total_cents".to_string(), serde_json::json!(2999));
    ctx.seed("block_products_purchases", "pur1", purchase_data);

    let mut msg = admin_get_msg("/admin/b/products/stats");
    let result = handlers::handle_admin(&ctx, &mut msg).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert_eq!(body["total_products"].as_i64().unwrap(), 2);
    assert_eq!(body["active_products"].as_i64().unwrap(), 1);
    assert_eq!(body["total_purchases"].as_i64().unwrap(), 1);
    assert!((body["total_revenue"].as_f64().unwrap() - 2999.0).abs() < 0.01);
}

// ============================================================
// User Product CRUD — ownership isolation
// ============================================================

#[tokio::test]
async fn user_create_product_in_own_group() {
    let ctx = MockContext::new();

    // Create a group for user_1
    let mut create_group = create_msg("/b/products/groups", "user_1", serde_json::json!({
        "name": "My Store"
    }));
    let group_result = handlers::handle_user(&ctx, &mut create_group).await;
    let group_id = response_json(&group_result)["id"].as_str().unwrap().to_string();

    // Create a product in that group
    let mut create_prod = create_msg("/b/products/products", "user_1", serde_json::json!({
        "name": "Widget",
        "base_price": 19.99,
        "group_id": group_id
    }));
    let result = handlers::handle_user(&ctx, &mut create_prod).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert_eq!(body["data"]["name"], "Widget");
    assert_eq!(body["data"]["created_by"], "user_1");
}

#[tokio::test]
async fn user_cannot_create_product_in_other_users_group() {
    let ctx = MockContext::new();

    // Create a group for user_1
    let mut create_group = create_msg("/b/products/groups", "user_1", serde_json::json!({
        "name": "User1 Store"
    }));
    let group_result = handlers::handle_user(&ctx, &mut create_group).await;
    let group_id = response_json(&group_result)["id"].as_str().unwrap().to_string();

    // user_2 tries to create a product in user_1's group
    let mut create_prod = create_msg("/b/products/products", "user_2", serde_json::json!({
        "name": "Sneaky Product",
        "group_id": group_id
    }));
    let result = handlers::handle_user(&ctx, &mut create_prod).await;
    assert!(is_error(&result, "invalid_argument"));
}

#[tokio::test]
async fn user_cannot_see_other_users_products() {
    let ctx = MockContext::new();

    // user_1 creates a product
    let mut create = create_msg("/b/products/products", "user_1", serde_json::json!({
        "name": "Private Product"
    }));
    let create_result = handlers::handle_user(&ctx, &mut create).await;
    let prod_id = response_json(&create_result)["id"].as_str().unwrap().to_string();

    // user_2 tries to get it
    let mut get = get_msg(&format!("/b/products/products/{}", prod_id), "user_2");
    let result = handlers::handle_user(&ctx, &mut get).await;
    assert!(is_error(&result, "not_found"));
}

#[tokio::test]
async fn user_cannot_update_other_users_products() {
    let ctx = MockContext::new();

    let mut create = create_msg("/b/products/products", "user_1", serde_json::json!({
        "name": "My Product"
    }));
    let create_result = handlers::handle_user(&ctx, &mut create).await;
    let prod_id = response_json(&create_result)["id"].as_str().unwrap().to_string();

    let mut update = update_msg(&format!("/b/products/products/{}", prod_id), "user_2", serde_json::json!({
        "name": "Hijacked!"
    }));
    let result = handlers::handle_user(&ctx, &mut update).await;
    assert!(is_error(&result, "not_found"));
}

#[tokio::test]
async fn user_cannot_delete_other_users_products() {
    let ctx = MockContext::new();

    let mut create = create_msg("/b/products/products", "user_1", serde_json::json!({
        "name": "My Product"
    }));
    let create_result = handlers::handle_user(&ctx, &mut create).await;
    let prod_id = response_json(&create_result)["id"].as_str().unwrap().to_string();

    let mut del = delete_msg(&format!("/b/products/products/{}", prod_id), "user_2");
    let result = handlers::handle_user(&ctx, &mut del).await;
    assert!(is_error(&result, "not_found"));
}

#[tokio::test]
async fn user_list_only_own_products() {
    let ctx = MockContext::new();

    // user_1 creates a product
    let mut c1 = create_msg("/b/products/products", "user_1", serde_json::json!({"name": "U1 Product"}));
    handlers::handle_user(&ctx, &mut c1).await;

    // user_2 creates a product
    let mut c2 = create_msg("/b/products/products", "user_2", serde_json::json!({"name": "U2 Product"}));
    handlers::handle_user(&ctx, &mut c2).await;

    // user_1 lists — should only see their own
    let mut list = get_msg("/b/products/products", "user_1");
    let result = handlers::handle_user(&ctx, &mut list).await;
    let body = response_json(&result);
    let records = body["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["data"]["name"], "U1 Product");
}

#[tokio::test]
async fn user_update_prevents_ownership_change() {
    let ctx = MockContext::new();

    let mut create = create_msg("/b/products/products", "user_1", serde_json::json!({"name": "Mine"}));
    let create_result = handlers::handle_user(&ctx, &mut create).await;
    let prod_id = response_json(&create_result)["id"].as_str().unwrap().to_string();

    // Try to change created_by — should be stripped
    let mut update = update_msg(&format!("/b/products/products/{}", prod_id), "user_1", serde_json::json!({
        "name": "Updated",
        "created_by": "attacker"
    }));
    let result = handlers::handle_user(&ctx, &mut update).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert_eq!(body["data"]["created_by"], "user_1");
}

// ============================================================
// User Group CRUD — ownership isolation
// ============================================================

#[tokio::test]
async fn user_list_only_own_groups() {
    let ctx = MockContext::new();

    let mut g1 = create_msg("/b/products/groups", "user_1", serde_json::json!({"name": "U1 Group"}));
    handlers::handle_user(&ctx, &mut g1).await;

    let mut g2 = create_msg("/b/products/groups", "user_2", serde_json::json!({"name": "U2 Group"}));
    handlers::handle_user(&ctx, &mut g2).await;

    let mut list = get_msg("/b/products/groups", "user_1");
    let result = handlers::handle_user(&ctx, &mut list).await;
    let body = response_json(&result);
    let records = body["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["data"]["name"], "U1 Group");
}

#[tokio::test]
async fn user_cannot_update_other_users_group() {
    let ctx = MockContext::new();

    let mut create = create_msg("/b/products/groups", "user_1", serde_json::json!({"name": "My Group"}));
    let create_result = handlers::handle_user(&ctx, &mut create).await;
    let group_id = response_json(&create_result)["id"].as_str().unwrap().to_string();

    let mut update = update_msg(&format!("/b/products/groups/{}", group_id), "user_2", serde_json::json!({
        "name": "Stolen"
    }));
    let result = handlers::handle_user(&ctx, &mut update).await;
    assert!(is_error(&result, "not_found"));
}

#[tokio::test]
async fn user_group_update_prevents_ownership_change() {
    let ctx = MockContext::new();

    let mut create = create_msg("/b/products/groups", "user_1", serde_json::json!({"name": "My Group"}));
    let create_result = handlers::handle_user(&ctx, &mut create).await;
    let group_id = response_json(&create_result)["id"].as_str().unwrap().to_string();

    let mut update = update_msg(&format!("/b/products/groups/{}", group_id), "user_1", serde_json::json!({
        "name": "Renamed",
        "user_id": "attacker"
    }));
    let result = handlers::handle_user(&ctx, &mut update).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert_eq!(body["data"]["user_id"], "user_1");
}

// ============================================================
// Public Catalog
// ============================================================

#[tokio::test]
async fn catalog_only_shows_active_products() {
    let ctx = MockContext::new();

    let mut d1 = HashMap::new();
    d1.insert("name".to_string(), serde_json::json!("Active"));
    d1.insert("status".to_string(), serde_json::json!("active"));
    ctx.seed("block_products_products", "p_active", d1);

    let mut d2 = HashMap::new();
    d2.insert("name".to_string(), serde_json::json!("Draft"));
    d2.insert("status".to_string(), serde_json::json!("draft"));
    ctx.seed("block_products_products", "p_draft", d2);

    let mut msg = get_msg("/b/products/catalog", "");
    let result = handlers::handle_user(&ctx, &mut msg).await;
    let body = response_json(&result);
    let records = body["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["data"]["name"], "Active");
}

#[tokio::test]
async fn catalog_get_hides_non_active() {
    let ctx = MockContext::new();

    let mut d = HashMap::new();
    d.insert("name".to_string(), serde_json::json!("Hidden"));
    d.insert("status".to_string(), serde_json::json!("draft"));
    ctx.seed("block_products_products", "p_hidden", d);

    let mut msg = get_msg("/b/products/catalog/p_hidden", "");
    let result = handlers::handle_user(&ctx, &mut msg).await;
    assert!(is_error(&result, "not_found"));
}

// ============================================================
// Group products endpoint
// ============================================================

#[tokio::test]
async fn user_group_products_list() {
    let ctx = MockContext::new();

    // Create group
    let mut cg = create_msg("/b/products/groups", "user_1", serde_json::json!({"name": "Store"}));
    let gr = handlers::handle_user(&ctx, &mut cg).await;
    let gid = response_json(&gr)["id"].as_str().unwrap().to_string();

    // Create product in group
    let mut cp = create_msg("/b/products/products", "user_1", serde_json::json!({
        "name": "In Group",
        "group_id": gid
    }));
    handlers::handle_user(&ctx, &mut cp).await;

    // List products in group
    let mut list = get_msg(&format!("/b/products/groups/{}/products", gid), "user_1");
    let result = handlers::handle_user(&ctx, &mut list).await;
    assert_eq!(result.action, Action::Respond);
    let body = response_json(&result);
    assert!(body["records"].as_array().unwrap().len() >= 1);
}

#[tokio::test]
async fn user_cannot_list_other_users_group_products() {
    let ctx = MockContext::new();

    let mut cg = create_msg("/b/products/groups", "user_1", serde_json::json!({"name": "Private"}));
    let gr = handlers::handle_user(&ctx, &mut cg).await;
    let gid = response_json(&gr)["id"].as_str().unwrap().to_string();

    // user_2 tries to list user_1's group products
    let mut list = get_msg(&format!("/b/products/groups/{}/products", gid), "user_2");
    let result = handlers::handle_user(&ctx, &mut list).await;
    assert!(is_error(&result, "not_found"));
}

// ============================================================
// Not-found routes
// ============================================================

#[tokio::test]
async fn unknown_admin_route() {
    let ctx = MockContext::new();
    let mut msg = admin_get_msg("/admin/b/products/nonexistent");
    let result = handlers::handle_admin(&ctx, &mut msg).await;
    assert!(is_error(&result, "not_found"));
}

#[tokio::test]
async fn unknown_user_route() {
    let ctx = MockContext::new();
    let mut msg = get_msg("/b/products/nonexistent", "user_1");
    let result = handlers::handle_user(&ctx, &mut msg).await;
    assert!(is_error(&result, "not_found"));
}
