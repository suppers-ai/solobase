use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp};

use crate::wafer::block_world::types::*;
use crate::helpers::*;

// ---------------------------------------------------------------------------
// Quota config
// ---------------------------------------------------------------------------

pub struct QuotaConfig {
    pub max_storage_bytes: i64,
    pub max_file_size_bytes: i64,
    pub max_files_per_bucket: i64,
}

impl Default for QuotaConfig {
    fn default() -> Self {
        Self {
            max_storage_bytes: 1_073_741_824, // 1GB
            max_file_size_bytes: 104_857_600,  // 100MB
            max_files_per_bucket: 10_000,
        }
    }
}

// ---------------------------------------------------------------------------
// Public helpers
// ---------------------------------------------------------------------------

pub fn get_user_quota(user_id: &str) -> QuotaConfig {
    match db::get_by_field("cloud_quotas", "user_id", serde_json::json!(user_id)) {
        Ok(record) => QuotaConfig {
            max_storage_bytes: record.data.get("max_storage_bytes").and_then(|v| v.as_i64()).unwrap_or(1_073_741_824),
            max_file_size_bytes: record.data.get("max_file_size_bytes").and_then(|v| v.as_i64()).unwrap_or(104_857_600),
            max_files_per_bucket: record.data.get("max_files_per_bucket").and_then(|v| v.as_i64()).unwrap_or(10_000),
        },
        Err(_) => QuotaConfig::default(),
    }
}

pub fn get_user_usage(user_id: &str) -> serde_json::Value {
    let filters = vec![Filter {
        field: "uploaded_by".to_string(),
        operator: FilterOp::Equal,
        value: serde_json::json!(user_id),
    }];

    let total_bytes = db::sum("storage_objects", "size", &filters).unwrap_or(0.0) as i64;
    let file_count = db::count("storage_objects", &filters).unwrap_or(0);

    serde_json::json!({
        "total_bytes": total_bytes,
        "file_count": file_count
    })
}

pub fn check_quota(msg: &Message, user_id: &str, file_size: i64) -> Result<(), BlockResult> {
    let quota = get_user_quota(user_id);
    let usage = get_user_usage(user_id);

    let current_bytes = usage.get("total_bytes").and_then(|v| v.as_i64()).unwrap_or(0);

    if file_size > quota.max_file_size_bytes {
        return Err(err_bad_request(
            msg,
            &format!("File exceeds maximum size of {} bytes", quota.max_file_size_bytes),
        ));
    }

    if current_bytes + file_size > quota.max_storage_bytes {
        return Err(err_bad_request(msg, "Storage quota exceeded"));
    }

    Ok(())
}
