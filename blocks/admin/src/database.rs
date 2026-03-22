use crate::wafer::block_world::types::*;
use crate::helpers::*;
use wafer_core::clients::database as db;

pub fn handle(msg: &Message) -> BlockResult {
    let action = msg_get_meta(msg, "req.action");
    let path = msg_get_meta(msg, "req.resource");

    match (action, path) {
        ("retrieve", "/admin/database/info") => handle_info(msg),
        ("retrieve", "/admin/database/tables") => handle_tables(msg),
        ("retrieve", p) if p.starts_with("/admin/database/tables/") && p.ends_with("/columns") => {
            handle_columns(msg)
        }
        ("create", "/admin/database/query") => handle_query(msg),
        _ => err_not_found(msg, "not found"),
    }
}

/// Sanitize an identifier to prevent SQL injection. Only allows
/// alphanumeric characters and underscores.
fn sanitize_ident(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn handle_info(msg: &Message) -> BlockResult {
    let tables = match db::query_raw(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        &[],
    ) {
        Ok(t) => t,
        Err(e) => return err_internal(msg, &format!("Database error: {}", convert_error(e).message)),
    };

    let table_names: Vec<&str> = tables.iter()
        .filter_map(|r| r.data.get("name").and_then(|v| v.as_str()))
        .collect();

    json_respond(msg, &serde_json::json!({
        "type": "sqlite",
        "tables": table_names,
        "table_count": table_names.len()
    }))
}

fn handle_tables(msg: &Message) -> BlockResult {
    let tables = match db::query_raw(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        &[],
    ) {
        Ok(t) => t,
        Err(e) => return err_internal(msg, &format!("Database error: {}", convert_error(e).message)),
    };

    let mut table_info = Vec::new();
    for table in &tables {
        let name = table.data.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let safe_name = sanitize_ident(name);
        let count = db::query_raw(&format!("SELECT COUNT(*) as cnt FROM \"{}\"", safe_name), &[])
            .ok()
            .and_then(|r| r.first().and_then(|r| r.data.get("cnt").and_then(|v| v.as_i64())))
            .unwrap_or(0);

        table_info.push(serde_json::json!({
            "name": name,
            "row_count": count
        }));
    }

    json_respond(msg, &serde_json::json!(table_info))
}

fn handle_columns(msg: &Message) -> BlockResult {
    let path = msg_get_meta(msg, "req.resource");
    // Extract table name from /admin/database/tables/{name}/columns
    let table_name = path
        .strip_prefix("/admin/database/tables/")
        .and_then(|s| s.strip_suffix("/columns"))
        .unwrap_or("");

    if table_name.is_empty() {
        return err_bad_request(msg, "Missing table name");
    }

    let safe_table = sanitize_ident(table_name);
    let columns = match db::query_raw(&format!("PRAGMA table_info(\"{}\")", safe_table), &[]) {
        Ok(c) => c,
        Err(e) => return err_internal(msg, &format!("Database error: {}", convert_error(e).message)),
    };

    let col_info: Vec<serde_json::Value> = columns.iter().map(|c| {
        serde_json::json!({
            "name": c.data.get("name").and_then(|v| v.as_str()).unwrap_or(""),
            "type": c.data.get("type").and_then(|v| v.as_str()).unwrap_or(""),
            "notnull": c.data.get("notnull").and_then(|v| v.as_i64()).unwrap_or(0) == 1,
            "pk": c.data.get("pk").and_then(|v| v.as_i64()).unwrap_or(0) == 1,
            "default_value": c.data.get("dflt_value")
        })
    }).collect();

    json_respond(msg, &serde_json::json!({"table": table_name, "columns": col_info}))
}

fn handle_query(msg: &Message) -> BlockResult {
    #[derive(serde::Deserialize)]
    struct QueryReq {
        query: String,
        #[serde(default)]
        args: Vec<serde_json::Value>,
    }

    let body: QueryReq = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    // Only allow read-only queries (SELECT, PRAGMA, EXPLAIN) to prevent data modification
    let query_upper = body.query.trim().to_uppercase();
    let first_word = query_upper.split_whitespace().next().unwrap_or("");
    match first_word {
        "SELECT" | "PRAGMA" | "EXPLAIN" => {
            match db::query_raw(&body.query, &body.args) {
                Ok(records) => json_respond(msg, &serde_json::json!({
                    "rows": records,
                    "row_count": records.len()
                })),
                Err(e) => err_bad_request(msg, &format!("Query error: {}", convert_error(e).message)),
            }
        }
        _ => {
            err_forbidden(msg, "Only SELECT, PRAGMA, and EXPLAIN queries are allowed")
        }
    }
}
