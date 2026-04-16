use std::collections::HashMap;

use wafer_core::clients::{
    database as db,
    database::{ListOptions, SortField},
};
use wafer_run::{context::Context, types::*, InputStream, OutputStream};
use wafer_sql_utils::{ddl, introspect, Backend};

use super::sanitize_ident;
use crate::blocks::helpers::{err_bad_request, err_internal, err_not_found, ok_json};

pub async fn handle(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/custom-tables") => handle_list_tables(ctx).await,
        ("create", "/admin/custom-tables") => handle_create_table(ctx, input).await,
        ("delete", _)
            if path.starts_with("/admin/custom-tables/") && !path.contains("/records") =>
        {
            handle_drop_table(ctx, msg).await
        }
        // Record CRUD
        ("retrieve", _) if path.contains("/records") => handle_list_records(ctx, msg).await,
        ("create", _) if path.contains("/records") => handle_create_record(ctx, msg, input).await,
        ("update", _) if path.contains("/records/") => handle_update_record(ctx, msg, input).await,
        ("delete", _) if path.contains("/records/") => handle_delete_record(ctx, msg).await,
        _ => err_not_found("not found"),
    }
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

async fn handle_list_tables(ctx: &dyn Context) -> OutputStream {
    let (sql, args) = introspect::build_list_tables_like("custom_", Backend::Sqlite);
    let tables = match db::query_raw(ctx, &sql, &args).await {
        Ok(t) => t,
        Err(e) => return err_internal(&format!("Database error: {e}")),
    };

    let names: Vec<&str> = tables
        .iter()
        .filter_map(|r| r.data.get("name").and_then(|v| v.as_str()))
        .collect();

    ok_json(&serde_json::json!({"tables": names}))
}

async fn handle_create_table(ctx: &dyn Context, input: InputStream) -> OutputStream {
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
    fn default_col_type() -> String {
        "TEXT".to_string()
    }

    let raw = input.collect_to_bytes().await;
    let body: Req = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    // Sanitize name
    let table_name = format!(
        "custom_{}",
        body.name
            .replace(|c: char| !c.is_alphanumeric() && c != '_', "")
    );

    let mut col_defs = vec!["id TEXT PRIMARY KEY".to_string()];
    for col in &body.columns {
        let safe_name = col
            .name
            .replace(|c: char| !c.is_alphanumeric() && c != '_', "");
        let safe_type = match col.col_type.to_uppercase().as_str() {
            "TEXT" | "INTEGER" | "REAL" | "BLOB" => col.col_type.to_uppercase(),
            _ => "TEXT".to_string(),
        };
        col_defs.push(format!("\"{}\" {}", safe_name, safe_type));
    }
    col_defs.push("created_at TEXT DEFAULT CURRENT_TIMESTAMP".to_string());
    col_defs.push("updated_at TEXT DEFAULT CURRENT_TIMESTAMP".to_string());

    let sql = format!(
        "CREATE TABLE IF NOT EXISTS \"{}\" ({})",
        table_name,
        col_defs.join(", ")
    );

    match db::exec_raw(ctx, &sql, &[]).await {
        Ok(_) => ok_json(&serde_json::json!({"table": table_name, "created": true})),
        Err(e) => err_internal(&format!("Failed to create table: {e}")),
    }
}

async fn handle_drop_table(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    let table_name = extract_table_name(path);
    if table_name.is_empty() {
        return err_bad_request("Missing table name");
    }

    let full_name = if table_name.starts_with("custom_") {
        table_name.to_string()
    } else {
        format!("custom_{}", table_name)
    };
    let safe_name = sanitize_ident(&full_name);

    let drop_sql = ddl::build_drop_table(&safe_name, Backend::Sqlite);
    match db::exec_raw(ctx, &drop_sql, &[]).await {
        Ok(_) => ok_json(&serde_json::json!({"deleted": true})),
        Err(e) => err_internal(&format!("Failed to drop table: {e}")),
    }
}

async fn handle_list_records(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    let table_name = extract_table_name(path);
    if table_name.is_empty() {
        return err_bad_request("Missing table name");
    }

    let full_name = if table_name.starts_with("custom_") {
        table_name.to_string()
    } else {
        format!("custom_{}", table_name)
    };

    let (_, page_size, offset) = msg.pagination_params(20);
    let opts = ListOptions {
        sort: vec![SortField {
            field: "created_at".to_string(),
            desc: true,
        }],
        limit: page_size as i64,
        offset: offset as i64,
        ..Default::default()
    };

    match db::list(ctx, &full_name, &opts).await {
        Ok(result) => ok_json(&result),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_create_record(
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
) -> OutputStream {
    let path = msg.path();
    let table_name = extract_table_name(path);
    if table_name.is_empty() {
        return err_bad_request("Missing table name");
    }

    let full_name = if table_name.starts_with("custom_") {
        table_name.to_string()
    } else {
        format!("custom_{}", table_name)
    };

    let raw = input.collect_to_bytes().await;
    let body: HashMap<String, serde_json::Value> = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    match db::create(ctx, &full_name, body).await {
        Ok(record) => ok_json(&record),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_update_record(
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
) -> OutputStream {
    let path = msg.path();
    let table_name = extract_table_name(path);
    let record_id = extract_record_id(path);
    if table_name.is_empty() || record_id.is_empty() {
        return err_bad_request("Missing table name or record ID");
    }

    let full_name = if table_name.starts_with("custom_") {
        table_name.to_string()
    } else {
        format!("custom_{}", table_name)
    };

    let raw = input.collect_to_bytes().await;
    let body: HashMap<String, serde_json::Value> = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    match db::update(ctx, &full_name, record_id, body).await {
        Ok(record) => ok_json(&record),
        Err(e) => {
            let msg_str = format!("{e}");
            if msg_str.contains("not found") || msg_str.contains("Not found") {
                err_not_found("Record not found")
            } else {
                err_internal(&format!("Database error: {e}"))
            }
        }
    }
}

async fn handle_delete_record(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    let table_name = extract_table_name(path);
    let record_id = extract_record_id(path);
    if table_name.is_empty() || record_id.is_empty() {
        return err_bad_request("Missing table name or record ID");
    }

    let full_name = if table_name.starts_with("custom_") {
        table_name.to_string()
    } else {
        format!("custom_{}", table_name)
    };

    match db::delete(ctx, &full_name, record_id).await {
        Ok(()) => ok_json(&serde_json::json!({"deleted": true})),
        Err(e) => {
            let msg_str = format!("{e}");
            if msg_str.contains("not found") || msg_str.contains("Not found") {
                err_not_found("Record not found")
            } else {
                err_internal(&format!("Database error: {e}"))
            }
        }
    }
}
