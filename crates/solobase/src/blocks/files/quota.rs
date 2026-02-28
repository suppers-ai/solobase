use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_run::services::database::{self, Filter, FilterOp};
use super::get_db;
use super::models::QuotaConfig;

pub fn get_user_quota(ctx: &dyn Context, user_id: &str) -> QuotaConfig {
    let db = match get_db(ctx) {
        Ok(db) => db,
        Err(_) => return QuotaConfig::default(),
    };

    // Check for user-specific override
    match database::get_by_field(db.as_ref(), "cloud_quotas", "user_id", serde_json::Value::String(user_id.to_string())) {
        Ok(record) => {
            QuotaConfig {
                max_storage_bytes: record.data.get("max_storage_bytes").and_then(|v| v.as_i64()).unwrap_or(1_073_741_824),
                max_file_size_bytes: record.data.get("max_file_size_bytes").and_then(|v| v.as_i64()).unwrap_or(104_857_600),
                max_files_per_bucket: record.data.get("max_files_per_bucket").and_then(|v| v.as_i64()).unwrap_or(10_000),
                reset_period_days: record.data.get("reset_period_days").and_then(|v| v.as_i64()).unwrap_or(0),
            }
        }
        Err(_) => QuotaConfig::default(),
    }
}

pub fn get_user_usage(ctx: &dyn Context, user_id: &str) -> serde_json::Value {
    let db = match get_db(ctx) {
        Ok(db) => db,
        Err(_) => return serde_json::json!({"total_bytes": 0, "file_count": 0}),
    };

    let filters = vec![Filter {
        field: "uploaded_by".to_string(),
        operator: FilterOp::Equal,
        value: serde_json::Value::String(user_id.to_string()),
    }];

    let total_bytes = db.sum("storage_objects", "size", &filters).unwrap_or(0.0) as i64;
    let file_count = db.count("storage_objects", &filters).unwrap_or(0);

    serde_json::json!({
        "total_bytes": total_bytes,
        "file_count": file_count
    })
}

pub fn check_quota(ctx: &dyn Context, user_id: &str, _bucket: &str, file_size: i64) -> Result<(), Result_> {
    let quota = get_user_quota(ctx, user_id);
    let usage = get_user_usage(ctx, user_id);

    let current_bytes = usage.get("total_bytes").and_then(|v| v.as_i64()).unwrap_or(0);

    if file_size > quota.max_file_size_bytes {
        return Err(err_bad_request(
            Message::new("", ""),
            &format!("File exceeds maximum size of {} bytes", quota.max_file_size_bytes),
        ));
    }

    if current_bytes + file_size > quota.max_storage_bytes {
        return Err(err_bad_request(
            Message::new("", ""),
            "Storage quota exceeded",
        ));
    }

    Ok(())
}
