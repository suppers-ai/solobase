use std::collections::HashMap;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::{InputStream, OutputStream};

use crate::blocks::helpers::{
    self, err_bad_request, err_forbidden, err_internal, err_not_found, ok_json,
};

use super::{ACCESS_LOGS_COLLECTION, BUCKETS_COLLECTION, QUOTAS_COLLECTION, SHARES_COLLECTION};

pub async fn handle(ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        // User-facing cloud storage
        ("retrieve", "/b/cloudstorage/shares") => handle_list_shares(ctx, &msg).await,
        ("create", "/b/cloudstorage/shares") => handle_create_share(ctx, &msg, input).await,
        ("delete", _) if path.starts_with("/b/cloudstorage/shares/") => {
            handle_delete_share(ctx, &msg).await
        }
        ("retrieve", "/b/cloudstorage/quota") => handle_get_quota(ctx, &msg).await,
        // Admin cloud storage
        ("retrieve", "/admin/b/cloudstorage/shares") => handle_admin_list_shares(ctx, &msg).await,
        ("retrieve", "/admin/b/cloudstorage/access-logs") => handle_access_logs(ctx, &msg).await,
        ("retrieve", "/admin/b/cloudstorage/quotas") => handle_admin_quotas(ctx, &msg).await,
        ("update", _) if path.starts_with("/admin/b/cloudstorage/quotas/") => {
            handle_update_quota(ctx, &msg, input).await
        }
        _ => err_not_found("not found"),
    }
}

async fn handle_list_shares(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let user_id = msg.user_id().to_string();

    let opts = ListOptions {
        filters: vec![Filter {
            field: "created_by".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id),
        }],
        sort: vec![SortField {
            field: "created_at".to_string(),
            desc: true,
        }],
        limit: 100,
        ..Default::default()
    };

    match db::list(ctx, SHARES_COLLECTION, &opts).await {
        Ok(result) => ok_json(&result),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_create_share(
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
) -> OutputStream {
    #[derive(serde::Deserialize)]
    struct Req {
        bucket: String,
        key: String,
        expires_in_hours: Option<i64>,
        max_access_count: Option<i64>,
    }
    let raw = input.collect_to_bytes().await;
    let body: Req = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    // Validate bucket/key
    if body.bucket.is_empty() || body.key.is_empty() {
        return err_bad_request("Bucket and key are required");
    }
    if body.bucket.contains("..") || body.bucket.contains('/') || body.bucket.contains('\0') {
        return err_bad_request("Invalid bucket name");
    }
    if body.key.contains("..") || body.key.starts_with('/') || body.key.contains('\0') {
        return err_bad_request("Invalid object key");
    }

    // Verify the user owns this bucket
    let is_admin = helpers::is_admin(&msg);
    if !is_admin {
        let user_id = msg.user_id();
        let opts = wafer_core::clients::database::ListOptions {
            filters: vec![
                wafer_core::clients::database::Filter {
                    field: "name".to_string(),
                    operator: wafer_core::clients::database::FilterOp::Equal,
                    value: serde_json::Value::String(body.bucket.clone()),
                },
                wafer_core::clients::database::Filter {
                    field: "created_by".to_string(),
                    operator: wafer_core::clients::database::FilterOp::Equal,
                    value: serde_json::Value::String(user_id.to_string()),
                },
            ],
            limit: 1,
            ..Default::default()
        };
        let owns_bucket = match db::list(ctx, BUCKETS_COLLECTION, &opts).await {
            Ok(result) => !result.records.is_empty(),
            _ => false,
        };
        if !owns_bucket {
            return err_forbidden("Access denied to this bucket");
        }
    }

    // Verify the file actually exists before creating a share
    if wafer_core::clients::storage::get(ctx, &body.bucket, &body.key)
        .await
        .is_err()
    {
        return err_not_found("File not found in storage");
    }

    // Generate share token
    let token = super::share::generate_share_token(ctx, &body.bucket, &body.key).await;
    let token = match token {
        Ok(t) => t,
        Err(r) => return r,
    };

    let now = chrono::Utc::now();
    let expires_at = body
        .expires_in_hours
        .map(|h| (now + chrono::Duration::hours(h)).to_rfc3339());

    let mut data = HashMap::new();
    data.insert(
        "token".to_string(),
        serde_json::Value::String(token.clone()),
    );
    data.insert("bucket".to_string(), serde_json::Value::String(body.bucket));
    data.insert("key".to_string(), serde_json::Value::String(body.key));
    data.insert(
        "created_by".to_string(),
        serde_json::Value::String(msg.user_id().to_string()),
    );
    data.insert(
        "created_at".to_string(),
        serde_json::Value::String(now.to_rfc3339()),
    );
    data.insert("access_count".to_string(), serde_json::json!(0));
    if let Some(exp) = &expires_at {
        data.insert(
            "expires_at".to_string(),
            serde_json::Value::String(exp.clone()),
        );
    }
    if let Some(max) = body.max_access_count {
        data.insert("max_access_count".to_string(), serde_json::json!(max));
    }

    match db::create(ctx, SHARES_COLLECTION, data).await {
        Ok(record) => ok_json(&serde_json::json!({
            "id": record.id,
            "token": token,
            "direct_url": format!("/b/storage/direct/{}", token)
        })),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_delete_share(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    let id = path.strip_prefix("/b/cloudstorage/shares/").unwrap_or("");
    if id.is_empty() {
        return err_bad_request("Missing share ID");
    }

    // Verify ownership
    if let Ok(share) = db::get(ctx, SHARES_COLLECTION, id).await {
        let owner = share
            .data
            .get("created_by")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if owner != msg.user_id() && !helpers::is_admin(&msg) {
            return err_forbidden("Cannot delete another user's share");
        }
    }

    match db::delete(ctx, SHARES_COLLECTION, id).await {
        Ok(()) => ok_json(&serde_json::json!({"deleted": true})),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found("Share not found"),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_get_quota(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let quota = super::quota::get_user_quota(ctx, msg.user_id()).await;
    let usage = super::quota::get_user_usage(ctx, msg.user_id()).await;
    ok_json(&serde_json::json!({
        "quota": quota,
        "usage": usage
    }))
}

async fn handle_admin_list_shares(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let (page, page_size, _) = msg.pagination_params(20);
    let opts = ListOptions {
        sort: vec![SortField {
            field: "created_at".to_string(),
            desc: true,
        }],
        limit: page_size as i64,
        offset: ((page - 1) * page_size) as i64,
        ..Default::default()
    };
    match db::list(ctx, SHARES_COLLECTION, &opts).await {
        Ok(result) => ok_json(&result),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_access_logs(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let (page, page_size, _) = msg.pagination_params(50);

    let mut filters = Vec::new();
    let share_id = msg.query("share_id").to_string();
    if !share_id.is_empty() {
        filters.push(Filter {
            field: "share_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(share_id),
        });
    }

    let opts = ListOptions {
        filters,
        sort: vec![SortField {
            field: "accessed_at".to_string(),
            desc: true,
        }],
        limit: page_size as i64,
        offset: ((page - 1) * page_size) as i64,
    };

    match db::list(ctx, ACCESS_LOGS_COLLECTION, &opts).await {
        Ok(result) => ok_json(&result),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_admin_quotas(ctx: &dyn Context, _msg: &Message) -> OutputStream {
    let opts = ListOptions {
        limit: 1000,
        ..Default::default()
    };
    match db::list(ctx, QUOTAS_COLLECTION, &opts).await {
        Ok(result) => ok_json(&result),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

async fn handle_update_quota(
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
) -> OutputStream {
    let path = msg.path();
    let user_id = path
        .strip_prefix("/admin/b/cloudstorage/quotas/")
        .unwrap_or("");
    if user_id.is_empty() {
        return err_bad_request("Missing user ID");
    }

    let raw = input.collect_to_bytes().await;
    let body: HashMap<String, serde_json::Value> = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    let mut data = body;
    data.insert(
        "user_id".to_string(),
        serde_json::Value::String(user_id.to_string()),
    );
    data.insert(
        "updated_at".to_string(),
        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
    );

    match db::upsert(
        ctx,
        QUOTAS_COLLECTION,
        "user_id",
        serde_json::Value::String(user_id.to_string()),
        data,
    )
    .await
    {
        Ok(record) => ok_json(&record),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}
