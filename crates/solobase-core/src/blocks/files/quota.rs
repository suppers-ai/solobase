use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp};
use crate::blocks::helpers::RecordExt;
use super::models::QuotaConfig;

pub async fn get_user_quota(ctx: &dyn Context, user_id: &str) -> QuotaConfig {
    // Check for user-specific override
    match db::get_by_field(ctx, "cloud_quotas", "user_id", serde_json::Value::String(user_id.to_string())).await {
        Ok(record) => {
            QuotaConfig {
                max_storage_bytes: record.data.get("max_storage_bytes").and_then(|v| v.as_i64()).unwrap_or(1_073_741_824),
                max_file_size_bytes: record.data.get("max_file_size_bytes").and_then(|v| v.as_i64()).unwrap_or(104_857_600),
                max_files_per_bucket: record.data.get("max_files_per_bucket").and_then(|v| v.as_i64()).unwrap_or(10_000),
                reset_period_days: record.i64_field("reset_period_days"),
            }
        }
        Err(_) => QuotaConfig::default(),
    }
}

pub async fn get_user_usage(ctx: &dyn Context, user_id: &str) -> serde_json::Value {
    let filters = vec![Filter {
        field: "uploaded_by".to_string(),
        operator: FilterOp::Equal,
        value: serde_json::Value::String(user_id.to_string()),
    }];

    let total_bytes = db::sum(ctx, "storage_objects", "size", &filters).await.unwrap_or(0.0) as i64;
    let file_count = db::count(ctx, "storage_objects", &filters).await.unwrap_or(0);

    serde_json::json!({
        "total_bytes": total_bytes,
        "file_count": file_count
    })
}

pub async fn check_quota(ctx: &dyn Context, user_id: &str, file_size: i64) -> Result<(), Result_> {
    let quota = get_user_quota(ctx, user_id).await;
    let usage = get_user_usage(ctx, user_id).await;

    let current_bytes = usage.get("total_bytes").and_then(|v| v.as_i64()).unwrap_or(0);

    if file_size > quota.max_file_size_bytes {
        return Err(err_bad_request(
            &Message::new("", ""),
            &format!("File exceeds maximum size of {} bytes", quota.max_file_size_bytes),
        ));
    }

    if current_bytes + file_size > quota.max_storage_bytes {
        return Err(err_bad_request(
            &Message::new("", ""),
            "Storage quota exceeded",
        ));
    }

    Ok(())
}
