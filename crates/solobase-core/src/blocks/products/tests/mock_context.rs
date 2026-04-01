use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use wafer_core::clients::database::{Record, RecordList};
use wafer_run::context::Context;
use wafer_run::types::*;

/// In-memory database: collection name → Vec<Record>
type Db = HashMap<String, Vec<Record>>;

/// MockContext provides an in-memory database and config store for testing.
pub struct MockContext {
    db: Arc<Mutex<Db>>,
    config: HashMap<String, String>,
    next_id: AtomicU64,
}

impl MockContext {
    pub fn new() -> Self {
        Self {
            db: Arc::new(Mutex::new(HashMap::new())),
            config: HashMap::new(),
            next_id: AtomicU64::new(1),
        }
    }

    pub fn with_config(mut self, key: &str, value: &str) -> Self {
        self.config.insert(key.to_string(), value.to_string());
        self
    }

    /// Insert a record directly for test setup.
    pub fn seed(&self, collection: &str, id: &str, data: HashMap<String, serde_json::Value>) {
        let mut db = self.db.lock().unwrap();
        let records = db.entry(collection.to_string()).or_default();
        records.push(Record {
            id: id.to_string(),
            data,
        });
    }

    fn next_id(&self) -> String {
        self.next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            .to_string()
    }

    fn handle_db_call(&self, kind: &str, data: &[u8]) -> Result<Vec<u8>, WaferError> {
        match kind {
            "database.get" => self.db_get(data),
            "database.list" => self.db_list(data),
            "database.create" => self.db_create(data),
            "database.update" => self.db_update(data),
            "database.delete" => self.db_delete(data),
            "database.count" => self.db_count(data),
            "database.sum" => self.db_sum(data),
            "database.exec_raw" => self.db_exec_raw(data),
            _ => Err(WaferError::new(
                "not_implemented",
                format!("unhandled db op: {kind}"),
            )),
        }
    }

    fn handle_config_call(&self, _kind: &str, data: &[u8]) -> Result<Vec<u8>, WaferError> {
        #[derive(serde::Deserialize)]
        struct Req {
            key: String,
        }
        let req: Req =
            serde_json::from_slice(data).map_err(|e| WaferError::new("internal", e.to_string()))?;
        match self.config.get(&req.key) {
            Some(v) => Ok(serde_json::to_vec(&serde_json::json!({"value": v})).unwrap()),
            None => Err(WaferError::new(
                "not_found",
                format!("config key '{}' not found", req.key),
            )),
        }
    }

    // --- Database operations ---

    fn db_get(&self, data: &[u8]) -> Result<Vec<u8>, WaferError> {
        #[derive(serde::Deserialize)]
        struct Req {
            collection: String,
            id: String,
        }
        let req: Req =
            serde_json::from_slice(data).map_err(|e| WaferError::new("internal", e.to_string()))?;
        let db = self.db.lock().unwrap();
        let records = db
            .get(&req.collection)
            .ok_or_else(|| WaferError::new("not_found", "not found"))?;
        let record = records
            .iter()
            .find(|r| r.id == req.id)
            .ok_or_else(|| WaferError::new("not_found", "not found"))?;
        Ok(serde_json::to_vec(record).unwrap())
    }

    fn db_list(&self, data: &[u8]) -> Result<Vec<u8>, WaferError> {
        #[derive(serde::Deserialize)]
        struct Req {
            collection: String,
            #[serde(default)]
            filters: Vec<FilterDef>,
            #[serde(default)]
            sort: Vec<SortDef>,
            #[serde(default = "default_limit")]
            limit: i64,
            #[serde(default)]
            offset: i64,
        }
        fn default_limit() -> i64 {
            1000
        }
        #[derive(serde::Deserialize)]
        struct SortDef {
            field: String,
            #[serde(default)]
            desc: bool,
        }

        let req: Req =
            serde_json::from_slice(data).map_err(|e| WaferError::new("internal", e.to_string()))?;

        let db = self.db.lock().unwrap();
        let empty = Vec::new();
        let records = db.get(&req.collection).unwrap_or(&empty);

        let mut filtered: Vec<&Record> = records
            .iter()
            .filter(|r| req.filters.iter().all(|f| matches_filter(r, f)))
            .collect();

        // Sort
        for s in req.sort.iter().rev() {
            filtered.sort_by(|a, b| {
                let va = a.data.get(&s.field);
                let vb = b.data.get(&s.field);
                let cmp = compare_json_values(va, vb);
                if s.desc {
                    cmp.reverse()
                } else {
                    cmp
                }
            });
        }

        let total_count = filtered.len() as i64;
        let offset = req.offset.max(0) as usize;
        let limit = req.limit.max(0) as usize;
        let page_records: Vec<Record> = filtered
            .into_iter()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect();

        let page = if req.limit > 0 {
            req.offset / req.limit + 1
        } else {
            1
        };
        let result = RecordList {
            records: page_records,
            total_count,
            page,
            page_size: req.limit,
        };
        Ok(serde_json::to_vec(&result).unwrap())
    }

    fn db_create(&self, data: &[u8]) -> Result<Vec<u8>, WaferError> {
        #[derive(serde::Deserialize)]
        struct Req {
            collection: String,
            data: HashMap<String, serde_json::Value>,
        }
        let req: Req =
            serde_json::from_slice(data).map_err(|e| WaferError::new("internal", e.to_string()))?;

        let id = self.next_id();
        let record = Record { id, data: req.data };
        let mut db = self.db.lock().unwrap();
        db.entry(req.collection).or_default().push(record.clone());
        Ok(serde_json::to_vec(&record).unwrap())
    }

    fn db_update(&self, data: &[u8]) -> Result<Vec<u8>, WaferError> {
        #[derive(serde::Deserialize)]
        struct Req {
            collection: String,
            id: String,
            data: HashMap<String, serde_json::Value>,
        }
        let req: Req =
            serde_json::from_slice(data).map_err(|e| WaferError::new("internal", e.to_string()))?;

        let mut db = self.db.lock().unwrap();
        let records = db
            .get_mut(&req.collection)
            .ok_or_else(|| WaferError::new("not_found", "not found"))?;
        let record = records
            .iter_mut()
            .find(|r| r.id == req.id)
            .ok_or_else(|| WaferError::new("not_found", "not found"))?;
        for (k, v) in req.data {
            record.data.insert(k, v);
        }
        Ok(serde_json::to_vec(&record).unwrap())
    }

    fn db_delete(&self, data: &[u8]) -> Result<Vec<u8>, WaferError> {
        #[derive(serde::Deserialize)]
        struct Req {
            collection: String,
            id: String,
        }
        let req: Req =
            serde_json::from_slice(data).map_err(|e| WaferError::new("internal", e.to_string()))?;

        let mut db = self.db.lock().unwrap();
        let records = db
            .get_mut(&req.collection)
            .ok_or_else(|| WaferError::new("not_found", "not found"))?;
        let idx = records
            .iter()
            .position(|r| r.id == req.id)
            .ok_or_else(|| WaferError::new("not_found", "not found"))?;
        records.remove(idx);
        Ok(b"{}".to_vec())
    }

    fn db_count(&self, data: &[u8]) -> Result<Vec<u8>, WaferError> {
        #[derive(serde::Deserialize)]
        struct Req {
            collection: String,
            #[serde(default)]
            filters: Vec<FilterDef>,
        }

        let req: Req =
            serde_json::from_slice(data).map_err(|e| WaferError::new("internal", e.to_string()))?;

        let db = self.db.lock().unwrap();
        let empty = Vec::new();
        let records = db.get(&req.collection).unwrap_or(&empty);
        let count = records
            .iter()
            .filter(|r| req.filters.iter().all(|f| matches_filter(r, f)))
            .count() as i64;
        Ok(serde_json::to_vec(&serde_json::json!({"count": count})).unwrap())
    }

    fn db_sum(&self, data: &[u8]) -> Result<Vec<u8>, WaferError> {
        #[derive(serde::Deserialize)]
        struct Req {
            collection: String,
            field: String,
            #[serde(default)]
            filters: Vec<FilterDef>,
        }

        let req: Req =
            serde_json::from_slice(data).map_err(|e| WaferError::new("internal", e.to_string()))?;

        let db = self.db.lock().unwrap();
        let empty = Vec::new();
        let records = db.get(&req.collection).unwrap_or(&empty);
        let sum: f64 = records
            .iter()
            .filter(|r| req.filters.iter().all(|f| matches_filter(r, f)))
            .filter_map(|r| r.data.get(&req.field).and_then(|v| v.as_f64()))
            .sum();
        Ok(serde_json::to_vec(&serde_json::json!({"sum": sum})).unwrap())
    }

    /// Minimal exec_raw support for atomic UPDATE ... WHERE id = ? AND status = ? patterns.
    fn db_exec_raw(&self, data: &[u8]) -> Result<Vec<u8>, WaferError> {
        #[derive(serde::Deserialize)]
        struct Req {
            query: String,
            #[serde(default)]
            args: Vec<serde_json::Value>,
        }
        let req: Req =
            serde_json::from_slice(data).map_err(|e| WaferError::new("internal", e.to_string()))?;

        let query_upper = req.query.to_uppercase();

        // Match: UPDATE <table> SET ... WHERE id = ?N AND status IN (...)
        // or: UPDATE <table> SET ... WHERE id = ?N AND status = ?M
        if !query_upper.starts_with("UPDATE ") {
            return Ok(serde_json::to_vec(&serde_json::json!({"rows_affected": 0})).unwrap());
        }

        // Extract table name (word after UPDATE)
        let table = req
            .query
            .split_whitespace()
            .nth(1)
            .unwrap_or("")
            .to_string();

        // Find the id argument: look for "WHERE id = ?N" or "id = ?N"
        // Also look for status condition
        let mut target_id = String::new();
        let mut required_status: Vec<String> = Vec::new();
        let mut set_fields: HashMap<String, serde_json::Value> = HashMap::new();

        // Parse SET clause fields: field = ?N or field = 'literal'
        if let Some(set_start) = req.query.find(" SET ").or_else(|| req.query.find(" set ")) {
            let after_set = &req.query[set_start + 5..];
            let where_pos = after_set
                .to_uppercase()
                .find(" WHERE ")
                .unwrap_or(after_set.len());
            let set_clause = &after_set[..where_pos];
            for assignment in set_clause.split(',') {
                let parts: Vec<&str> = assignment.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let field = parts[0].trim().to_string();
                    let val_str = parts[1].trim();
                    if let Some(param) = val_str.strip_prefix('?') {
                        if let Ok(idx) = param.parse::<usize>() {
                            if let Some(v) = req.args.get(idx - 1) {
                                set_fields.insert(field, v.clone());
                            }
                        }
                    } else if val_str.starts_with('\'') && val_str.ends_with('\'') {
                        set_fields.insert(
                            field,
                            serde_json::Value::String(val_str[1..val_str.len() - 1].to_string()),
                        );
                    }
                }
            }
        }

        // Parse WHERE clause for id and status conditions
        if let Some(where_start) = req.query.to_uppercase().find(" WHERE ") {
            let where_clause = &req.query[where_start + 7..];
            for condition in where_clause
                .split(" AND ")
                .chain(where_clause.split(" and "))
            {
                let cond = condition.trim();
                if cond.to_uppercase().starts_with("ID =") || cond.to_uppercase().starts_with("ID=")
                {
                    let val = cond.splitn(2, '=').nth(1).unwrap_or("").trim();
                    if let Some(param) = val.strip_prefix('?') {
                        if let Ok(idx) = param.parse::<usize>() {
                            if let Some(v) = req.args.get(idx - 1) {
                                target_id = v.as_str().unwrap_or("").to_string();
                            }
                        }
                    }
                }
                if cond.to_uppercase().contains("STATUS IN")
                    || cond.to_uppercase().contains("STATUS =")
                {
                    // Extract status values from IN clause or = clause
                    if let Some(in_start) = cond.find('(') {
                        let in_end = cond.find(')').unwrap_or(cond.len());
                        let statuses = &cond[in_start + 1..in_end];
                        for s in statuses.split(',') {
                            let s = s.trim().trim_matches('\'');
                            if !s.is_empty() {
                                required_status.push(s.to_string());
                            }
                        }
                    } else if cond.to_uppercase().contains("STATUS =") {
                        let val = cond.rsplit('=').next().unwrap_or("").trim();
                        if let Some(param) = val.strip_prefix('?') {
                            if let Ok(idx) = param.parse::<usize>() {
                                if let Some(v) = req.args.get(idx - 1) {
                                    required_status.push(v.as_str().unwrap_or("").to_string());
                                }
                            }
                        } else {
                            let s = val.trim_matches('\'');
                            if !s.is_empty() {
                                required_status.push(s.to_string());
                            }
                        }
                    }
                }
            }
        }

        let mut db = self.db.lock().unwrap();
        let records = match db.get_mut(&table) {
            Some(r) => r,
            None => {
                return Ok(serde_json::to_vec(&serde_json::json!({"rows_affected": 0})).unwrap())
            }
        };

        let mut rows_affected = 0i64;
        for record in records.iter_mut() {
            let id_matches = target_id.is_empty() || record.id == target_id;
            let status_matches = required_status.is_empty() || {
                let current = record
                    .data
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                required_status.iter().any(|s| s == current)
            };
            if id_matches && status_matches {
                for (k, v) in &set_fields {
                    record.data.insert(k.clone(), v.clone());
                }
                rows_affected += 1;
            }
        }

        Ok(serde_json::to_vec(&serde_json::json!({"rows_affected": rows_affected})).unwrap())
    }
}

// --- Filter matching ---

#[derive(serde::Deserialize)]
struct FilterDef {
    field: String,
    operator: String,
    value: serde_json::Value,
}

fn matches_filter(record: &Record, filter: &FilterDef) -> bool {
    let val = record.data.get(&filter.field);
    match filter.operator.as_str() {
        "eq" => val.map_or(false, |v| v == &filter.value),
        "neq" => val.map_or(true, |v| v != &filter.value),
        "like" => {
            let pattern = filter.value.as_str().unwrap_or("");
            let text = val.and_then(|v| v.as_str()).unwrap_or("");
            if pattern.starts_with('%') && pattern.ends_with('%') {
                text.contains(&pattern[1..pattern.len() - 1])
            } else if pattern.starts_with('%') {
                text.ends_with(&pattern[1..])
            } else if pattern.ends_with('%') {
                text.starts_with(&pattern[..pattern.len() - 1])
            } else {
                text == pattern
            }
        }
        "gt" => compare_values(val, &filter.value) == std::cmp::Ordering::Greater,
        "gte" => matches!(
            compare_values(val, &filter.value),
            std::cmp::Ordering::Greater | std::cmp::Ordering::Equal
        ),
        "lt" => compare_values(val, &filter.value) == std::cmp::Ordering::Less,
        "lte" => matches!(
            compare_values(val, &filter.value),
            std::cmp::Ordering::Less | std::cmp::Ordering::Equal
        ),
        _ => true,
    }
}

fn compare_values(a: Option<&serde_json::Value>, b: &serde_json::Value) -> std::cmp::Ordering {
    match (a.and_then(|v| v.as_f64()), b.as_f64()) {
        (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
        _ => std::cmp::Ordering::Equal,
    }
}

fn compare_json_values(
    a: Option<&serde_json::Value>,
    b: Option<&serde_json::Value>,
) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => {
            if let (Some(a), Some(b)) = (a.as_str(), b.as_str()) {
                a.cmp(b)
            } else if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
                a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                std::cmp::Ordering::Equal
            }
        }
        _ => std::cmp::Ordering::Equal,
    }
}

// --- Context implementation ---

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Context for MockContext {
    async fn call_block(&self, block_name: &str, msg: &mut Message) -> Result_ {
        let kind = msg.kind.clone();
        let data = msg.data.clone();

        let result = match block_name {
            "wafer-run/database" => self.handle_db_call(&kind, &data),
            "wafer-run/config" => self.handle_config_call(&kind, &data),
            _ => Err(WaferError::new(
                "not_found",
                format!("block '{}' not found", block_name),
            )),
        };

        match result {
            Ok(response_data) => Result_ {
                action: Action::Respond,
                response: Some(Response {
                    data: response_data,
                    meta: Vec::new(),
                }),
                error: None,
                message: None,
            },
            Err(e) => Result_ {
                action: Action::Error,
                response: None,
                error: Some(e),
                message: None,
            },
        }
    }

    fn is_cancelled(&self) -> bool {
        false
    }
    fn config_get(&self, key: &str) -> Option<&str> {
        self.config.get(key).map(|s| s.as_str())
    }
}

// --- Test message builders ---

/// Build a request message with JSON body, action, path, and user_id.
pub fn request_msg(action: &str, path: &str, user_id: &str, body: serde_json::Value) -> Message {
    let data = serde_json::to_vec(&body).unwrap();
    let mut msg = Message::new("http.request", data);
    msg.set_meta("req.action", action);
    msg.set_meta("req.resource", path);
    if !user_id.is_empty() {
        msg.set_meta("auth.user_id", user_id);
    }
    msg
}

/// Build a GET request with optional query params.
pub fn get_msg(path: &str, user_id: &str) -> Message {
    request_msg("retrieve", path, user_id, serde_json::json!({}))
}

/// Build a POST/create request.
pub fn create_msg(path: &str, user_id: &str, body: serde_json::Value) -> Message {
    request_msg("create", path, user_id, body)
}

/// Build a PATCH/update request.
pub fn update_msg(path: &str, user_id: &str, body: serde_json::Value) -> Message {
    request_msg("update", path, user_id, body)
}

/// Build a DELETE request.
pub fn delete_msg(path: &str, user_id: &str) -> Message {
    request_msg("delete", path, user_id, serde_json::json!({}))
}

/// Build an admin GET request.
pub fn admin_get_msg(path: &str) -> Message {
    let mut msg = get_msg(path, "admin_1");
    msg.set_meta("auth.user_roles", "admin");
    msg
}

/// Build an admin create request.
pub fn admin_create_msg(path: &str, body: serde_json::Value) -> Message {
    let mut msg = create_msg(path, "admin_1", body);
    msg.set_meta("auth.user_roles", "admin");
    msg
}

/// Extract the JSON response body from a Result_.
pub fn response_json(result: &Result_) -> serde_json::Value {
    result
        .response
        .as_ref()
        .map(|r| serde_json::from_slice(&r.data).unwrap_or(serde_json::Value::Null))
        .unwrap_or(serde_json::Value::Null)
}

/// Check if a result is an error with a specific code.
pub fn is_error(result: &Result_, code: &str) -> bool {
    let code: ErrorCode = code.into();
    result.action == Action::Error && result.error.as_ref().map_or(false, |e| e.code == code)
}
