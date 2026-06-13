use std::collections::HashMap;

use wafer_run::ErrorCode;

use super::harness::*;
use crate::blocks::products::handlers;

// ============================================================
// Admin Product CRUD
// ============================================================

#[tokio::test]
async fn admin_create_product() {
    let ctx = ctx().await;
    let (msg, input) = admin_create_msg(
        "/admin/b/products/products",
        serde_json::json!({
            "name": "Cloud Hosting",
            "description": "Managed hosting",
            "base_price": 29.99,
            "currency": "USD"
        }),
    );

    let out = handlers::handle_admin(&ctx, &msg, input).await;
    let body = output_to_json(out).await;
    assert!(body["id"].as_str().is_some());
    assert_eq!(body["data"]["name"], "Cloud Hosting");
    assert_eq!(body["data"]["status"], "draft");
    assert_eq!(body["data"]["created_by"], "admin_1");
}

#[tokio::test]
async fn admin_list_products() {
    let ctx = ctx().await;

    // Create two products
    let (msg1, input1) = admin_create_msg(
        "/admin/b/products/products",
        serde_json::json!({
            "name": "Product A", "base_price": 10
        }),
    );
    handlers::handle_admin(&ctx, &msg1, input1).await;
    let (msg2, input2) = admin_create_msg(
        "/admin/b/products/products",
        serde_json::json!({
            "name": "Product B", "base_price": 20
        }),
    );
    handlers::handle_admin(&ctx, &msg2, input2).await;

    let (list_msg, list_input) = admin_get_msg("/admin/b/products/products");
    let out = handlers::handle_admin(&ctx, &list_msg, list_input).await;
    let body = output_to_json(out).await;
    assert!(body["records"].as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn admin_get_product() {
    let ctx = ctx().await;

    let (create_msg_data, create_input) = admin_create_msg(
        "/admin/b/products/products",
        serde_json::json!({
            "name": "Widget", "base_price": 5.0
        }),
    );
    let create_out = handlers::handle_admin(&ctx, &create_msg_data, create_input).await;
    let id = output_to_json(create_out).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (get_msg_data, get_input) = admin_get_msg(&format!("/admin/b/products/products/{}", id));
    let out = handlers::handle_admin(&ctx, &get_msg_data, get_input).await;
    let body = output_to_json(out).await;
    assert_eq!(body["data"]["name"], "Widget");
}

#[tokio::test]
async fn admin_update_product() {
    let ctx = ctx().await;

    let (create, create_input) = admin_create_msg(
        "/admin/b/products/products",
        serde_json::json!({
            "name": "Old Name", "base_price": 10
        }),
    );
    let create_out = handlers::handle_admin(&ctx, &create, create_input).await;
    let id = output_to_json(create_out).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (mut update, update_input) = request_msg(
        "update",
        &format!("/admin/b/products/products/{}", id),
        "admin_1",
        serde_json::json!({
            "name": "New Name", "base_price": 20
        }),
    );
    update.set_meta("auth.user_roles", "admin");
    let out = handlers::handle_admin(&ctx, &update, update_input).await;
    let body = output_to_json(out).await;
    assert_eq!(body["data"]["name"], "New Name");
}

#[tokio::test]
async fn admin_delete_product() {
    let ctx = ctx().await;

    let (create, create_input) = admin_create_msg(
        "/admin/b/products/products",
        serde_json::json!({
            "name": "To Delete"
        }),
    );
    let create_out = handlers::handle_admin(&ctx, &create, create_input).await;
    let id = output_to_json(create_out).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (mut del, del_input) = delete_msg(&format!("/admin/b/products/products/{}", id), "admin_1");
    del.set_meta("auth.user_roles", "admin");
    let out = handlers::handle_admin(&ctx, &del, del_input).await;
    let body = output_to_json(out).await;
    assert_eq!(body["deleted"], true);

    // Verify it's gone
    let (get, get_input) = admin_get_msg(&format!("/admin/b/products/products/{}", id));
    let out = handlers::handle_admin(&ctx, &get, get_input).await;
    assert!(output_is_error(out, ErrorCode::NotFound).await);
}

// ============================================================
// Admin Group CRUD
// ============================================================

#[tokio::test]
async fn admin_create_and_list_groups() {
    let ctx = ctx().await;

    let (create, create_input) = admin_create_msg(
        "/admin/b/products/groups",
        serde_json::json!({
            "name": "Electronics"
        }),
    );
    let out = handlers::handle_admin(&ctx, &create, create_input).await;
    let body = output_to_json(out).await;
    assert_eq!(body["data"]["name"], "Electronics");
    assert_eq!(body["data"]["user_id"], "admin_1");

    let (list, list_input) = admin_get_msg("/admin/b/products/groups");
    let list_out = handlers::handle_admin(&ctx, &list, list_input).await;
    let list_body = output_to_json(list_out).await;
    assert_eq!(list_body["records"].as_array().unwrap().len(), 1);
}

// ============================================================
// Admin Types CRUD
// ============================================================

#[tokio::test]
async fn admin_create_and_list_types() {
    let ctx = ctx().await;

    let (create, create_input) = admin_create_msg(
        "/admin/b/products/types",
        serde_json::json!({
            "name": "subscription", "display_name": "Subscription"
        }),
    );
    handlers::handle_admin(&ctx, &create, create_input).await;

    let (list, list_input) = admin_get_msg("/admin/b/products/types");
    let out = handlers::handle_admin(&ctx, &list, list_input).await;
    let body = output_to_json(out).await;
    assert_eq!(body["records"].as_array().unwrap().len(), 1);
}

// ============================================================
// Admin Pricing Templates
// ============================================================

#[tokio::test]
async fn admin_pricing_template_crud() {
    let ctx = ctx().await;

    // Create
    let (create, create_input) = admin_create_msg(
        "/admin/b/products/pricing",
        serde_json::json!({
            "name": "volume-discount",
            "price_formula": "base * quantity * 0.9"
        }),
    );
    let create_out = handlers::handle_admin(&ctx, &create, create_input).await;
    let id = output_to_json(create_out).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Update
    let (mut update, update_input) = request_msg(
        "update",
        &format!("/admin/b/products/pricing/{}", id),
        "admin_1",
        serde_json::json!({
            "price_formula": "base * quantity * 0.85"
        }),
    );
    update.set_meta("auth.user_roles", "admin");
    let update_out = handlers::handle_admin(&ctx, &update, update_input).await;
    let update_body = output_to_json(update_out).await;
    assert_eq!(
        update_body["data"]["price_formula"],
        "base * quantity * 0.85"
    );

    // Delete
    let (mut del, del_input) = delete_msg(&format!("/admin/b/products/pricing/{}", id), "admin_1");
    del.set_meta("auth.user_roles", "admin");
    let del_out = handlers::handle_admin(&ctx, &del, del_input).await;
    assert_eq!(output_to_json(del_out).await["deleted"], true);
}

// ============================================================
// Admin Stats
// ============================================================

#[tokio::test]
async fn admin_stats() {
    let ctx = ctx().await;

    // Seed some products
    let mut data = HashMap::new();
    data.insert("name".to_string(), serde_json::json!("Active Product"));
    data.insert("status".to_string(), serde_json::json!("active"));
    seed(&ctx, "suppers_ai__products__products", "p1", data).await;

    let mut data2 = HashMap::new();
    data2.insert("name".to_string(), serde_json::json!("Draft Product"));
    data2.insert("status".to_string(), serde_json::json!("draft"));
    seed(&ctx, "suppers_ai__products__products", "p2", data2).await;

    // Seed a completed purchase (user_id is NOT NULL in the real schema)
    let mut purchase_data = HashMap::new();
    purchase_data.insert("user_id".to_string(), serde_json::json!("user_1"));
    purchase_data.insert("status".to_string(), serde_json::json!("completed"));
    purchase_data.insert("total_cents".to_string(), serde_json::json!(2999));
    seed(
        &ctx,
        "suppers_ai__products__purchases",
        "pur1",
        purchase_data,
    )
    .await;

    let (msg, input) = admin_get_msg("/admin/b/products/stats");
    let out = handlers::handle_admin(&ctx, &msg, input).await;
    let body = output_to_json(out).await;
    assert_eq!(body["total_products"].as_i64().unwrap(), 2);
    assert_eq!(body["active_products"].as_i64().unwrap(), 1);
    assert_eq!(body["total_purchases"].as_i64().unwrap(), 1);
    assert!((body["total_revenue"].as_f64().unwrap() - 2999.0).abs() < 0.01);
}

// ============================================================
// User Product CRUD — ownership isolation
// ============================================================

async fn user_products_ctx() -> crate::test_support::TestContext {
    ctx_with(&[("SOLOBASE_SHARED__ALLOW_USER_PRODUCTS", "true")]).await
}

#[tokio::test]
async fn user_create_product_in_own_group() {
    let ctx = user_products_ctx().await;

    // Create a group for user_1
    let (create_group, cg_input) = create_msg(
        "/b/products/groups",
        "user_1",
        serde_json::json!({
            "name": "My Store"
        }),
    );
    let group_out = handlers::handle_user(&ctx, &create_group, cg_input).await;
    let group_id = output_to_json(group_out).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a product in that group
    let (create_prod, cp_input) = create_msg(
        "/b/products/products",
        "user_1",
        serde_json::json!({
            "name": "Widget",
            "base_price": 19.99,
            "group_id": group_id
        }),
    );
    let out = handlers::handle_user(&ctx, &create_prod, cp_input).await;
    let body = output_to_json(out).await;
    assert_eq!(body["data"]["name"], "Widget");
    assert_eq!(body["data"]["created_by"], "user_1");
}

#[tokio::test]
async fn user_cannot_create_product_in_other_users_group() {
    let ctx = user_products_ctx().await;

    // Create a group for user_1
    let (create_group, cg_input) = create_msg(
        "/b/products/groups",
        "user_1",
        serde_json::json!({
            "name": "User1 Store"
        }),
    );
    let group_out = handlers::handle_user(&ctx, &create_group, cg_input).await;
    let group_id = output_to_json(group_out).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    // user_2 tries to create a product in user_1's group
    let (create_prod, cp_input) = create_msg(
        "/b/products/products",
        "user_2",
        serde_json::json!({
            "name": "Sneaky Product",
            "group_id": group_id
        }),
    );
    let out = handlers::handle_user(&ctx, &create_prod, cp_input).await;
    assert!(output_is_error(out, ErrorCode::InvalidArgument).await);
}

#[tokio::test]
async fn user_cannot_see_other_users_products() {
    let ctx = user_products_ctx().await;

    // user_1 creates a product
    let (create, create_input) = create_msg(
        "/b/products/products",
        "user_1",
        serde_json::json!({
            "name": "Private Product"
        }),
    );
    let create_out = handlers::handle_user(&ctx, &create, create_input).await;
    let prod_id = output_to_json(create_out).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    // user_2 tries to get it
    let (get, get_input) = get_msg(&format!("/b/products/products/{}", prod_id), "user_2");
    let out = handlers::handle_user(&ctx, &get, get_input).await;
    assert!(output_is_error(out, ErrorCode::NotFound).await);
}

#[tokio::test]
async fn user_cannot_update_other_users_products() {
    let ctx = user_products_ctx().await;

    let (create, create_input) = create_msg(
        "/b/products/products",
        "user_1",
        serde_json::json!({
            "name": "My Product"
        }),
    );
    let create_out = handlers::handle_user(&ctx, &create, create_input).await;
    let prod_id = output_to_json(create_out).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (update, update_input) = update_msg(
        &format!("/b/products/products/{}", prod_id),
        "user_2",
        serde_json::json!({
            "name": "Hijacked!"
        }),
    );
    let out = handlers::handle_user(&ctx, &update, update_input).await;
    assert!(output_is_error(out, ErrorCode::NotFound).await);
}

#[tokio::test]
async fn user_cannot_delete_other_users_products() {
    let ctx = user_products_ctx().await;

    let (create, create_input) = create_msg(
        "/b/products/products",
        "user_1",
        serde_json::json!({
            "name": "My Product"
        }),
    );
    let create_out = handlers::handle_user(&ctx, &create, create_input).await;
    let prod_id = output_to_json(create_out).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (del, del_input) = delete_msg(&format!("/b/products/products/{}", prod_id), "user_2");
    let out = handlers::handle_user(&ctx, &del, del_input).await;
    assert!(output_is_error(out, ErrorCode::NotFound).await);
}

#[tokio::test]
async fn user_list_only_own_products() {
    let ctx = user_products_ctx().await;

    // user_1 creates a product
    let (c1, c1_input) = create_msg(
        "/b/products/products",
        "user_1",
        serde_json::json!({"name": "U1 Product"}),
    );
    handlers::handle_user(&ctx, &c1, c1_input).await;

    // user_2 creates a product
    let (c2, c2_input) = create_msg(
        "/b/products/products",
        "user_2",
        serde_json::json!({"name": "U2 Product"}),
    );
    handlers::handle_user(&ctx, &c2, c2_input).await;

    // user_1 lists — should only see their own
    let (list, list_input) = get_msg("/b/products/products", "user_1");
    let out = handlers::handle_user(&ctx, &list, list_input).await;
    let body = output_to_json(out).await;
    let records = body["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["data"]["name"], "U1 Product");
}

#[tokio::test]
async fn user_update_prevents_ownership_change() {
    let ctx = user_products_ctx().await;

    let (create, create_input) = create_msg(
        "/b/products/products",
        "user_1",
        serde_json::json!({"name": "Mine"}),
    );
    let create_out = handlers::handle_user(&ctx, &create, create_input).await;
    let prod_id = output_to_json(create_out).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Try to change created_by — should be stripped
    let (update, update_input) = update_msg(
        &format!("/b/products/products/{}", prod_id),
        "user_1",
        serde_json::json!({
            "name": "Updated",
            "created_by": "attacker"
        }),
    );
    let out = handlers::handle_user(&ctx, &update, update_input).await;
    let body = output_to_json(out).await;
    assert_eq!(body["data"]["created_by"], "user_1");
}

// ============================================================
// User Group CRUD — ownership isolation
// ============================================================

#[tokio::test]
async fn user_list_only_own_groups() {
    let ctx = user_products_ctx().await;

    let (g1, g1_input) = create_msg(
        "/b/products/groups",
        "user_1",
        serde_json::json!({"name": "U1 Group"}),
    );
    handlers::handle_user(&ctx, &g1, g1_input).await;

    let (g2, g2_input) = create_msg(
        "/b/products/groups",
        "user_2",
        serde_json::json!({"name": "U2 Group"}),
    );
    handlers::handle_user(&ctx, &g2, g2_input).await;

    let (list, list_input) = get_msg("/b/products/groups", "user_1");
    let out = handlers::handle_user(&ctx, &list, list_input).await;
    let body = output_to_json(out).await;
    let records = body["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["data"]["name"], "U1 Group");
}

#[tokio::test]
async fn user_cannot_update_other_users_group() {
    let ctx = user_products_ctx().await;

    let (create, create_input) = create_msg(
        "/b/products/groups",
        "user_1",
        serde_json::json!({"name": "My Group"}),
    );
    let create_out = handlers::handle_user(&ctx, &create, create_input).await;
    let group_id = output_to_json(create_out).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (update, update_input) = update_msg(
        &format!("/b/products/groups/{}", group_id),
        "user_2",
        serde_json::json!({
            "name": "Stolen"
        }),
    );
    let out = handlers::handle_user(&ctx, &update, update_input).await;
    assert!(output_is_error(out, ErrorCode::NotFound).await);
}

#[tokio::test]
async fn user_group_update_prevents_ownership_change() {
    let ctx = user_products_ctx().await;

    let (create, create_input) = create_msg(
        "/b/products/groups",
        "user_1",
        serde_json::json!({"name": "My Group"}),
    );
    let create_out = handlers::handle_user(&ctx, &create, create_input).await;
    let group_id = output_to_json(create_out).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (update, update_input) = update_msg(
        &format!("/b/products/groups/{}", group_id),
        "user_1",
        serde_json::json!({
            "name": "Renamed",
            "user_id": "attacker"
        }),
    );
    let out = handlers::handle_user(&ctx, &update, update_input).await;
    let body = output_to_json(out).await;
    assert_eq!(body["data"]["user_id"], "user_1");
}

// ============================================================
// Public Catalog
// ============================================================

#[tokio::test]
async fn catalog_only_shows_active_products() {
    let ctx = ctx().await;

    let mut d1 = HashMap::new();
    d1.insert("name".to_string(), serde_json::json!("Active"));
    d1.insert("status".to_string(), serde_json::json!("active"));
    seed(&ctx, "suppers_ai__products__products", "p_active", d1).await;

    let mut d2 = HashMap::new();
    d2.insert("name".to_string(), serde_json::json!("Draft"));
    d2.insert("status".to_string(), serde_json::json!("draft"));
    seed(&ctx, "suppers_ai__products__products", "p_draft", d2).await;

    let (msg, input) = get_msg("/b/products/catalog", "");
    let out = handlers::handle_user(&ctx, &msg, input).await;
    let body = output_to_json(out).await;
    let records = body["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["data"]["name"], "Active");
}

#[tokio::test]
async fn catalog_get_hides_non_active() {
    let ctx = ctx().await;

    let mut d = HashMap::new();
    d.insert("name".to_string(), serde_json::json!("Hidden"));
    d.insert("status".to_string(), serde_json::json!("draft"));
    seed(&ctx, "suppers_ai__products__products", "p_hidden", d).await;

    let (msg, input) = get_msg("/b/products/catalog/p_hidden", "");
    let out = handlers::handle_user(&ctx, &msg, input).await;
    assert!(output_is_error(out, ErrorCode::NotFound).await);
}

// ============================================================
// Group products endpoint
// ============================================================

#[tokio::test]
async fn user_group_products_list() {
    let ctx = user_products_ctx().await;

    // Create group
    let (cg, cg_input) = create_msg(
        "/b/products/groups",
        "user_1",
        serde_json::json!({"name": "Store"}),
    );
    let gr = handlers::handle_user(&ctx, &cg, cg_input).await;
    let gid = output_to_json(gr).await["id"].as_str().unwrap().to_string();

    // Create product in group
    let (cp, cp_input) = create_msg(
        "/b/products/products",
        "user_1",
        serde_json::json!({
            "name": "In Group",
            "group_id": gid
        }),
    );
    handlers::handle_user(&ctx, &cp, cp_input).await;

    // List products in group
    let (list, list_input) = get_msg(&format!("/b/products/groups/{}/products", gid), "user_1");
    let out = handlers::handle_user(&ctx, &list, list_input).await;
    let body = output_to_json(out).await;
    assert!(body["records"].as_array().unwrap().len() >= 1);
}

#[tokio::test]
async fn user_cannot_list_other_users_group_products() {
    let ctx = user_products_ctx().await;

    let (cg, cg_input) = create_msg(
        "/b/products/groups",
        "user_1",
        serde_json::json!({"name": "Private"}),
    );
    let gr = handlers::handle_user(&ctx, &cg, cg_input).await;
    let gid = output_to_json(gr).await["id"].as_str().unwrap().to_string();

    // user_2 tries to list user_1's group products
    let (list, list_input) = get_msg(&format!("/b/products/groups/{}/products", gid), "user_2");
    let out = handlers::handle_user(&ctx, &list, list_input).await;
    assert!(output_is_error(out, ErrorCode::NotFound).await);
}

// ============================================================
// User products disabled by default
// ============================================================

#[tokio::test]
async fn user_products_rejected_when_disabled() {
    let ctx = ctx().await; // no ALLOW_USER_PRODUCTS config → defaults to false

    let (create, create_input) = create_msg(
        "/b/products/products",
        "user_1",
        serde_json::json!({"name": "Test"}),
    );
    let out = handlers::handle_user(&ctx, &create, create_input).await;
    assert!(output_is_error(out, ErrorCode::PermissionDenied).await);

    let (list, list_input) = get_msg("/b/products/products", "user_1");
    let out = handlers::handle_user(&ctx, &list, list_input).await;
    assert!(output_is_error(out, ErrorCode::PermissionDenied).await);

    let (group, group_input) = create_msg(
        "/b/products/groups",
        "user_1",
        serde_json::json!({"name": "Group"}),
    );
    let out = handlers::handle_user(&ctx, &group, group_input).await;
    assert!(output_is_error(out, ErrorCode::PermissionDenied).await);
}

#[tokio::test]
async fn catalog_still_works_when_user_products_disabled() {
    let ctx = ctx().await; // user products disabled

    let mut d = std::collections::HashMap::new();
    d.insert("name".to_string(), serde_json::json!("Plan"));
    d.insert("status".to_string(), serde_json::json!("active"));
    seed(&ctx, "suppers_ai__products__products", "p1", d).await;

    let (msg, input) = get_msg("/b/products/catalog", "");
    let out = handlers::handle_user(&ctx, &msg, input).await;
    let body = output_to_json(out).await;
    assert_eq!(body["records"].as_array().unwrap().len(), 1);
}

// ============================================================
// Not-found routes
// ============================================================

#[tokio::test]
async fn unknown_admin_route() {
    let ctx = ctx().await;
    let (msg, input) = admin_get_msg("/admin/b/products/nonexistent");
    let out = handlers::handle_admin(&ctx, &msg, input).await;
    assert!(output_is_error(out, ErrorCode::NotFound).await);
}

#[tokio::test]
async fn unknown_user_route() {
    let ctx = ctx().await;
    let (msg, input) = get_msg("/b/products/nonexistent", "user_1");
    let out = handlers::handle_user(&ctx, &msg, input).await;
    assert!(output_is_error(out, ErrorCode::NotFound).await);
}

// ============================================================
// Page shell (ui::shell_page) + data_table adoption
// ============================================================

/// The products pages now render through `ui::shell_page`, which derives
/// `current_path` from the request path. The overview page at `/b/products/`
/// therefore correctly highlights the "Products" portal sidebar item — the old
/// `products_page` wrapper hardcoded `current_path="/b/products/admin/"`, which
/// never matched the `/b/products/` nav href, so the item rendered inactive.
/// This is the one intended visual-baseline shift in S2-M's page-shell
/// migration; pin it so it can't silently regress.
#[tokio::test]
async fn overview_highlights_products_nav_via_request_path() {
    let ctx = ctx().await;
    let (msg, _input) = admin_get_msg("/b/products/");
    let html = output_to_html(super::super::pages::overview(&ctx, &msg).await).await;

    // Full shell chrome present (shell_page wrapped it).
    assert!(html.contains("<!DOCTYPE html>"), "expected full document");
    assert!(html.contains(r#"class="shell""#), "expected shell chrome");
    // Products portal nav item is active because current_path == its href.
    assert!(
        html.contains(r#"href="/b/products/""#),
        "Products nav item should be present"
    );
    assert!(
        html.contains("is-active"),
        "the active sidebar item (Products) should carry is-active"
    );
}

/// The products list pages adopt `ui::components::data_table`, which carries
/// the PR #75 mobile card-collapse fix via `td[data-label]`. Assert the manage
/// page renders the `.data-table` structure with per-cell data labels (so the
/// mobile baseline collapse works) instead of the old `.table-container`.
#[tokio::test]
async fn manage_products_uses_data_table_with_mobile_labels() {
    let ctx = ctx().await;
    let (c, c_input) = admin_create_msg(
        "/admin/b/products/products",
        serde_json::json!({ "name": "Widget", "base_price": 5 }),
    );
    handlers::handle_admin(&ctx, &c, c_input).await;

    let (msg, _input) = admin_get_msg("/b/products/admin/manage");
    let html = output_to_html(super::super::pages::manage_products(&ctx, &msg).await).await;

    assert!(
        html.contains(r#"class="data-table""#),
        "manage page should render the shared data_table component"
    );
    assert!(
        html.contains(r#"data-label="Name""#),
        "data_table cells should carry data-label for the mobile card collapse"
    );
    assert!(html.contains("Widget"), "the seeded product should render");
}
