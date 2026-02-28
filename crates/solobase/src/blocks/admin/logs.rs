use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_run::services::database::{self, Filter, FilterOp, SortField};
use super::get_db;

const COLLECTION: &str = "audit_logs";

pub fn handle(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/logs") => handle_list(ctx, msg),
        _ => err_not_found(msg.clone(), "not found"),
    }
}

fn handle_list(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) { Ok(db) => db, Err(r) => return r };

    let (page, page_size, _) = msg.pagination_params(50);

    let mut filters = Vec::new();
    let user_id = msg.query("user_id").to_string();
    if !user_id.is_empty() {
        filters.push(Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id),
        });
    }
    let action_filter = msg.query("action").to_string();
    if !action_filter.is_empty() {
        filters.push(Filter {
            field: "action".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(action_filter),
        });
    }
    let resource = msg.query("resource").to_string();
    if !resource.is_empty() {
        filters.push(Filter {
            field: "resource".to_string(),
            operator: FilterOp::Like,
            value: serde_json::Value::String(format!("%{}%", resource)),
        });
    }

    let sort = vec![SortField { field: "created_at".to_string(), desc: true }];

    match database::paginated_list(db.as_ref(), COLLECTION, page as i64, page_size as i64, filters, sort) {
        Ok(result) => json_respond(msg.clone(), 200, &result),
        Err(e) => err_internal(msg.clone(), &format!("Database error: {e}")),
    }
}
