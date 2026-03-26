use std::collections::HashMap;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::{config, network};
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, SortField};
use super::PROJECTS_COLLECTION;
use crate::blocks::helpers::RecordExt;

/// Call the control plane API. Returns the parsed JSON response body on success.
async fn control_plane_request(
    ctx: &dyn Context,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
) -> Result<(u16, serde_json::Value), String> {
    let base_url = match config::get(ctx, "CONTROL_PLANE_URL").await {
        Ok(url) => url,
        Err(_) => return Err("CONTROL_PLANE_URL not configured".to_string()),
    };
    let secret = match config::get(ctx, "CONTROL_PLANE_SECRET").await {
        Ok(s) => s,
        Err(_) => return Err("CONTROL_PLANE_SECRET not configured".to_string()),
    };

    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("X-Admin-Secret".to_string(), secret);

    let resp = network::do_request(ctx, method, &url, &headers, body)
        .await
        .map_err(|e| format!("Control plane request failed: {e}"))?;

    let json: serde_json::Value = serde_json::from_slice(&resp.body)
        .unwrap_or_else(|_| serde_json::json!({"error": String::from_utf8_lossy(&resp.body).to_string()}));

    Ok((resp.status_code, json))
}

/// Update deployment status in the database.
async fn update_status(ctx: &dyn Context, id: &str, status: &str) {
    let mut data = HashMap::new();
    data.insert("status".to_string(), serde_json::Value::String(status.to_string()));
    data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    let _ = db::update(ctx, PROJECTS_COLLECTION, id, data).await;
}

pub async fn handle_admin(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/b/projects") => handle_admin_list(ctx, msg).await,
        ("retrieve", "/admin/b/projects/stats") => handle_admin_stats(ctx, msg).await,
        ("retrieve", _) if path.starts_with("/admin/b/projects/") => handle_admin_get(ctx, msg).await,
        ("update", _) if path.starts_with("/admin/b/projects/") => handle_admin_update(ctx, msg).await,
        _ => err_not_found(msg, "not found"),
    }
}

pub async fn handle_user(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/b/projects") => handle_list(ctx, msg).await,
        ("retrieve", _) if path.starts_with("/b/projects/") => handle_get(ctx, msg).await,
        ("create", "/b/projects") => handle_create(ctx, msg).await,
        ("update", _) if path.starts_with("/b/projects/") => handle_update(ctx, msg).await,
        ("delete", _) if path.starts_with("/b/projects/") => handle_delete(ctx, msg).await,
        _ => err_not_found(msg, "not found"),
    }
}

// --- User handlers ---

async fn handle_list(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return err_forbidden(msg, "Authentication required");
    }

    let (page, page_size, _) = msg.pagination_params(20);

    let filters = vec![Filter {
        field: "user_id".to_string(),
        operator: FilterOp::Equal,
        value: serde_json::Value::String(user_id),
    }];
    let sort = vec![SortField { field: "created_at".to_string(), desc: true }];

    match db::paginated_list(ctx, PROJECTS_COLLECTION, page as i64, page_size as i64, filters, sort).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_get(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return err_forbidden(msg, "Authentication required");
    }

    let path = msg.path();
    let id = path.strip_prefix("/b/projects/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, "Missing deployment ID");
    }

    match db::get(ctx, PROJECTS_COLLECTION, id).await {
        Ok(record) => {
            let owner = record.str_field("user_id");
            if owner != user_id {
                return err_not_found(msg, "Deployment not found");
            }
            json_respond(msg, &record)
        }
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Deployment not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_create(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return err_forbidden(msg, "Authentication required");
    }

    let body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };

    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    if name.is_empty() {
        return err_bad_request(msg, "Name is required");
    }
    if name.len() > 100 {
        return err_bad_request(msg, "Name must be 100 characters or fewer");
    }

    let slug = name.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "-");

    let now = chrono::Utc::now().to_rfc3339();

    let mut data = HashMap::new();
    data.insert("user_id".to_string(), serde_json::Value::String(user_id));
    data.insert("name".to_string(), serde_json::Value::String(name));
    data.insert("slug".to_string(), serde_json::Value::String(slug.clone()));
    data.insert("status".to_string(), serde_json::Value::String("inactive".to_string()));
    if let Some(config) = body.get("config") {
        data.insert("config".to_string(), config.clone());
    }
    if let Some(plan_id) = body.get("plan_id") {
        data.insert("plan_id".to_string(), plan_id.clone());
    }
    if let Some(purchase_id) = body.get("purchase_id") {
        data.insert("purchase_id".to_string(), purchase_id.clone());
    }
    data.insert("created_at".to_string(), serde_json::Value::String(now.clone()));
    data.insert("updated_at".to_string(), serde_json::Value::String(now));

    let record = match db::create(ctx, PROJECTS_COLLECTION, data).await {
        Ok(r) => r,
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    };

    let record_id = record.id.clone();

    // Provision tenant on the control plane (best-effort, non-blocking for the user response)
    let plan = body.get("plan_id").and_then(|v| v.as_str()).unwrap_or("hobby");
    let provision_body = serde_json::json!({
        "subdomain": slug,
        "plan": plan,
    });
    let provision_bytes = serde_json::to_vec(&provision_body).unwrap_or_default();

    match control_plane_request(ctx, "POST", "/_control/tenants", Some(&provision_bytes)).await {
        Ok((status_code, resp_json)) if status_code < 300 => {
            // Store the tenant ID from the control plane response
            let mut update_data = HashMap::new();
            update_data.insert("status".to_string(), serde_json::Value::String("active".to_string()));
            if let Some(tenant_id) = resp_json.get("id").and_then(|v| v.as_str()) {
                update_data.insert("tenant_id".to_string(), serde_json::Value::String(tenant_id.to_string()));
            }
            if let Some(subdomain) = resp_json.get("subdomain").and_then(|v| v.as_str()) {
                update_data.insert("subdomain".to_string(), serde_json::Value::String(subdomain.to_string()));
            }
            update_data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            match db::update(ctx, PROJECTS_COLLECTION, &record_id, update_data).await {
                Ok(updated) => json_respond(msg, &updated),
                Err(_) => json_respond(msg, &record),
            }
        }
        Ok((status_code, resp_json)) => {
            // Provisioning failed — update status to "failed" with error details
            let error_msg = resp_json.get("error").and_then(|v| v.as_str())
                .unwrap_or("Provisioning failed");
            let mut update_data = HashMap::new();
            update_data.insert("status".to_string(), serde_json::Value::String("failed".to_string()));
            update_data.insert("provision_error".to_string(), serde_json::Value::String(
                format!("HTTP {}: {}", status_code, error_msg)
            ));
            update_data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            let _ = db::update(ctx, PROJECTS_COLLECTION, &record_id, update_data).await;
            err_internal(msg, &format!("Provisioning failed: {error_msg}"))
        }
        Err(e) => {
            // Control plane unreachable — keep as pending, user can retry
            let mut update_data = HashMap::new();
            update_data.insert("provision_error".to_string(), serde_json::Value::String(e.clone()));
            update_data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            let _ = db::update(ctx, PROJECTS_COLLECTION, &record_id, update_data).await;
            // Still return the record — it's in "pending" status, admin can provision later
            json_respond(msg, &record)
        }
    }
}

async fn handle_update(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return err_forbidden(msg, "Authentication required");
    }

    let path = msg.path();
    let id = path.strip_prefix("/b/projects/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, "Missing deployment ID");
    }

    // Verify ownership
    match db::get(ctx, PROJECTS_COLLECTION, id).await {
        Ok(record) => {
            let owner = record.str_field("user_id");
            if owner != user_id {
                return err_not_found(msg, "Deployment not found");
            }
        }
        Err(e) if e.code == ErrorCode::NotFound => return err_not_found(msg, "Deployment not found"),
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    }

    let mut body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };

    // Users cannot change status directly
    body.remove("status");
    body.remove("user_id");

    body.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

    match db::update(ctx, PROJECTS_COLLECTION, id, body).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Deployment not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_delete(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return err_forbidden(msg, "Authentication required");
    }

    let path = msg.path();
    let id = path.strip_prefix("/b/projects/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, "Missing deployment ID");
    }

    // Verify ownership and get deployment details
    let record = match db::get(ctx, PROJECTS_COLLECTION, id).await {
        Ok(record) => {
            let owner = record.str_field("user_id");
            if owner != user_id {
                return err_not_found(msg, "Deployment not found");
            }
            record
        }
        Err(e) if e.code == ErrorCode::NotFound => return err_not_found(msg, "Deployment not found"),
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    };

    // Deprovision tenant on the control plane
    let subdomain = record.data.get("subdomain")
        .or_else(|| record.data.get("slug"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !subdomain.is_empty() {
        let path = format!("/_control/tenants/{}", subdomain);
        if let Err(e) = control_plane_request(ctx, "DELETE", &path, None).await {
            // Log but don't block deletion — admin can clean up orphaned tenants
            let mut err_data = HashMap::new();
            err_data.insert("deprovision_error".to_string(), serde_json::Value::String(e));
            err_data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            let _ = db::update(ctx, PROJECTS_COLLECTION, id, err_data).await;
        }
    }

    // Soft delete: set status="deleted" and deleted_at
    let now = chrono::Utc::now().to_rfc3339();
    let mut data = HashMap::new();
    data.insert("status".to_string(), serde_json::Value::String("deleted".to_string()));
    data.insert("deleted_at".to_string(), serde_json::Value::String(now.clone()));
    data.insert("updated_at".to_string(), serde_json::Value::String(now));

    match db::update(ctx, PROJECTS_COLLECTION, id, data).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

// --- Admin handlers ---

async fn handle_admin_list(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let (page, page_size, _) = msg.pagination_params(20);

    let mut filters = Vec::new();
    let user_id = msg.query("user_id").to_string();
    if !user_id.is_empty() {
        filters.push(Filter { field: "user_id".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(user_id) });
    }
    let status = msg.query("status").to_string();
    if !status.is_empty() {
        filters.push(Filter { field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String(status) });
    }

    let sort = vec![SortField { field: "created_at".to_string(), desc: true }];

    match db::paginated_list(ctx, PROJECTS_COLLECTION, page as i64, page_size as i64, filters, sort).await {
        Ok(result) => json_respond(msg, &result),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_admin_get(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/b/projects/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, "Missing deployment ID");
    }

    match db::get(ctx, PROJECTS_COLLECTION, id).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Deployment not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_admin_update(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let path = msg.path();
    let id = path.strip_prefix("/admin/b/projects/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request(msg, "Missing deployment ID");
    }

    let mut body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };

    // Handle admin actions that interact with the control plane
    if let Some(admin_action) = body.remove("action").and_then(|v| v.as_str().map(String::from)) {
        let record = match db::get(ctx, PROJECTS_COLLECTION, id).await {
            Ok(r) => r,
            Err(e) if e.code == ErrorCode::NotFound => return err_not_found(msg, "Deployment not found"),
            Err(e) => return err_internal(msg, &format!("Database error: {e}")),
        };

        let subdomain = record.data.get("subdomain")
            .or_else(|| record.data.get("slug"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match admin_action.as_str() {
            "provision" => {
                // (Re-)provision a pending/failed deployment
                let slug = record.str_field("slug");
                let plan = record.data.get("plan_id").and_then(|v| v.as_str()).unwrap_or("hobby");
                let provision_body = serde_json::json!({ "subdomain": slug, "plan": plan });
                let provision_bytes = serde_json::to_vec(&provision_body).unwrap_or_default();

                match control_plane_request(ctx, "POST", "/_control/tenants", Some(&provision_bytes)).await {
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
                        update_data.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                        match db::update(ctx, PROJECTS_COLLECTION, id, update_data).await {
                            Ok(updated) => return json_respond(msg, &updated),
                            Err(e) => return err_internal(msg, &format!("Database error: {e}")),
                        }
                    }
                    Ok((sc, resp)) => {
                        let error_msg = resp.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
                        update_status(ctx, id, "failed").await;
                        return err_internal(msg, &format!("Provisioning failed ({}): {}", sc, error_msg));
                    }
                    Err(e) => return err_internal(msg, &e),
                }
            }
            "deprovision" => {
                if !subdomain.is_empty() {
                    let cp_path = format!("/_control/tenants/{}", subdomain);
                    match control_plane_request(ctx, "DELETE", &cp_path, None).await {
                        Ok((sc, _)) if sc < 300 || sc == 404 => {
                            update_status(ctx, id, "deleted").await;
                        }
                        Ok((sc, resp)) => {
                            let error_msg = resp.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
                            return err_internal(msg, &format!("Deprovision failed ({}): {}", sc, error_msg));
                        }
                        Err(e) => return err_internal(msg, &e),
                    }
                }
                let now = chrono::Utc::now().to_rfc3339();
                let mut del_data = HashMap::new();
                del_data.insert("status".to_string(), serde_json::Value::String("deleted".to_string()));
                del_data.insert("deleted_at".to_string(), serde_json::Value::String(now.clone()));
                del_data.insert("updated_at".to_string(), serde_json::Value::String(now));
                match db::update(ctx, PROJECTS_COLLECTION, id, del_data).await {
                    Ok(updated) => return json_respond(msg, &updated),
                    Err(e) => return err_internal(msg, &format!("Database error: {e}")),
                }
            }
            _ => return err_bad_request(msg, &format!("Unknown action: {admin_action}")),
        }
    }

    body.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

    match db::update(ctx, PROJECTS_COLLECTION, id, body).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Deployment not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_admin_stats(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let total = db::count(ctx, PROJECTS_COLLECTION, &[]).await.unwrap_or(0);
    let pending = db::count(ctx, PROJECTS_COLLECTION, &[Filter {
        field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("pending".to_string()),
    }]).await.unwrap_or(0);
    let active = db::count(ctx, PROJECTS_COLLECTION, &[Filter {
        field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("active".to_string()),
    }]).await.unwrap_or(0);
    let stopped = db::count(ctx, PROJECTS_COLLECTION, &[Filter {
        field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("stopped".to_string()),
    }]).await.unwrap_or(0);
    let failed = db::count(ctx, PROJECTS_COLLECTION, &[Filter {
        field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("failed".to_string()),
    }]).await.unwrap_or(0);
    let deleted = db::count(ctx, PROJECTS_COLLECTION, &[Filter {
        field: "status".to_string(), operator: FilterOp::Equal, value: serde_json::Value::String("deleted".to_string()),
    }]).await.unwrap_or(0);

    json_respond(msg, &serde_json::json!({
        "total": total,
        "pending": pending,
        "active": active,
        "stopped": stopped,
        "failed": failed,
        "deleted": deleted
    }))
}
