use super::{LINE_ITEMS_COLLECTION, PRICING_COLLECTION, PRODUCTS_COLLECTION, PURCHASES_COLLECTION};
use crate::blocks::helpers::RecordExt;
use std::collections::HashMap;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

pub async fn handle_create(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    #[derive(serde::Deserialize)]
    struct CreateReq {
        items: Vec<PurchaseItem>,
        currency: Option<String>,
    }
    #[derive(serde::Deserialize)]
    struct PurchaseItem {
        product_id: String,
        quantity: i64,
        #[serde(default)]
        variables: HashMap<String, f64>,
    }

    let body: CreateReq = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };

    if body.items.is_empty() {
        return err_bad_request(msg, "No items in purchase");
    }

    let currency = body.currency.unwrap_or_else(|| "USD".to_string());
    let now = chrono::Utc::now().to_rfc3339();
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return err_unauthorized(msg, "Authentication required to create a purchase");
    }

    // Calculate totals
    let mut total_amount = 0.0;
    let mut line_items_data = Vec::new();

    for item in &body.items {
        if item.quantity <= 0 {
            return err_bad_request(msg, "Quantity must be positive");
        }
        let product = match db::get(ctx, PRODUCTS_COLLECTION, &item.product_id).await {
            Ok(p) => p,
            Err(_) => return err_not_found(msg, &format!("Product {} not found", item.product_id)),
        };

        // Reject draft/deleted/inactive products
        let product_status = product
            .data
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("active");
        if product_status != "active" {
            return err_bad_request(
                msg,
                &format!(
                    "Product {} is not available for purchase (status: {})",
                    item.product_id, product_status
                ),
            );
        }
        if !product
            .data
            .get("deleted_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .is_empty()
        {
            return err_not_found(msg, &format!("Product {} not found", item.product_id));
        }

        let product_name = product
            .data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        // Calculate price — use pricing template if set, otherwise fall back to base_price
        let unit_price = if let Some(template_id) = product
            .data
            .get("pricing_template_id")
            .and_then(|v| v.as_str())
        {
            if !template_id.is_empty() {
                if let Ok(template) = db::get(ctx, PRICING_COLLECTION, template_id).await {
                    let formula = template
                        .data
                        .get("price_formula")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0");
                    super::pricing::evaluate_formula(formula, &item.variables).unwrap_or(0.0)
                } else {
                    product
                        .data
                        .get("base_price")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0)
                }
            } else {
                product
                    .data
                    .get("base_price")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
            }
        } else {
            product
                .data
                .get("base_price")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
        };

        // Reject negative or zero unit prices (prevent price manipulation via variables)
        if unit_price < 0.0 {
            return err_bad_request(
                msg,
                &format!(
                    "Invalid price for product {}: price cannot be negative",
                    item.product_id
                ),
            );
        }

        let line_total = unit_price * item.quantity as f64;
        total_amount += line_total;

        line_items_data.push((
            item.product_id.clone(),
            product_name,
            item.quantity,
            unit_price,
            line_total,
            &item.variables,
        ));
    }

    let total_cents = (total_amount * 100.0).round() as i64;
    if total_cents <= 0 {
        return err_bad_request(msg, "Purchase total must be greater than zero");
    }

    // Create purchase
    let mut purchase_data = HashMap::new();
    purchase_data.insert(
        "user_id".to_string(),
        serde_json::Value::String(user_id.clone()),
    );
    purchase_data.insert(
        "status".to_string(),
        serde_json::Value::String("pending".to_string()),
    );
    purchase_data.insert("total_cents".to_string(), serde_json::json!(total_cents));
    purchase_data.insert("amount_cents".to_string(), serde_json::json!(total_cents));
    purchase_data.insert("currency".to_string(), serde_json::Value::String(currency));
    purchase_data.insert(
        "provider".to_string(),
        serde_json::Value::String("manual".to_string()),
    );
    purchase_data.insert(
        "created_at".to_string(),
        serde_json::Value::String(now.clone()),
    );
    purchase_data.insert(
        "updated_at".to_string(),
        serde_json::Value::String(now.clone()),
    );

    let purchase = match db::create(ctx, PURCHASES_COLLECTION, purchase_data).await {
        Ok(p) => p,
        Err(e) => return err_internal(msg, &format!("Failed to create purchase: {e}")),
    };

    // Create line items — roll back purchase on failure
    for (product_id, product_name, qty, unit_price, line_total, variables) in &line_items_data {
        let mut item_data = HashMap::new();
        item_data.insert(
            "purchase_id".to_string(),
            serde_json::Value::String(purchase.id.clone()),
        );
        item_data.insert(
            "product_id".to_string(),
            serde_json::Value::String(product_id.clone()),
        );
        item_data.insert(
            "product_name".to_string(),
            serde_json::Value::String(product_name.clone()),
        );
        item_data.insert("quantity".to_string(), serde_json::json!(qty));
        item_data.insert("unit_price".to_string(), serde_json::json!(unit_price));
        item_data.insert("total_price".to_string(), serde_json::json!(line_total));
        item_data.insert("variables".to_string(), serde_json::json!(variables));
        item_data.insert(
            "created_at".to_string(),
            serde_json::Value::String(now.clone()),
        );
        if let Err(e) = db::create(ctx, LINE_ITEMS_COLLECTION, item_data).await {
            // Clean up the purchase since line items are incomplete
            let _ = db::delete(ctx, PURCHASES_COLLECTION, &purchase.id).await;
            return err_internal(msg, &format!("Failed to create line item: {e}"));
        }
    }

    json_respond(
        msg,
        &serde_json::json!({
            "id": purchase.id,
            "status": "pending",
            "total_cents": total_cents,
            "item_count": line_items_data.len()
        }),
    )
}

pub async fn handle_list_user(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    let (page, page_size, _) = msg.pagination_params(20);

    let filters = vec![Filter {
        field: "user_id".to_string(),
        operator: FilterOp::Equal,
        value: serde_json::Value::String(user_id),
    }];
    let sort = vec![SortField {
        field: "created_at".to_string(),
        desc: true,
    }];

    match db::paginated_list(
        ctx,
        PURCHASES_COLLECTION,
        page as i64,
        page_size as i64,
        filters,
        sort,
    )
    .await
    {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

pub async fn handle_list_admin(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let (page, page_size, _) = msg.pagination_params(20);

    let mut filters = Vec::new();
    let status = msg.query("status").to_string();
    if !status.is_empty() {
        filters.push(Filter {
            field: "status".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(status),
        });
    }
    let user_id = msg.query("user_id").to_string();
    if !user_id.is_empty() {
        filters.push(Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id),
        });
    }

    let sort = vec![SortField {
        field: "created_at".to_string(),
        desc: true,
    }];

    match db::paginated_list(
        ctx,
        PURCHASES_COLLECTION,
        page as i64,
        page_size as i64,
        filters,
        sort,
    )
    .await
    {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

pub async fn handle_get(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.rsplit('/').next().unwrap_or("");
    if id.is_empty() || id == "purchases" {
        return err_bad_request(msg, "Missing purchase ID");
    }

    let purchase = match db::get(ctx, PURCHASES_COLLECTION, id).await {
        Ok(p) => p,
        Err(e) if e.code == ErrorCode::NotFound => return err_not_found(msg, "Purchase not found"),
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    };

    // Verify access: user can only view their own, admin can view all
    let purchase_user = purchase.str_field("user_id");
    if purchase_user != msg.user_id()
        && !msg
            .get_meta("auth.user_roles")
            .split(',')
            .any(|r| r.trim() == "admin")
    {
        return err_forbidden(msg, "Access denied");
    }

    // Get line items
    let items_opts = ListOptions {
        filters: vec![Filter {
            field: "purchase_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(id.to_string()),
        }],
        ..Default::default()
    };
    let line_items = db::list(ctx, LINE_ITEMS_COLLECTION, &items_opts)
        .await
        .map(|r| r.records)
        .unwrap_or_default();

    json_respond(
        msg,
        &serde_json::json!({
            "purchase": purchase,
            "line_items": line_items
        }),
    )
}

pub async fn handle_refund(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    // /admin/b/products/purchases/{id}/refund
    let id = path
        .strip_prefix("/admin/b/products/purchases/")
        .and_then(|s| s.strip_suffix("/refund"))
        .unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, "Missing purchase ID");
    }

    #[derive(serde::Deserialize, Default)]
    struct RefundReq {
        reason: Option<String>,
    }
    let body: RefundReq = msg.decode().unwrap_or_default();

    // Verify purchase exists
    if let Err(e) = db::get(ctx, PURCHASES_COLLECTION, id).await {
        if e.code == ErrorCode::NotFound {
            return err_not_found(msg, "Purchase not found");
        }
        return err_internal(msg, &format!("Database error: {e}"));
    }

    // Atomic status transition: completed → refunded (prevents double-refund race)
    let now = chrono::Utc::now().to_rfc3339();
    let refunded_by = msg.user_id().to_string();
    let reason_val = body.reason.unwrap_or_default();

    let rows = db::exec_raw(
        ctx,
        "UPDATE block_products_purchases SET status = 'refunded', refunded_at = ?1, refunded_by = ?2, refund_reason = ?3, updated_at = ?1 WHERE id = ?4 AND status = 'completed'",
        &[
            serde_json::Value::String(now),
            serde_json::Value::String(refunded_by),
            serde_json::Value::String(reason_val),
            serde_json::Value::String(id.to_string()),
        ],
    ).await.unwrap_or(0);

    if rows == 0 {
        return err_bad_request(
            msg,
            "Can only refund completed purchases (status may have changed concurrently)",
        );
    }

    // Fetch the updated record for the response
    match db::get(ctx, PURCHASES_COLLECTION, id).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}
