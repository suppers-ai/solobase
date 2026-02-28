use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use super::get_db;

pub fn handle(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/database/info") => handle_info(ctx, msg),
        ("retrieve", "/admin/database/tables") => handle_tables(ctx, msg),
        ("retrieve", _) if path.starts_with("/admin/database/tables/") && path.ends_with("/columns") => {
            handle_columns(ctx, msg)
        }
        ("create", "/admin/database/query") => handle_query(ctx, msg),
        _ => err_not_found(msg.clone(), "not found"),
    }
}

fn handle_info(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) {
        Ok(db) => db,
        Err(r) => return r,
    };

    // Get database info via raw query
    let tables = match db.query_raw("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name", &[]) {
        Ok(t) => t,
        Err(e) => return err_internal(msg.clone(), &format!("Database error: {e}")),
    };

    let table_names: Vec<&str> = tables.iter()
        .filter_map(|r| r.data.get("name").and_then(|v| v.as_str()))
        .collect();

    json_respond(msg.clone(), 200, &serde_json::json!({
        "type": "sqlite",
        "tables": table_names,
        "table_count": table_names.len()
    }))
}

fn handle_tables(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) {
        Ok(db) => db,
        Err(r) => return r,
    };

    let tables = match db.query_raw("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name", &[]) {
        Ok(t) => t,
        Err(e) => return err_internal(msg.clone(), &format!("Database error: {e}")),
    };

    let mut table_info = Vec::new();
    for table in &tables {
        let name = table.data.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let count = db.query_raw(&format!("SELECT COUNT(*) as cnt FROM \"{}\"", name), &[])
            .ok()
            .and_then(|r| r.first().and_then(|r| r.data.get("cnt").and_then(|v| v.as_i64())))
            .unwrap_or(0);

        table_info.push(serde_json::json!({
            "name": name,
            "row_count": count
        }));
    }

    json_respond(msg.clone(), 200, &serde_json::json!(table_info))
}

fn handle_columns(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) {
        Ok(db) => db,
        Err(r) => return r,
    };

    let path = msg.path();
    // Extract table name from /admin/database/tables/{name}/columns
    let table_name = path
        .strip_prefix("/admin/database/tables/")
        .and_then(|s| s.strip_suffix("/columns"))
        .unwrap_or("");

    if table_name.is_empty() {
        return err_bad_request(msg.clone(), "Missing table name");
    }

    let columns = match db.query_raw(&format!("PRAGMA table_info(\"{}\")", table_name), &[]) {
        Ok(c) => c,
        Err(e) => return err_internal(msg.clone(), &format!("Database error: {e}")),
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

    json_respond(msg.clone(), 200, &serde_json::json!({"table": table_name, "columns": col_info}))
}

fn handle_query(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let db = match get_db(ctx) {
        Ok(db) => db,
        Err(r) => return r,
    };

    #[derive(serde::Deserialize)]
    struct QueryReq {
        query: String,
        #[serde(default)]
        args: Vec<serde_json::Value>,
    }
    let body: QueryReq = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg.clone(), &format!("Invalid body: {e}")),
    };

    let query_upper = body.query.trim().to_uppercase();
    if query_upper.starts_with("SELECT") || query_upper.starts_with("PRAGMA") || query_upper.starts_with("EXPLAIN") {
        match db.query_raw(&body.query, &body.args) {
            Ok(records) => json_respond(msg.clone(), 200, &serde_json::json!({
                "rows": records,
                "row_count": records.len()
            })),
            Err(e) => err_bad_request(msg.clone(), &format!("Query error: {e}")),
        }
    } else {
        match db.exec_raw(&body.query, &body.args) {
            Ok(affected) => json_respond(msg.clone(), 200, &serde_json::json!({
                "affected_rows": affected
            })),
            Err(e) => err_bad_request(msg.clone(), &format!("Exec error: {e}")),
        }
    }
}
