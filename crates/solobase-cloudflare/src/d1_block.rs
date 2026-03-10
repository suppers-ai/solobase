//! D1Block — Cloudflare D1 database as a WAFER Block.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use wafer_run::block::{Block, BlockInfo};
use wafer_run::types::*;

use crate::database::{self, D1DatabaseService, Filter, FilterOp, ListOptions, SortField};

pub struct D1Block {
    db: Arc<D1DatabaseService>,
}

// Safety: wasm32-unknown-unknown is single-threaded.
unsafe impl Send for D1Block {}
unsafe impl Sync for D1Block {}

impl D1Block {
    pub fn new(db: D1DatabaseService) -> Self {
        Self { db: Arc::new(db) }
    }
}

// --- Wire-format request/response types ---

#[derive(Deserialize)]
struct DbGetReq { collection: String, id: String }

#[derive(Deserialize)]
struct DbListReq {
    collection: String,
    #[serde(default)]
    filters: Vec<DbFilterDef>,
    #[serde(default)]
    sort: Vec<DbSortDef>,
    #[serde(default)]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

#[derive(Deserialize)]
struct DbCreateReq {
    collection: String,
    data: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct DbUpdateReq {
    collection: String,
    id: String,
    data: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct DbDeleteReq { collection: String, id: String }

#[derive(Deserialize)]
struct DbCountReq {
    collection: String,
    #[serde(default)]
    filters: Vec<DbFilterDef>,
}

#[derive(Deserialize)]
struct DbSumReq {
    collection: String,
    field: String,
    #[serde(default)]
    filters: Vec<DbFilterDef>,
}

#[derive(Deserialize)]
struct DbQueryRawReq {
    query: String,
    #[serde(default)]
    args: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct DbExecRawReq {
    query: String,
    #[serde(default)]
    args: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct DbFilterDef {
    field: String,
    #[serde(default = "default_op")]
    operator: String,
    #[serde(default)]
    value: serde_json::Value,
}

fn default_op() -> String { "eq".to_string() }

#[derive(Deserialize)]
struct DbSortDef {
    field: String,
    #[serde(default)]
    desc: bool,
}

#[derive(Serialize)]
struct CountResp { count: i64 }

#[derive(Serialize)]
struct SumResp { sum: f64 }

#[derive(Serialize)]
struct ExecRawResp { rows_affected: i64 }

// --- Helpers ---

fn parse_filter_op(op: &str) -> FilterOp {
    match op {
        "eq" | "=" | "equal" => FilterOp::Equal,
        "neq" | "!=" | "not_equal" => FilterOp::NotEqual,
        "gt" | ">" | "greater_than" => FilterOp::GreaterThan,
        "gte" | ">=" | "greater_equal" => FilterOp::GreaterEqual,
        "lt" | "<" | "less_than" => FilterOp::LessThan,
        "lte" | "<=" | "less_equal" => FilterOp::LessEqual,
        "like" => FilterOp::Like,
        "in" => FilterOp::In,
        "is_null" => FilterOp::IsNull,
        "is_not_null" => FilterOp::IsNotNull,
        _ => FilterOp::Equal,
    }
}

fn convert_filters(defs: Vec<DbFilterDef>) -> Vec<Filter> {
    defs.into_iter()
        .map(|f| Filter {
            field: f.field,
            operator: parse_filter_op(&f.operator),
            value: f.value,
        })
        .collect()
}

fn convert_sort(defs: Vec<DbSortDef>) -> Vec<SortField> {
    defs.into_iter()
        .map(|s| SortField { field: s.field, desc: s.desc })
        .collect()
}

fn respond_json<T: Serialize>(msg: &Message, data: &T) -> Result_ {
    match serde_json::to_vec(data) {
        Ok(body) => msg.clone().respond(Response { data: body, meta: HashMap::new() }),
        Err(e) => err_result("internal", e.to_string()),
    }
}

fn respond_empty(msg: &Message) -> Result_ {
    msg.clone().respond(Response { data: Vec::new(), meta: HashMap::new() })
}

fn err_result(code: &str, message: impl Into<String>) -> Result_ {
    Result_::error(WaferError::new(code, message))
}

fn decode_req<T: serde::de::DeserializeOwned>(msg: &mut Message, op: &str) -> Result<T, Result_> {
    msg.decode::<T>().map_err(|e| err_result("invalid_argument", format!("invalid {op}: {e}")))
}

// --- Block implementation ---

#[async_trait::async_trait(?Send)]
impl Block for D1Block {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "solobase/d1".to_string(),
            version: "0.1.0".to_string(),
            interface: "database@v1".to_string(),
            summary: "Cloudflare D1 database block".to_string(),
            instance_mode: InstanceMode::PerNode,
            allowed_modes: Vec::new(),
            admin_ui: None,
            runtime: BlockRuntime::Native,
            requires: Vec::new(),
        }
    }

    async fn handle(&self, _ctx: &dyn wafer_run::context::Context, msg: &mut Message) -> Result_ {
        match msg.kind.as_str() {
            "database.get" => {
                let req = decode_req::<DbGetReq>(msg, "database.get")?;
                match self.db.get(&req.collection, &req.id).await {
                    Ok(record) => respond_json(msg, &record),
                    Err(_) => err_result("not_found", "record not found"),
                }
            }
            "database.list" => {
                let req = decode_req::<DbListReq>(msg, "database.list")?;
                let opts = ListOptions {
                    filters: convert_filters(req.filters),
                    sort: convert_sort(req.sort),
                    limit: req.limit,
                    offset: req.offset,
                };
                match self.db.list(&req.collection, &opts).await {
                    Ok(list) => respond_json(msg, &list),
                    Err(e) => err_result("internal", format!("database list error: {e}")),
                }
            }
            "database.create" => {
                let req = decode_req::<DbCreateReq>(msg, "database.create")?;
                match self.db.create(&req.collection, req.data).await {
                    Ok(record) => respond_json(msg, &record),
                    Err(e) => err_result("internal", format!("database create error: {e}")),
                }
            }
            "database.update" => {
                let req = decode_req::<DbUpdateReq>(msg, "database.update")?;
                match self.db.update(&req.collection, &req.id, req.data).await {
                    Ok(record) => respond_json(msg, &record),
                    Err(e) => err_result("internal", format!("database update error: {e}")),
                }
            }
            "database.delete" => {
                let req = decode_req::<DbDeleteReq>(msg, "database.delete")?;
                match self.db.delete(&req.collection, &req.id).await {
                    Ok(()) => respond_empty(msg),
                    Err(e) => err_result("internal", format!("database delete error: {e}")),
                }
            }
            "database.count" => {
                let req = decode_req::<DbCountReq>(msg, "database.count")?;
                let filters = convert_filters(req.filters);
                match self.db.count(&req.collection, &filters).await {
                    Ok(count) => respond_json(msg, &CountResp { count }),
                    Err(e) => err_result("internal", format!("database count error: {e}")),
                }
            }
            "database.sum" => {
                let req = decode_req::<DbSumReq>(msg, "database.sum")?;
                let col = database::sanitize_ident(&req.field);
                let table = database::sanitize_ident(&req.collection);
                let filters = convert_filters(req.filters);
                // Build WHERE clause from filters
                let mut where_parts: Vec<String> = Vec::new();
                let mut args: Vec<serde_json::Value> = Vec::new();
                for f in &filters {
                    let fc = database::sanitize_ident(&f.field);
                    match f.operator {
                        FilterOp::IsNull => where_parts.push(format!("{} IS NULL", fc)),
                        FilterOp::IsNotNull => where_parts.push(format!("{} IS NOT NULL", fc)),
                        _ => {
                            where_parts.push(format!("{} {} ?", fc, f.operator.as_sql()));
                            args.push(f.value.clone());
                        }
                    }
                }
                let where_sql = if where_parts.is_empty() {
                    "1=1".to_string()
                } else {
                    where_parts.join(" AND ")
                };
                let sql = format!("SELECT COALESCE(SUM({}), 0) as s FROM {} WHERE {}", col, table, where_sql);
                match self.db.query_raw(&sql, &args).await {
                    Ok(records) => {
                        let sum = records.first()
                            .and_then(|r| r.data.get("s"))
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        respond_json(msg, &SumResp { sum })
                    }
                    Err(e) => err_result("internal", format!("database sum error: {e}")),
                }
            }
            "database.query_raw" => {
                let req = decode_req::<DbQueryRawReq>(msg, "database.query_raw")?;
                match self.db.query_raw(&req.query, &req.args).await {
                    Ok(records) => respond_json(msg, &records),
                    Err(e) => err_result("internal", format!("database query_raw error: {e}")),
                }
            }
            "database.exec_raw" => {
                let req = decode_req::<DbExecRawReq>(msg, "database.exec_raw")?;
                match self.db.exec_raw(&req.query, &req.args).await {
                    Ok(()) => respond_json(msg, &ExecRawResp { rows_affected: 0 }),
                    Err(e) => err_result("internal", format!("database exec_raw error: {e}")),
                }
            }
            other => err_result("unimplemented", format!("unknown database op: {other}")),
        }
    }

    async fn lifecycle(
        &self,
        _ctx: &dyn wafer_run::context::Context,
        _event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
