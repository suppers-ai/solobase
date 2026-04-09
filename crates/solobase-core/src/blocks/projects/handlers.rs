use super::PROJECTS_COLLECTION;
use crate::blocks::helpers::RecordExt;
use std::collections::HashMap;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, Record, SortField};
use wafer_core::clients::{config, network};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

use crate::plans;

/// User's plan and addon project slots.
struct UserPlan {
    plan: String,
    addon_projects: usize,
}

/// Look up the user's current plan and addon project count.
/// Falls back to free with 0 addons when no active subscription exists.
async fn get_user_plan(ctx: &dyn Context, user_id: &str) -> UserPlan {
    let rows = db::query_raw(
        ctx,
        "SELECT plan, COALESCE(addon_projects, 0) as addon_projects FROM subscriptions WHERE user_id = ?1 AND status = 'active'",
        &[serde_json::Value::String(user_id.to_string())],
    ).await;

    match rows {
        Ok(records) if !records.is_empty() => {
            let plan = records[0]
                .data
                .get("plan")
                .and_then(|v| v.as_str())
                .unwrap_or("free")
                .to_string();
            let addon_projects = records[0]
                .data
                .get("addon_projects")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as usize;
            UserPlan {
                plan,
                addon_projects,
            }
        }
        _ => UserPlan {
            plan: "free".to_string(),
            addon_projects: 0,
        },
    }
}

/// Count the user's live projects — pending, active, or inactive (for create limit).
/// Excludes "deleted" and "failed" statuses.
async fn count_live_projects(ctx: &dyn Context, user_id: &str) -> i64 {
    // Count pending + active + inactive (exclude deleted and failed)
    let mut total = 0i64;
    for status in &["pending", "active", "inactive"] {
        let filters = vec![
            Filter {
                field: "user_id".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String(user_id.to_string()),
            },
            Filter {
                field: "status".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String(status.to_string()),
            },
        ];
        total += db::count(ctx, PROJECTS_COLLECTION, &filters)
            .await
            .unwrap_or(0);
    }
    total
}

/// Count the user's active projects (for activate limit).
async fn count_active_projects(ctx: &dyn Context, user_id: &str) -> i64 {
    let filters = vec![
        Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.to_string()),
        },
        Filter {
            field: "status".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String("active".to_string()),
        },
    ];
    db::count(ctx, PROJECTS_COLLECTION, &filters)
        .await
        .unwrap_or(0)
}

/// Reserved subdomains that cannot be used as project names.
const RESERVED_SUBDOMAINS: &[&str] = &[
    "admin",
    "api",
    "app",
    "auth",
    "billing",
    "blog",
    "cdn",
    "cloud",
    "console",
    "dashboard",
    "dev",
    "docs",
    "help",
    "internal",
    "login",
    "mail",
    "manage",
    "platform",
    "settings",
    "staging",
    "status",
    "support",
    "test",
    "www",
];

/// Validate a subdomain string.
/// Rules: only lowercase letters, numbers, and hyphens. Must start with a letter.
/// Min 3, max 63 chars. No consecutive hyphens. Cannot end with a hyphen.
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
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(
            "Subdomain must only contain lowercase letters, numbers, and hyphens".to_string(),
        );
    }
    if RESERVED_SUBDOMAINS.contains(&name) {
        return Err(format!("Subdomain '{}' is reserved", name));
    }
    Ok(())
}

/// Call the control plane API. Returns the parsed JSON response body on success.
///
/// When a `DISPATCHER` service binding is available (indicated by `HAS_DISPATCHER_BINDING`
/// config), requests are routed via the `solobase/dispatcher` block (internal RPC) instead
/// of making an external HTTP request. This avoids 522 errors from loopback requests.
async fn control_plane_request(
    ctx: &dyn Context,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
) -> Result<(u16, serde_json::Value), String> {
    let secret = match config::get(ctx, "SUPPERS_AI__PROJECTS__CONTROL_PLANE_SECRET").await {
        Ok(s) => s,
        Err(_) => return Err("CONTROL_PLANE_SECRET not configured".to_string()),
    };

    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("X-Control-Api-Key".to_string(), secret);

    // Prefer the dispatcher service binding (internal RPC) over external HTTP.
    let use_dispatcher = config::get(ctx, "SOLOBASE_SHARED__HAS_DISPATCHER_BINDING")
        .await
        .map(|v| v == "true")
        .unwrap_or(false);

    let resp = if use_dispatcher {
        // Route via the solobase/dispatcher block — same message format as network.do.
        // The URL is relative to the dispatcher worker, so we use a placeholder origin.
        let url = format!("https://internal{}", path);
        network::do_request_via(ctx, "solobase/dispatcher", method, &url, &headers, body)
            .await
            .map_err(|e| format!("Dispatcher request failed: {e}"))?
    } else {
        let base_url = match config::get(ctx, "SUPPERS_AI__PROJECTS__CONTROL_PLANE_URL").await {
            Ok(url) => url,
            Err(_) => return Err("CONTROL_PLANE_URL not configured".to_string()),
        };
        let url = format!("{}{}", base_url.trim_end_matches('/'), path);
        network::do_request(ctx, method, &url, &headers, body)
            .await
            .map_err(|e| format!("Control plane request failed: {e}"))?
    };

    let json: serde_json::Value = serde_json::from_slice(&resp.body).unwrap_or_else(
        |_| serde_json::json!({"error": String::from_utf8_lossy(&resp.body).to_string()}),
    );

    Ok((resp.status_code, json))
}

/// Update deployment status in the database.
async fn update_status(ctx: &dyn Context, id: &str, status: &str) {
    let mut data = HashMap::new();
    data.insert(
        "status".to_string(),
        serde_json::Value::String(status.to_string()),
    );
    data.insert(
        "updated_at".to_string(),
        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
    );
    let _ = db::update(ctx, PROJECTS_COLLECTION, id, data).await;
}

pub async fn handle_admin(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/b/projects") => handle_admin_list(ctx, msg).await,
        ("retrieve", "/admin/b/projects/stats") => handle_admin_stats(ctx, msg).await,
        ("retrieve", _) if path.starts_with("/admin/b/projects/") => {
            handle_admin_get(ctx, msg).await
        }
        ("update", _) if path.starts_with("/admin/b/projects/") => {
            handle_admin_update(ctx, msg).await
        }
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
    let sort = vec![SortField {
        field: "created_at".to_string(),
        desc: true,
    }];

    match db::paginated_list(
        ctx,
        PROJECTS_COLLECTION,
        page as i64,
        page_size as i64,
        filters,
        sort,
    )
    .await
    {
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

    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_lowercase();
    if name.is_empty() {
        return err_bad_request(msg, "Subdomain is required");
    }

    // Validate subdomain format
    if let Err(e) = validate_subdomain(&name) {
        return err_bad_request(msg, &e);
    }

    let slug = name.clone();

    // Check if subdomain is already taken (only live projects — pending, active, inactive)
    let mut taken = false;
    for status in &["pending", "active", "inactive"] {
        let slug_filters = vec![
            Filter {
                field: "slug".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String(slug.clone()),
            },
            Filter {
                field: "status".to_string(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String(status.to_string()),
            },
        ];
        if db::count(ctx, PROJECTS_COLLECTION, &slug_filters)
            .await
            .unwrap_or(0)
            > 0
        {
            taken = true;
            break;
        }
    }
    let _slug_filters: Vec<Filter> = vec![];
    if taken {
        return err_conflict(msg, &format!("Subdomain '{}' is already taken", slug));
    }

    // Check plan limit for project creation (count non-deleted projects)
    let user_plan = get_user_plan(ctx, &user_id).await;
    let limits = plans::get_limits(&user_plan.plan);
    let max_created = limits
        .max_projects_created
        .saturating_add(user_plan.addon_projects);
    let current_count = count_live_projects(ctx, &user_id).await;
    if current_count as usize >= max_created {
        return err_bad_request(
            msg,
            "Project limit reached. Upgrade your plan or purchase extra project slots.",
        );
    }

    let now = chrono::Utc::now().to_rfc3339();

    let mut data = HashMap::new();
    data.insert("user_id".to_string(), serde_json::Value::String(user_id));
    data.insert("name".to_string(), serde_json::Value::String(name));
    data.insert("slug".to_string(), serde_json::Value::String(slug.clone()));
    data.insert(
        "status".to_string(),
        serde_json::Value::String("pending".to_string()),
    );
    if let Some(config) = body.get("config") {
        data.insert("config".to_string(), config.clone());
    }
    if let Some(plan_id) = body.get("plan_id") {
        data.insert("plan_id".to_string(), plan_id.clone());
    }
    if let Some(purchase_id) = body.get("purchase_id") {
        data.insert("purchase_id".to_string(), purchase_id.clone());
    }
    data.insert(
        "created_at".to_string(),
        serde_json::Value::String(now.clone()),
    );
    data.insert("updated_at".to_string(), serde_json::Value::String(now));

    let record = match db::create(ctx, PROJECTS_COLLECTION, data).await {
        Ok(r) => r,
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    };

    json_respond(msg, &record)
}

async fn handle_update(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return err_forbidden(msg, "Authentication required");
    }

    let id = {
        let path = msg.path();
        let raw = path.strip_prefix("/b/projects/").unwrap_or("");
        if raw.is_empty() {
            return err_bad_request(msg, "Missing deployment ID");
        }
        raw.to_string()
    };

    // Verify ownership and get current record
    let record = match db::get(ctx, PROJECTS_COLLECTION, &id).await {
        Ok(record) => {
            let owner = record.str_field("user_id");
            if owner != user_id {
                return err_not_found(msg, "Deployment not found");
            }
            record
        }
        Err(e) if e.code == ErrorCode::NotFound => {
            return err_not_found(msg, "Deployment not found")
        }
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    };

    let mut body: HashMap<String, serde_json::Value> = match msg.decode() {
        Ok(b) => b,
        Err(e) => return err_bad_request(msg, &format!("Invalid body: {e}")),
    };

    // Handle lifecycle actions
    if let Some(action) = body
        .remove("action")
        .and_then(|v| v.as_str().map(String::from))
    {
        match action.as_str() {
            "activate" => return handle_activate(ctx, msg, &id, &user_id, &record).await,
            "deactivate" => return handle_deactivate(ctx, msg, &id, &record).await,
            _ => return err_bad_request(msg, &format!("Unknown action: {action}")),
        }
    }

    // Users cannot change status directly
    body.remove("status");
    body.remove("user_id");

    body.insert(
        "updated_at".to_string(),
        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
    );

    match db::update(ctx, PROJECTS_COLLECTION, &id, body).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Deployment not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

/// Activate a pending or inactive project: check plan capacity, provision via control plane.
async fn handle_activate(
    ctx: &dyn Context,
    msg: &mut Message,
    id: &str,
    user_id: &str,
    record: &Record,
) -> Result_ {
    let status = record.str_field("status");
    if status != "pending" && status != "inactive" {
        return err_bad_request(
            msg,
            &format!("Project cannot be activated from '{}' status", status),
        );
    }

    // Check plan capacity for active projects
    let user_plan = get_user_plan(ctx, user_id).await;
    let limits = plans::get_limits(&user_plan.plan);
    let max_active = limits
        .max_projects_active
        .saturating_add(user_plan.addon_projects);

    if max_active == 0 {
        return err_forbidden(msg, "Upgrade to a paid plan to activate projects");
    }

    let active_count = count_active_projects(ctx, user_id).await;
    if active_count as usize >= max_active {
        return err_bad_request(
            msg,
            "Active project limit reached. Deactivate a project or upgrade.",
        );
    }

    // Provision via control plane
    let slug = record.str_field("slug");
    let plan_id = record
        .data
        .get("plan_id")
        .and_then(|v: &serde_json::Value| v.as_str())
        .unwrap_or("free");
    let provision_body = serde_json::json!({
        "subdomain": slug,
        "plan": plan_id,
    });
    let provision_bytes = serde_json::to_vec(&provision_body).unwrap_or_default();

    match control_plane_request(ctx, "POST", "/_control/projects", Some(&provision_bytes)).await {
        Ok((status_code, resp_json)) if status_code < 300 => {
            let mut update_data = HashMap::new();
            update_data.insert(
                "status".to_string(),
                serde_json::Value::String("active".to_string()),
            );
            if let Some(tenant_id) = resp_json.get("id").and_then(|v| v.as_str()) {
                update_data.insert(
                    "tenant_id".to_string(),
                    serde_json::Value::String(tenant_id.to_string()),
                );
            }
            if let Some(subdomain) = resp_json.get("subdomain").and_then(|v| v.as_str()) {
                update_data.insert(
                    "subdomain".to_string(),
                    serde_json::Value::String(subdomain.to_string()),
                );
            }
            update_data.insert("provision_error".to_string(), serde_json::Value::Null);
            update_data.insert("grace_period_end".to_string(), serde_json::Value::Null);
            update_data.insert(
                "updated_at".to_string(),
                serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
            );

            match db::update(ctx, PROJECTS_COLLECTION, id, update_data).await {
                Ok(updated) => json_respond(msg, &updated),
                Err(e) => err_internal(msg, &format!("Database error: {e}")),
            }
        }
        Ok((status_code, resp_json)) => {
            let error_msg = resp_json
                .get("message")
                .and_then(|v| v.as_str())
                .or_else(|| resp_json.get("error").and_then(|v| v.as_str()))
                .unwrap_or("Provisioning failed");

            if status_code == 409 {
                return err_conflict(msg, error_msg);
            }

            // Update status to "failed"
            let mut update_data = HashMap::new();
            update_data.insert(
                "status".to_string(),
                serde_json::Value::String("failed".to_string()),
            );
            update_data.insert(
                "provision_error".to_string(),
                serde_json::Value::String(format!("HTTP {}: {}", status_code, error_msg)),
            );
            update_data.insert(
                "updated_at".to_string(),
                serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
            );
            let _ = db::update(ctx, PROJECTS_COLLECTION, id, update_data).await;
            err_internal(msg, &format!("Provisioning failed: {error_msg}"))
        }
        Err(e) => {
            let mut update_data = HashMap::new();
            update_data.insert(
                "provision_error".to_string(),
                serde_json::Value::String(e.clone()),
            );
            update_data.insert(
                "updated_at".to_string(),
                serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
            );
            let _ = db::update(ctx, PROJECTS_COLLECTION, id, update_data).await;
            err_internal(msg, &format!("Control plane error: {e}"))
        }
    }
}

/// Deactivate an active project: set status to inactive, set grace period.
/// Does NOT delete CF resources -- they stay alive during the 30-day grace period.
async fn handle_deactivate(
    ctx: &dyn Context,
    msg: &mut Message,
    id: &str,
    record: &Record,
) -> Result_ {
    let status = record.str_field("status");
    if status != "active" {
        return err_bad_request(
            msg,
            &format!(
                "Only active projects can be deactivated (current status: '{}')",
                status
            ),
        );
    }

    let now = chrono::Utc::now();
    let grace_end = now + chrono::Duration::days(30);
    let grace_end_str = grace_end.to_rfc3339();

    // Update local DB record
    let mut update_data = HashMap::new();
    update_data.insert(
        "status".to_string(),
        serde_json::Value::String("inactive".to_string()),
    );
    update_data.insert(
        "grace_period_end".to_string(),
        serde_json::Value::String(grace_end_str.clone()),
    );
    update_data.insert(
        "updated_at".to_string(),
        serde_json::Value::String(now.to_rfc3339()),
    );

    let result = match db::update(ctx, PROJECTS_COLLECTION, id, update_data).await {
        Ok(updated) => updated,
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    };

    // Notify control plane about the status change (best-effort)
    let slug = record.str_field("slug");
    if !slug.is_empty() {
        let cp_body = serde_json::json!({
            "status": "inactive",
            "grace_period_end": grace_end_str,
        });
        let cp_bytes = serde_json::to_vec(&cp_body).unwrap_or_default();
        let cp_path = format!("/_control/projects/{}", slug);
        let _ = control_plane_request(ctx, "PUT", &cp_path, Some(&cp_bytes)).await;
    }

    json_respond(msg, &result)
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
        Err(e) if e.code == ErrorCode::NotFound => {
            return err_not_found(msg, "Deployment not found")
        }
        Err(e) => return err_internal(msg, &format!("Database error: {e}")),
    };

    // Deprovision tenant on the control plane
    let subdomain = record
        .data
        .get("subdomain")
        .or_else(|| record.data.get("slug"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !subdomain.is_empty() {
        let path = format!("/_control/projects/{}", subdomain);
        if let Err(e) = control_plane_request(ctx, "DELETE", &path, None).await {
            // Log but don't block deletion — admin can clean up orphaned tenants
            let mut err_data = HashMap::new();
            err_data.insert(
                "deprovision_error".to_string(),
                serde_json::Value::String(e),
            );
            err_data.insert(
                "updated_at".to_string(),
                serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
            );
            let _ = db::update(ctx, PROJECTS_COLLECTION, id, err_data).await;
        }
    }

    // Soft delete: set status="deleted" and deleted_at
    let now = chrono::Utc::now().to_rfc3339();
    let mut data = HashMap::new();
    data.insert(
        "status".to_string(),
        serde_json::Value::String("deleted".to_string()),
    );
    data.insert(
        "deleted_at".to_string(),
        serde_json::Value::String(now.clone()),
    );
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
        filters.push(Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id),
        });
    }
    let status = msg.query("status").to_string();
    if !status.is_empty() {
        filters.push(Filter {
            field: "status".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(status),
        });
    }

    let sort = vec![SortField {
        field: "created_at".to_string(),
        desc: true,
    }];

    match db::paginated_list(
        ctx,
        PROJECTS_COLLECTION,
        page as i64,
        page_size as i64,
        filters,
        sort,
    )
    .await
    {
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
    if let Some(admin_action) = body
        .remove("action")
        .and_then(|v| v.as_str().map(String::from))
    {
        let record = match db::get(ctx, PROJECTS_COLLECTION, id).await {
            Ok(r) => r,
            Err(e) if e.code == ErrorCode::NotFound => {
                return err_not_found(msg, "Deployment not found")
            }
            Err(e) => return err_internal(msg, &format!("Database error: {e}")),
        };

        let subdomain = record
            .data
            .get("subdomain")
            .or_else(|| record.data.get("slug"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match admin_action.as_str() {
            "provision" => {
                // (Re-)provision a pending/failed deployment
                let slug = record.str_field("slug");
                let plan = record
                    .data
                    .get("plan_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("hobby");
                let provision_body = serde_json::json!({ "subdomain": slug, "plan": plan });
                let provision_bytes = serde_json::to_vec(&provision_body).unwrap_or_default();

                match control_plane_request(
                    ctx,
                    "POST",
                    "/_control/projects",
                    Some(&provision_bytes),
                )
                .await
                {
                    Ok((sc, resp)) if sc < 300 => {
                        let mut update_data = HashMap::new();
                        update_data.insert(
                            "status".to_string(),
                            serde_json::Value::String("active".to_string()),
                        );
                        if let Some(tid) = resp.get("id").and_then(|v| v.as_str()) {
                            update_data.insert(
                                "tenant_id".to_string(),
                                serde_json::Value::String(tid.to_string()),
                            );
                        }
                        if let Some(sub) = resp.get("subdomain").and_then(|v| v.as_str()) {
                            update_data.insert(
                                "subdomain".to_string(),
                                serde_json::Value::String(sub.to_string()),
                            );
                        }
                        update_data.insert("provision_error".to_string(), serde_json::Value::Null);
                        update_data.insert(
                            "updated_at".to_string(),
                            serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                        );
                        match db::update(ctx, PROJECTS_COLLECTION, id, update_data).await {
                            Ok(updated) => return json_respond(msg, &updated),
                            Err(e) => return err_internal(msg, &format!("Database error: {e}")),
                        }
                    }
                    Ok((sc, resp)) => {
                        let error_msg = resp
                            .get("error")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        update_status(ctx, id, "failed").await;
                        return err_internal(
                            msg,
                            &format!("Provisioning failed ({}): {}", sc, error_msg),
                        );
                    }
                    Err(e) => return err_internal(msg, &e),
                }
            }
            "deprovision" => {
                if !subdomain.is_empty() {
                    let cp_path = format!("/_control/projects/{}", subdomain);
                    match control_plane_request(ctx, "DELETE", &cp_path, None).await {
                        Ok((sc, _)) if sc < 300 || sc == 404 => {
                            update_status(ctx, id, "deleted").await;
                        }
                        Ok((sc, resp)) => {
                            let error_msg = resp
                                .get("error")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            return err_internal(
                                msg,
                                &format!("Deprovision failed ({}): {}", sc, error_msg),
                            );
                        }
                        Err(e) => return err_internal(msg, &e),
                    }
                }
                let now = chrono::Utc::now().to_rfc3339();
                let mut del_data = HashMap::new();
                del_data.insert(
                    "status".to_string(),
                    serde_json::Value::String("deleted".to_string()),
                );
                del_data.insert(
                    "deleted_at".to_string(),
                    serde_json::Value::String(now.clone()),
                );
                del_data.insert("updated_at".to_string(), serde_json::Value::String(now));
                match db::update(ctx, PROJECTS_COLLECTION, id, del_data).await {
                    Ok(updated) => return json_respond(msg, &updated),
                    Err(e) => return err_internal(msg, &format!("Database error: {e}")),
                }
            }
            _ => return err_bad_request(msg, &format!("Unknown action: {admin_action}")),
        }
    }

    body.insert(
        "updated_at".to_string(),
        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
    );

    match db::update(ctx, PROJECTS_COLLECTION, id, body).await {
        Ok(record) => json_respond(msg, &record),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found(msg, "Deployment not found"),
        Err(e) => err_internal(msg, &format!("Database error: {e}")),
    }
}

async fn handle_admin_stats(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let total = db::count(ctx, PROJECTS_COLLECTION, &[]).await.unwrap_or(0);
    let pending = db::count(
        ctx,
        PROJECTS_COLLECTION,
        &[Filter {
            field: "status".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String("pending".to_string()),
        }],
    )
    .await
    .unwrap_or(0);
    let active = db::count(
        ctx,
        PROJECTS_COLLECTION,
        &[Filter {
            field: "status".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String("active".to_string()),
        }],
    )
    .await
    .unwrap_or(0);
    let inactive = db::count(
        ctx,
        PROJECTS_COLLECTION,
        &[Filter {
            field: "status".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String("inactive".to_string()),
        }],
    )
    .await
    .unwrap_or(0);
    let stopped = db::count(
        ctx,
        PROJECTS_COLLECTION,
        &[Filter {
            field: "status".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String("stopped".to_string()),
        }],
    )
    .await
    .unwrap_or(0);
    let failed = db::count(
        ctx,
        PROJECTS_COLLECTION,
        &[Filter {
            field: "status".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String("failed".to_string()),
        }],
    )
    .await
    .unwrap_or(0);
    let deleted = db::count(
        ctx,
        PROJECTS_COLLECTION,
        &[Filter {
            field: "status".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String("deleted".to_string()),
        }],
    )
    .await
    .unwrap_or(0);

    json_respond(
        msg,
        &serde_json::json!({
            "total": total,
            "pending": pending,
            "active": active,
            "inactive": inactive,
            "stopped": stopped,
            "failed": failed,
            "deleted": deleted
        }),
    )
}
