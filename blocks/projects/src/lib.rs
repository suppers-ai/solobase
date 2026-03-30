wit_bindgen::generate!({
    world: "wafer-block",
    path: "../../../wafer-run/wit/wit",
    additional_derives: [serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash],
    export_macro_name: "export_block",
});

use exports::wafer::block_world::block::Guest;
use wafer::block_world::types::*;

// wafer-core clients (use WASM sync variants via WIT call-block import)
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, SortField};
use wafer_core::clients::{config, network};

mod helpers;
use helpers::*;

use std::collections::HashMap;

struct DeploymentsBlockWasm;

const DEPLOYMENTS_COLLECTION: &str = "block_deployments";

/// Reserved subdomains that cannot be used as project names.
const RESERVED_SUBDOMAINS: &[&str] = &[
    "admin", "api", "app", "auth", "billing", "blog", "cdn", "cloud",
    "console", "dashboard", "dev", "docs", "help", "internal", "login",
    "mail", "manage", "platform", "settings", "staging", "status",
    "support", "test", "www",
];

/// Validate a subdomain string.
fn validate_subdomain(name: &str) -> Result<(), String> {
    if name.len() < 3 {
        return Err("Subdomain must be at least 3 characters".to_string());
    }
    if name.len() > 63 {
        return Err("Subdomain must be 63 characters or fewer".to_string());
    }
    if !name.starts_with(|c: char| c.is_ascii_lowercase()) {
        return Err("Subdomain must start with a lowercase letter".to_string());
    }
    if name.ends_with('-') {
        return Err("Subdomain cannot end with a hyphen".to_string());
    }
    if name.contains("--") {
        return Err("Subdomain cannot contain consecutive hyphens".to_string());
    }
    if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err("Subdomain must only contain lowercase letters, numbers, and hyphens".to_string());
    }
    if RESERVED_SUBDOMAINS.contains(&name) {
        return Err(format!("Subdomain '{}' is reserved", name));
    }
    Ok(())
}

impl Guest for DeploymentsBlockWasm {
    fn info() -> BlockInfo {
        BlockInfo {
            name: "suppers-ai/deployments".to_string(),
            version: "1.0.0".to_string(),
            interface: "http-handler@v1".to_string(),
            summary: "Deployment management for users and admins".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: Vec::new(),
            collections: Vec::new(),
            config_schema: None,
        }
    }

    fn handle(msg: Message) -> BlockResult {
        let action = msg_get_meta(&msg, "req.action").to_string();
        let path = msg_get_meta(&msg, "req.resource").to_string();

        // Admin routes
        if path.starts_with("/admin/b/deployments") {
            return handle_admin(&msg, &action, &path);
        }

        // User-facing routes
        if path.starts_with("/b/deployments") {
            return handle_user(&msg, &action, &path);
        }

        err_not_found(&msg, "not found")
    }

    fn lifecycle(_event: LifecycleEvent) -> Result<(), WaferError> {
        Ok(())
    }
}

export_block!(DeploymentsBlockWasm);

// ---------------------------------------------------------------------------
// Route dispatch
// ---------------------------------------------------------------------------

fn handle_admin(msg: &Message, action: &str, path: &str) -> BlockResult {
    match (action, path) {
        ("retrieve", "/admin/b/deployments") => handle_admin_list(msg),
        ("retrieve", "/admin/b/deployments/stats") => handle_admin_stats(msg),
        ("retrieve", _) if path.starts_with("/admin/b/deployments/") => handle_admin_get(msg, path),
        ("update", _) if path.starts_with("/admin/b/deployments/") => handle_admin_update(msg, path),
        _ => err_not_found(msg, "not found"),
    }
}

fn handle_user(msg: &Message, action: &str, path: &str) -> BlockResult {
    match (action, path) {
        ("retrieve", "/b/deployments") => handle_list(msg),
        ("retrieve", _) if path.starts_with("/b/deployments/") => handle_get(msg, path),
        ("create", "/b/deployments") => handle_create(msg),
        ("update", _) if path.starts_with("/b/deployments/") => handle_update(msg, path),
        ("delete", _) if path.starts_with("/b/deployments/") => handle_delete(msg, path),
        _ => err_not_found(msg, "not found"),
    }
}

// ---------------------------------------------------------------------------
// Control plane helper
// ---------------------------------------------------------------------------

fn control_plane_request(
    method: &str,
    cp_path: &str,
    body: Option<&[u8]>,
) -> Result<(u16, serde_json::Value), String> {
    let base_url = config::get("CONTROL_PLANE_URL")
        .map_err(|_| "CONTROL_PLANE_URL not configured".to_string())?;
    let secret = config::get("CONTROL_PLANE_SECRET")
        .map_err(|_| "CONTROL_PLANE_SECRET not configured".to_string())?;

    let url = format!("{}{}", base_url.trim_end_matches('/'), cp_path);
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("X-Admin-Secret".to_string(), secret);

    let resp = network::do_request(method, &url, &headers, body)
        .map_err(|e| format!("Control plane request failed: {}", e.message))?;

    let json: serde_json::Value = serde_json::from_slice(&resp.body)
        .unwrap_or_else(|_| serde_json::json!({"error": String::from_utf8_lossy(&resp.body).to_string()}));

    Ok((resp.status_code, json))
}

/// Update deployment status in the database.
fn update_status(id: &str, status: &str) {
    let mut data = HashMap::new();
    data.insert("status".to_string(), serde_json::Value::String(status.to_string()));
    data.insert("updated_at".to_string(), serde_json::Value::String(now_rfc3339()));
    let _ = db::update(DEPLOYMENTS_COLLECTION, id, data);
}

// ---------------------------------------------------------------------------
// User handlers
// ---------------------------------------------------------------------------

fn handle_list(msg: &Message) -> BlockResult {
    let user_id = msg_get_meta(msg, "auth.user_id").to_string();
    if user_id.is_empty() {
        return err_forbidden(msg, "Authentication required");
    }

    let (page, page_size) = pagination_params(msg, 20);

    let filters = vec![Filter {
        field: "user_id".to_string(),
        operator: FilterOp::Equal,
        value: serde_json::Value::String(user_id.clone()),
    }];
    let sort = vec![SortField { field: "created_at".to_string(), desc: true }];

    match paginated_list(DEPLOYMENTS_COLLECTION, page as i64, page_size as i64, filters, sort) {
        Ok(result) => {
            // Enrich each record with can_activate flag based on user's plan
            let cap = get_activation_capacity(DEPLOYMENTS_COLLECTION, &user_id);
            let has_room = cap.active_count < cap.max_active;

            let mut json_val = serde_json::to_value(&result).unwrap_or_default();
            if let Some(records) = json_val.get_mut("records").and_then(|v| v.as_array_mut()) {
                for record in records.iter_mut() {
                    let status = record.get("data")
                        .and_then(|d| d.get("status"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("");
                    let can_activate = status == "inactive" && has_room;
                    record.as_object_mut().map(|obj| {
                        obj.insert("can_activate".to_string(), serde_json::Value::Bool(can_activate));
                    });
                }
            }
            // Include plan info in the response
            json_val.as_object_mut().map(|obj| {
                obj.insert("plan".to_string(), serde_json::Value::String(cap.plan));
            });

            json_respond(msg, &json_val)
        }
        Err(e) => err_internal(msg, &format!("Database error: {}", e.message)),
    }
}

fn handle_get(msg: &Message, path: &str) -> BlockResult {
    let user_id = msg_get_meta(msg, "auth.user_id").to_string();
    if user_id.is_empty() {
        return err_forbidden(msg, "Authentication required");
    }

    let id = path.strip_prefix("/b/deployments/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, "Missing deployment ID");
    }

    match db::get(DEPLOYMENTS_COLLECTION, id) {
        Ok(record) => {
            let owner = str_field(&record, "user_id");
            if owner != user_id {
                return err_not_found(msg, "Deployment not found");
            }
            json_respond(msg, &serde_json::to_value(&record).unwrap_or_default())
        }
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Deployment not found"),
        Err(e) => err_internal(msg, &format!("Database error: {}", e.message)),
    }
}

fn handle_create(msg: &Message) -> BlockResult {
    let user_id = msg_get_meta(msg, "auth.user_id").to_string();
    if user_id.is_empty() {
        return err_forbidden(msg, "Authentication required");
    }

    let body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_lowercase();
    if name.is_empty() {
        return err_bad_request(msg, "Subdomain is required");
    }

    // Validate subdomain format
    if let Err(e) = validate_subdomain(&name) {
        return err_bad_request(msg, &e);
    }

    let slug = name.clone();

    // Check if subdomain is already taken (non-deleted projects)
    let slug_filters = vec![
        Filter {
            field: "slug".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(slug.clone()),
        },
        Filter {
            field: "status".to_string(),
            operator: FilterOp::NotEqual,
            value: serde_json::Value::String("deleted".to_string()),
        },
    ];
    match db::count(DEPLOYMENTS_COLLECTION, &slug_filters) {
        Ok(count) if count > 0 => {
            return err_conflict(msg, &format!("Subdomain '{}' is already taken", slug));
        }
        Err(e) => {
            return err_internal(msg, &format!("Database error: {}", e.message));
        }
        _ => {}
    }

    let now = now_rfc3339();

    let mut data = HashMap::new();
    data.insert("user_id".to_string(), serde_json::Value::String(user_id.clone()));
    data.insert("name".to_string(), serde_json::Value::String(name));
    data.insert("slug".to_string(), serde_json::Value::String(slug.clone()));
    data.insert("status".to_string(), serde_json::Value::String("inactive".to_string()));
    if let Some(cfg) = body.get("config") {
        data.insert("config".to_string(), cfg.clone());
    }
    if let Some(plan_id) = body.get("plan_id") {
        data.insert("plan_id".to_string(), plan_id.clone());
    }
    if let Some(purchase_id) = body.get("purchase_id") {
        data.insert("purchase_id".to_string(), purchase_id.clone());
    }
    data.insert("created_at".to_string(), serde_json::Value::String(now.clone()));
    data.insert("updated_at".to_string(), serde_json::Value::String(now));

    let record = match db::create(DEPLOYMENTS_COLLECTION, data) {
        Ok(r) => r,
        Err(e) => return err_internal(msg, &format!("Database error: {}", e.message)),
    };

    let record_id = record.id.clone();

    // Check whether this deployment should be auto-activated based on the user's plan
    let activated = activate_if_allowed(DEPLOYMENTS_COLLECTION, &user_id, &record_id);

    if !activated {
        // Plan does not allow more active deployments — return as inactive
        // Re-fetch to get the final state
        return match db::get(DEPLOYMENTS_COLLECTION, &record_id) {
            Ok(r) => json_respond(msg, &serde_json::to_value(&r).unwrap_or_default()),
            Err(_) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        };
    }

    // Deployment was activated — provision tenant on the control plane
    let plan = body.get("plan_id").and_then(|v| v.as_str()).unwrap_or("hobby");
    let provision_body = serde_json::json!({
        "subdomain": slug,
        "plan": plan,
    });
    let provision_bytes = serde_json::to_vec(&provision_body).unwrap_or_default();

    match control_plane_request("POST", "/_control/projects", Some(&provision_bytes)) {
        Ok((status_code, resp_json)) if status_code < 300 => {
            // Store the tenant ID from the control plane response
            let mut update_data = HashMap::new();
            if let Some(tenant_id) = resp_json.get("id").and_then(|v| v.as_str()) {
                update_data.insert("tenant_id".to_string(), serde_json::Value::String(tenant_id.to_string()));
            }
            if let Some(subdomain) = resp_json.get("subdomain").and_then(|v| v.as_str()) {
                update_data.insert("subdomain".to_string(), serde_json::Value::String(subdomain.to_string()));
            }
            update_data.insert("updated_at".to_string(), serde_json::Value::String(now_rfc3339()));
            match db::update(DEPLOYMENTS_COLLECTION, &record_id, update_data) {
                Ok(updated) => json_respond(msg, &serde_json::to_value(&updated).unwrap_or_default()),
                Err(_) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
            }
        }
        Ok((status_code, resp_json)) => {
            // Provisioning failed -- update status to "failed" with error details
            let error_msg = resp_json.get("error").and_then(|v| v.as_str())
                .unwrap_or("Provisioning failed");
            let mut update_data = HashMap::new();
            update_data.insert("status".to_string(), serde_json::Value::String("failed".to_string()));
            update_data.insert("provision_error".to_string(), serde_json::Value::String(
                format!("HTTP {}: {}", status_code, error_msg)
            ));
            update_data.insert("updated_at".to_string(), serde_json::Value::String(now_rfc3339()));
            let _ = db::update(DEPLOYMENTS_COLLECTION, &record_id, update_data);
            err_internal(msg, &format!("Provisioning failed: {error_msg}"))
        }
        Err(e) => {
            // Control plane unreachable -- keep as active, admin can provision later
            let mut update_data = HashMap::new();
            update_data.insert("provision_error".to_string(), serde_json::Value::String(e));
            update_data.insert("updated_at".to_string(), serde_json::Value::String(now_rfc3339()));
            let _ = db::update(DEPLOYMENTS_COLLECTION, &record_id, update_data);
            // Re-fetch to return current state (active, but with provision_error)
            match db::get(DEPLOYMENTS_COLLECTION, &record_id) {
                Ok(r) => json_respond(msg, &serde_json::to_value(&r).unwrap_or_default()),
                Err(_) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
            }
        }
    }
}

fn handle_update(msg: &Message, path: &str) -> BlockResult {
    let user_id = msg_get_meta(msg, "auth.user_id").to_string();
    if user_id.is_empty() {
        return err_forbidden(msg, "Authentication required");
    }

    let id = path.strip_prefix("/b/deployments/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, "Missing deployment ID");
    }

    // Verify ownership
    match db::get(DEPLOYMENTS_COLLECTION, id) {
        Ok(record) => {
            let owner = str_field(&record, "user_id");
            if owner != user_id {
                return err_not_found(msg, "Deployment not found");
            }
        }
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => return err_not_found(msg, "Deployment not found"),
        Err(e) => return err_internal(msg, &format!("Database error: {}", e.message)),
    }

    let mut body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    // Users cannot change status directly
    body.remove("status");
    body.remove("user_id");

    stamp_updated(&mut body);

    match db::update(DEPLOYMENTS_COLLECTION, id, body) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Deployment not found"),
        Err(e) => err_internal(msg, &format!("Database error: {}", e.message)),
    }
}

fn handle_delete(msg: &Message, path: &str) -> BlockResult {
    let user_id = msg_get_meta(msg, "auth.user_id").to_string();
    if user_id.is_empty() {
        return err_forbidden(msg, "Authentication required");
    }

    let id = path.strip_prefix("/b/deployments/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, "Missing deployment ID");
    }

    // Verify ownership and get deployment details
    let record = match db::get(DEPLOYMENTS_COLLECTION, id) {
        Ok(record) => {
            let owner = str_field(&record, "user_id");
            if owner != user_id {
                return err_not_found(msg, "Deployment not found");
            }
            record
        }
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => return err_not_found(msg, "Deployment not found"),
        Err(e) => return err_internal(msg, &format!("Database error: {}", e.message)),
    };

    // Deprovision tenant on the control plane
    let subdomain = record.data.get("subdomain")
        .or_else(|| record.data.get("slug"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !subdomain.is_empty() {
        let cp_path = format!("/_control/projects/{}", subdomain);
        if let Err(e) = control_plane_request("DELETE", &cp_path, None) {
            // Log but don't block deletion -- admin can clean up orphaned tenants
            let mut err_data = HashMap::new();
            err_data.insert("deprovision_error".to_string(), serde_json::Value::String(e));
            err_data.insert("updated_at".to_string(), serde_json::Value::String(now_rfc3339()));
            let _ = db::update(DEPLOYMENTS_COLLECTION, id, err_data);
        }
    }

    // Soft delete: set status="deleted" and deleted_at
    let now = now_rfc3339();
    let mut data = HashMap::new();
    data.insert("status".to_string(), serde_json::Value::String("deleted".to_string()));
    data.insert("deleted_at".to_string(), serde_json::Value::String(now.clone()));
    data.insert("updated_at".to_string(), serde_json::Value::String(now));

    match db::update(DEPLOYMENTS_COLLECTION, id, data) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {}", e.message)),
    }
}

// ---------------------------------------------------------------------------
// Admin handlers
// ---------------------------------------------------------------------------

fn handle_admin_list(msg: &Message) -> BlockResult {
    let (page, page_size) = pagination_params(msg, 20);

    let mut filters = Vec::new();
    let user_id = msg_get_query(msg, "user_id").to_string();
    if !user_id.is_empty() {
        filters.push(Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id),
        });
    }
    let status = msg_get_query(msg, "status").to_string();
    if !status.is_empty() {
        filters.push(Filter {
            field: "status".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(status),
        });
    }

    let sort = vec![SortField { field: "created_at".to_string(), desc: true }];

    match paginated_list(DEPLOYMENTS_COLLECTION, page as i64, page_size as i64, filters, sort) {
        Ok(result) => json_respond(msg, &serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => err_internal(msg, &format!("Database error: {}", e.message)),
    }
}

fn handle_admin_get(msg: &Message, path: &str) -> BlockResult {
    let id = path.strip_prefix("/admin/b/deployments/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, "Missing deployment ID");
    }

    match db::get(DEPLOYMENTS_COLLECTION, id) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Deployment not found"),
        Err(e) => err_internal(msg, &format!("Database error: {}", e.message)),
    }
}

fn handle_admin_update(msg: &Message, path: &str) -> BlockResult {
    let id = path.strip_prefix("/admin/b/deployments/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, "Missing deployment ID");
    }

    let mut body: HashMap<String, serde_json::Value> = match decode_body(msg) {
        Ok(b) => b,
        Err(r) => return r,
    };

    // Handle admin actions that interact with the control plane
    if let Some(admin_action) = body.remove("action").and_then(|v| v.as_str().map(String::from)) {
        let record = match db::get(DEPLOYMENTS_COLLECTION, id) {
            Ok(r) => r,
            Err(e) if e.code == wafer_block::ErrorCode::NotFound => return err_not_found(msg, "Deployment not found"),
            Err(e) => return err_internal(msg, &format!("Database error: {}", e.message)),
        };

        let subdomain = record.data.get("subdomain")
            .or_else(|| record.data.get("slug"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match admin_action.as_str() {
            "provision" => {
                // (Re-)provision a pending/failed deployment
                let slug = str_field(&record, "slug");
                let plan = record.data.get("plan_id").and_then(|v| v.as_str()).unwrap_or("hobby");
                let provision_body = serde_json::json!({ "subdomain": slug, "plan": plan });
                let provision_bytes = serde_json::to_vec(&provision_body).unwrap_or_default();

                match control_plane_request("POST", "/_control/projects", Some(&provision_bytes)) {
                    Ok((sc, resp)) if sc < 300 => {
                        let mut update_data = HashMap::new();
                        update_data.insert("status".to_string(), serde_json::Value::String("active".to_string()));
                        if let Some(tid) = resp.get("id").and_then(|v| v.as_str()) {
                            update_data.insert("tenant_id".to_string(), serde_json::Value::String(tid.to_string()));
                        }
                        if let Some(sub) = resp.get("subdomain").and_then(|v| v.as_str()) {
                            update_data.insert("subdomain".to_string(), serde_json::Value::String(sub.to_string()));
                        }
                        update_data.insert("provision_error".to_string(), serde_json::Value::Null);
                        update_data.insert("updated_at".to_string(), serde_json::Value::String(now_rfc3339()));
                        match db::update(DEPLOYMENTS_COLLECTION, id, update_data) {
                            Ok(updated) => return json_respond(msg, &serde_json::to_value(&updated).unwrap_or_default()),
                            Err(e) => return err_internal(msg, &format!("Database error: {}", e.message)),
                        }
                    }
                    Ok((sc, resp)) => {
                        let error_msg = resp.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
                        update_status(id, "failed");
                        return err_internal(msg, &format!("Provisioning failed ({}): {}", sc, error_msg));
                    }
                    Err(e) => return err_internal(msg, &e),
                }
            }
            "deprovision" => {
                if !subdomain.is_empty() {
                    let cp_path = format!("/_control/projects/{}", subdomain);
                    match control_plane_request("DELETE", &cp_path, None) {
                        Ok((sc, _)) if sc < 300 || sc == 404 => {
                            update_status(id, "deleted");
                        }
                        Ok((sc, resp)) => {
                            let error_msg = resp.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
                            return err_internal(msg, &format!("Deprovision failed ({}): {}", sc, error_msg));
                        }
                        Err(e) => return err_internal(msg, &e),
                    }
                }
                let now = now_rfc3339();
                let mut del_data = HashMap::new();
                del_data.insert("status".to_string(), serde_json::Value::String("deleted".to_string()));
                del_data.insert("deleted_at".to_string(), serde_json::Value::String(now.clone()));
                del_data.insert("updated_at".to_string(), serde_json::Value::String(now));
                match db::update(DEPLOYMENTS_COLLECTION, id, del_data) {
                    Ok(updated) => return json_respond(msg, &serde_json::to_value(&updated).unwrap_or_default()),
                    Err(e) => return err_internal(msg, &format!("Database error: {}", e.message)),
                }
            }
            _ => return err_bad_request(msg, &format!("Unknown action: {admin_action}")),
        }
    }

    stamp_updated(&mut body);

    match db::update(DEPLOYMENTS_COLLECTION, id, body) {
        Ok(record) => json_respond(msg, &serde_json::to_value(&record).unwrap_or_default()),
        Err(e) if e.code == wafer_block::ErrorCode::NotFound => err_not_found(msg, "Deployment not found"),
        Err(e) => err_internal(msg, &format!("Database error: {}", e.message)),
    }
}

fn handle_admin_stats(msg: &Message) -> BlockResult {
    let total = db::count(DEPLOYMENTS_COLLECTION, &[]).unwrap_or(0);
    let pending = db::count(DEPLOYMENTS_COLLECTION, &[Filter {
        field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("pending".to_string()),
    }]).unwrap_or(0);
    let active = db::count(DEPLOYMENTS_COLLECTION, &[Filter {
        field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("active".to_string()),
    }]).unwrap_or(0);
    let stopped = db::count(DEPLOYMENTS_COLLECTION, &[Filter {
        field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("stopped".to_string()),
    }]).unwrap_or(0);
    let failed = db::count(DEPLOYMENTS_COLLECTION, &[Filter {
        field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("failed".to_string()),
    }]).unwrap_or(0);
    let deleted = db::count(DEPLOYMENTS_COLLECTION, &[Filter {
        field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("deleted".to_string()),
    }]).unwrap_or(0);

    json_respond(msg, &serde_json::json!({
        "total": total,
        "pending": pending,
        "active": active,
        "stopped": stopped,
        "failed": failed,
        "deleted": deleted
    }))
}
