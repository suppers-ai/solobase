use std::collections::HashMap;

use crate::wafer::block_world::types::*;
use crate::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{ListOptions, SortField};

pub fn handle(msg: &Message) -> BlockResult {
    let action = msg_get_meta(msg, "req.action");
    let path = msg_get_meta(msg, "req.resource");

    match (action, path) {
        ("retrieve", "/admin/custom-tables") => handle_list_tables(msg),
        ("create", "/admin/custom-tables") => handle_create_table(msg),
        ("delete", p) if p.starts_with("/admin/custom-tables/") && !p.contains("/records") => {
            handle_drop_table(msg)
        }
        // Record CRUD
        ("retrieve", p) if p.contains("/records") => handle_list_records(msg),
        ("create", p) if p.contains("/records") => handle_create_record(msg),
        ("update", p) if p.contains("/records/") => handle_update_record(msg),
        ("delete", p) if p.contains("/records/") => handle_delete_record(msg),
        _ => err_not_found(msg, "not found"),
    }
}

/// Sanitize an identifier to prevent SQL injection.
fn sanitize_ident(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn extract_table_name(path: &str) -> &str {
    let rest = path.strip_prefix("/admin/custom-tables/").unwrap_or("");
    if let Some(idx) = rest.find('/') {
        &rest[..idx]
    } else {
        rest
    }
}

fn extract_record_id(path: &str) -> &str {
    // /admin/custom-tables/{table}/records/{id}
    if let Some(idx) = path.rfind("/records/") {
        &path[idx + 9..]
    } else {
        ""
    }
}

fn handle_list_tables(msg: &Message) -> BlockResult {
    let tables = match db::query_raw(
        "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'custom_%' ORDER BY name",
        &[],
    ) {
        Ok(t) => t,
        Err(e) => return err_internal(msg, &format!("Database error: {}", convert_error(e).message)),
    };

    let names: Vec<&str> = tables.iter()
        .filter_map(|r| r.data.get("name").and_then(|v| v.as_str()))
        .collect();

    json_respond(msg, &serde_json::json!({"tables": names}))
}

fn handle_create_table(msg: &Message) -> BlockResult {
    #[derive(serde::Deserialize)]
    struct Req {
        name: String,
        #[serde(default)]
        columns: Vec<ColumnDef>,
    }
    #[derive(serde::Deserialize)]
    struct ColumnDef {
        name: String,
        #[serde(rename = "type", default = "default_col_type")]
        col_type: String,
    }
    fn default_col_type() -> String { "TEXT".to_string() }

    let body: Req = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    // Sanitize name
    let table_name = format!("custom_{}", body.name.replace(|c: char| !c.is_alphanumeric() && c != '_', ""));

    let mut col_defs = vec!["id TEXT PRIMARY KEY".to_string()];
    for col in &body.columns {
        let safe_name = col.name.replace(|c: char| !c.is_alphanumeric() && c != '_', "");
        let safe_type = match col.col_type.to_uppercase().as_str() {
            "TEXT" | "INTEGER" | "REAL" | "BLOB" => col.col_type.to_uppercase(),
            _ => "TEXT".to_string(),
        };
        col_defs.push(format!("\"{}\" {}", safe_name, safe_type));
    }
    col_defs.push("created_at TEXT DEFAULT CURRENT_TIMESTAMP".to_string());
    col_defs.push("updated_at TEXT DEFAULT CURRENT_TIMESTAMP".to_string());

    let sql = format!("CREATE TABLE IF NOT EXISTS \"{}\" ({})", table_name, col_defs.join(", "));

    match db::exec_raw(&sql, &[]) {
        Ok(_) => json_respond(msg, &serde_json::json!({"table": table_name, "created": true})),
        Err(e) => err_internal(msg, &format!("Failed to create table: {}", convert_error(e).message)),
    }
}

fn handle_drop_table(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let table_name = extract_table_name(path);
    if table_name.is_empty() { return err_bad_request(msg, "Missing table name"); }

    let full_name = if table_name.starts_with("custom_") { table_name.to_string() } else { format!("custom_{}", table_name) };
    let safe_name = sanitize_ident(&full_name);

    match db::exec_raw(&format!("DROP TABLE IF EXISTS \"{}\"", safe_name), &[]) {
        Ok(_) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) => err_internal(msg, &format!("Failed to drop table: {}", convert_error(e).message)),
    }
}

fn handle_list_records(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let table_name = extract_table_name(path);
    if table_name.is_empty() { return err_bad_request(msg, "Missing table name"); }

    let full_name = if table_name.starts_with("custom_") { table_name.to_string() } else { format!("custom_{}", table_name) };

    let (_, page_size, offset) = pagination_params(msg, 20);
    let opts = ListOptions {
        sort: vec![SortField { field: "created_at".to_string(), desc: true }],
        limit: page_size,
        offset,
        ..Default::default()
    };

    match db::list(&full_name, &opts) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {}", convert_error(e).message)),
    }
}

fn handle_create_record(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let table_name = extract_table_name(path);
    if table_name.is_empty() { return err_bad_request(msg, "Missing table name"); }

    let full_name = if table_name.starts_with("custom_") { table_name.to_string() } else { format!("custom_{}", table_name) };

    let body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    match db::create(&full_name, body) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {}", convert_error(e).message)),
    }
}

fn handle_update_record(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let table_name = extract_table_name(path);
    let record_id = extract_record_id(path);
    if table_name.is_empty() || record_id.is_empty() {
        return err_bad_request(msg, "Missing table name or record ID");
    }

    let full_name = if table_name.starts_with("custom_") { table_name.to_string() } else { format!("custom_{}", table_name) };

    let body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    match db::update(&full_name, record_id, body) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => {
            let msg_str = convert_error(e).message;
            if msg_str.contains("not found") || msg_str.contains("Not found") {
                err_not_found(msg, "Record not found")
            } else {
                err_internal(msg, &format!("Database error: {}", msg_str))
            }
        }
    }
}

fn handle_delete_record(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    let table_name = extract_table_name(path);
    let record_id = extract_record_id(path);
    if table_name.is_empty() || record_id.is_empty() {
        return err_bad_request(msg, "Missing table name or record ID");
    }

    let full_name = if table_name.starts_with("custom_") { table_name.to_string() } else { format!("custom_{}", table_name) };

    match db::delete(&full_name, record_id) {
        Ok(()) => json_respond(msg, &serde_json::json!({"deleted": true})),
        Err(e) => {
            let msg_str = convert_error(e).message;
            if msg_str.contains("not found") || msg_str.contains("Not found") {
                err_not_found(msg, "Record not found")
            } else {
                err_internal(msg, &format!("Database error: {}", msg_str))
            }
        }
    }
}
