use wafer_core::clients::database as db;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::{InputStream, OutputStream};
use wafer_sql_utils::{introspect, Backend};

use crate::blocks::helpers::{err_bad_request, err_forbidden, err_internal, err_not_found, ok_json};

pub async fn handle(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/database/info") => handle_info(ctx).await,
        ("retrieve", "/admin/database/tables") => handle_tables(ctx).await,
        ("retrieve", _)
            if path.starts_with("/admin/database/tables/") && path.ends_with("/columns") =>
        {
            handle_columns(ctx, msg).await
        }
        ("create", "/admin/database/query") => handle_query(ctx, input).await,
        _ => err_not_found("not found"),
    }
}

async fn handle_info(ctx: &dyn Context) -> OutputStream {
    let sql = introspect::build_list_tables(Backend::Sqlite);
    let tables = match db::query_raw(ctx, &sql, &[]).await {
        Ok(t) => t,
        Err(e) => return err_internal(&format!("Database error: {e}")),
    };

    let table_names: Vec<&str> = tables
        .iter()
        .filter_map(|r| r.data.get("name").and_then(|v| v.as_str()))
        .collect();

    ok_json(&serde_json::json!({
        "type": "sqlite",
        "tables": table_names,
        "table_count": table_names.len()
    }))
}

async fn handle_tables(ctx: &dyn Context) -> OutputStream {
    let sql = introspect::build_list_tables(Backend::Sqlite);
    let tables = match db::query_raw(ctx, &sql, &[]).await {
        Ok(t) => t,
        Err(e) => return err_internal(&format!("Database error: {e}")),
    };

    let mut table_info = Vec::new();
    for table in &tables {
        let name = table
            .data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let count_sql = introspect::build_table_row_count(name, Backend::Sqlite);
        let count = db::query_raw(ctx, &count_sql, &[])
            .await
            .ok()
            .and_then(|r| {
                r.first()
                    .and_then(|r| r.data.get("cnt").and_then(|v| v.as_i64()))
            })
            .unwrap_or(0);

        table_info.push(serde_json::json!({
            "name": name,
            "row_count": count
        }));
    }

    ok_json(&serde_json::json!(table_info))
}

async fn handle_columns(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    // Extract table name from /admin/database/tables/{name}/columns
    let table_name = path
        .strip_prefix("/admin/database/tables/")
        .and_then(|s| s.strip_suffix("/columns"))
        .unwrap_or("");

    if table_name.is_empty() {
        return err_bad_request("Missing table name");
    }

    let (info_sql, info_args) = introspect::build_table_info(table_name, Backend::Sqlite);
    let columns = match db::query_raw(ctx, &info_sql, &info_args).await {
        Ok(c) => c,
        Err(e) => return err_internal(&format!("Database error: {e}")),
    };

    let col_info: Vec<serde_json::Value> = columns
        .iter()
        .map(|c| {
            serde_json::json!({
                "name": c.data.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                "type": c.data.get("type").and_then(|v| v.as_str()).unwrap_or(""),
                "notnull": c.data.get("notnull").and_then(|v| v.as_i64()).unwrap_or(0) == 1,
                "pk": c.data.get("pk").and_then(|v| v.as_i64()).unwrap_or(0) == 1,
                "default_value": c.data.get("dflt_value")
            })
        })
        .collect();

    ok_json(&serde_json::json!({"table": table_name, "columns": col_info}))
}

async fn handle_query(ctx: &dyn Context, input: InputStream) -> OutputStream {
    #[derive(serde::Deserialize)]
    struct QueryReq {
        query: String,
        #[serde(default)]
        args: Vec<serde_json::Value>,
    }
    let raw = input.collect_to_bytes().await;
    let body: QueryReq = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    // Strict read-only query validation
    let trimmed = body.query.trim();

    // Reject multi-statement queries (prevent piggy-backed writes)
    if trimmed.contains(';') {
        return err_forbidden("Multi-statement queries are not allowed");
    }

    let query_upper = trimmed.to_uppercase();

    // Reject queries containing write keywords anywhere (not just first word)
    const FORBIDDEN_KEYWORDS: &[&str] = &[
        "INSERT",
        "UPDATE",
        "DELETE",
        "DROP",
        "ALTER",
        "CREATE",
        "REPLACE",
        "ATTACH",
        "DETACH",
        "REINDEX",
        "VACUUM",
        "SAVEPOINT",
        "RELEASE",
        "BEGIN",
        "COMMIT",
        "ROLLBACK",
        "RETURNING",
    ];
    for keyword in FORBIDDEN_KEYWORDS {
        // Check for keyword as a whole word (preceded and followed by non-alphanumeric)
        let upper = query_upper.as_str();
        let kw = *keyword;
        let mut start = 0;
        while let Some(pos) = upper[start..].find(kw) {
            let abs_pos = start + pos;
            let before_ok = abs_pos == 0 || !upper.as_bytes()[abs_pos - 1].is_ascii_alphanumeric();
            let after_pos = abs_pos + kw.len();
            let after_ok =
                after_pos >= upper.len() || !upper.as_bytes()[after_pos].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return err_forbidden(&format!(
                    "{} is not allowed in read-only queries",
                    keyword
                ));
            }
            start = abs_pos + kw.len();
        }
    }

    let first_word = query_upper.split_whitespace().next().unwrap_or("");

    // For PRAGMA: only allow known read-only pragmas
    if first_word == "PRAGMA" {
        const SAFE_PRAGMAS: &[&str] = &[
            "TABLE_INFO",
            "TABLE_LIST",
            "TABLE_XINFO",
            "INDEX_LIST",
            "INDEX_INFO",
            "FOREIGN_KEY_LIST",
            "DATABASE_LIST",
            "COMPILE_OPTIONS",
            "INTEGRITY_CHECK",
            "QUICK_CHECK",
            "PAGE_COUNT",
            "PAGE_SIZE",
            "FREELIST_COUNT",
        ];
        let pragma_name = query_upper
            .split_whitespace()
            .nth(1)
            .unwrap_or("")
            .trim_start_matches('"')
            .split('(')
            .next()
            .unwrap_or("");
        if !SAFE_PRAGMAS.iter().any(|p| pragma_name.starts_with(p)) {
            return err_forbidden(
                "Only read-only PRAGMA queries are allowed (table_info, index_list, etc.)",
            );
        }
    }

    // User-provided query (admin SQL explorer) -- stays as raw SQL
    match first_word {
        "SELECT" | "PRAGMA" | "EXPLAIN" | "WITH" => {
            match db::query_raw(ctx, &body.query, &body.args).await {
                Ok(records) => {
                    let row_count = records.len();
                    ok_json(&serde_json::json!({
                        "rows": records,
                        "row_count": row_count
                    }))
                }
                Err(e) => err_bad_request(&format!("Query error: {e}")),
            }
        }
        _ => err_forbidden("Only SELECT, PRAGMA, EXPLAIN, and WITH queries are allowed"),
    }
}
