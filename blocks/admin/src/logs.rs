use crate::wafer::block_world::types::*;
use crate::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, SortField};

const COLLECTION: &str = "audit_logs";

pub fn handle(msg: &Message) -> BlockResult {
    let action = msg_get_meta(msg, "req.action");
    let path = msg_get_meta(msg, "req.resource");

    match (action, path) {
        ("retrieve", "/admin/logs") => handle_list(msg),
        _ => err_not_found(msg, "not found"),
    }
}

fn handle_list(msg: &Message) -> BlockResult {
    let (page, page_size, _) = pagination_params(msg, 50);

    let mut filters = Vec::new();
    let user_id = msg_query(msg, "user_id");
    if !user_id.is_empty() {
        filters.push(Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.to_string()),
        });
    }
    let action_filter = msg_query(msg, "action");
    if !action_filter.is_empty() {
        filters.push(Filter {
            field: "action".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(action_filter.to_string()),
        });
    }
    let resource = msg_query(msg, "resource");
    if !resource.is_empty() {
        filters.push(Filter {
            field: "resource".to_string(),
            operator: FilterOp::Like,
            value: serde_json::Value::String(format!("%{}%", resource)),
        });
    }

    let sort = vec![SortField { field: "created_at".to_string(), desc: true }];

    match db::paginated_list(COLLECTION, page, page_size, filters, sort) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {}", convert_error(e).message)),
    }
}
