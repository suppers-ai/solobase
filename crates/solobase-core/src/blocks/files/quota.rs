use wafer_core::clients::{
    database as db,
    database::{Filter, FilterOp},
};
use wafer_run::{context::Context, OutputStream};

use super::{models::QuotaConfig, OBJECTS_TABLE};
use crate::blocks::helpers::{err_bad_request, RecordExt};

/// Per-user quota override table. Stores explicit byte/file caps that
/// override the block defaults for individual users.
pub(crate) const TABLE: &str = "suppers_ai__files__cloud_quotas";

pub async fn get_user_quota(ctx: &dyn Context, user_id: &str) -> QuotaConfig {
    // Check for user-specific override
    match db::get_by_field(
        ctx,
        TABLE,
        "user_id",
        serde_json::Value::String(user_id.to_string()),
    )
    .await
    {
        Ok(record) => QuotaConfig {
            max_storage_bytes: record
                .data
                .get("max_storage_bytes")
                .and_then(|v| v.as_i64())
                .unwrap_or(1_073_741_824),
            max_file_size_bytes: record
                .data
                .get("max_file_size_bytes")
                .and_then(|v| v.as_i64())
                .unwrap_or(104_857_600),
            max_files_per_bucket: record
                .data
                .get("max_files_per_bucket")
                .and_then(|v| v.as_i64())
                .unwrap_or(10_000),
            reset_period_days: record.i64_field("reset_period_days"),
        },
        Err(_) => QuotaConfig::default(),
    }
}

pub async fn get_user_usage(ctx: &dyn Context, user_id: &str) -> serde_json::Value {
    let filters = vec![Filter {
        field: "uploaded_by".to_string(),
        operator: FilterOp::Equal,
        value: serde_json::Value::String(user_id.to_string()),
    }];

    let total_bytes = db::sum(ctx, OBJECTS_TABLE, "size", &filters)
        .await
        .unwrap_or(0.0) as i64;
    let file_count = db::count(ctx, OBJECTS_TABLE, &filters).await.unwrap_or(0);

    serde_json::json!({
        "total_bytes": total_bytes,
        "file_count": file_count
    })
}

pub async fn check_quota(
    ctx: &dyn Context,
    user_id: &str,
    file_size: i64,
) -> Result<(), OutputStream> {
    let quota = get_user_quota(ctx, user_id).await;
    let usage = get_user_usage(ctx, user_id).await;

    let current_bytes = usage
        .get("total_bytes")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    if file_size > quota.max_file_size_bytes {
        return Err(err_bad_request(&format!(
            "File exceeds maximum size of {} bytes",
            quota.max_file_size_bytes
        )));
    }

    if current_bytes + file_size > quota.max_storage_bytes {
        return Err(err_bad_request("Storage quota exceeded"));
    }

    let file_count = usage
        .get("file_count")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if quota.max_files_per_bucket > 0 && file_count >= quota.max_files_per_bucket {
        return Err(err_bad_request(&format!(
            "File count limit reached (max {})",
            quota.max_files_per_bucket
        )));
    }

    Ok(())
}

/// Sweep `pending`-status object rows older than `older_than_seconds` for
/// the given user. Pending rows are inserted before the actual storage
/// upload to close the quota TOCTOU window; if the upload errors AND the
/// compensating delete also errors, the row sticks around and inflates the
/// user's quota usage forever. Calling this best-effort on each new upload
/// keeps the table self-healing without a separate cron.
///
/// 1 hour is a comfortable cutoff: the largest realistic upload finishes
/// inside that window, and anything still pending afterward is almost
/// certainly an orphan.
pub async fn sweep_stale_pending(ctx: &dyn Context, user_id: &str, older_than_seconds: i64) {
    let cutoff = (chrono::Utc::now() - chrono::Duration::seconds(older_than_seconds)).to_rfc3339();
    let filters = vec![
        Filter {
            field: "uploaded_by".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.to_string()),
        },
        Filter {
            field: "status".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String("pending".into()),
        },
        Filter {
            field: "uploaded_at".into(),
            operator: FilterOp::LessThan,
            value: serde_json::Value::String(cutoff),
        },
    ];
    if let Err(e) = db::delete_by_filters(ctx, OBJECTS_TABLE, filters).await {
        tracing::warn!(error = %e, user_id = %user_id, "failed to sweep stale pending uploads");
    }
}
