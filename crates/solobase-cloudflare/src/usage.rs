//! Project usage tracking and plan limit enforcement.
//!
//! Tracks API request counts per project per month in D1.
//! Checks plan limits before dispatching to blocks.
//!
//! The check (read) is blocking — needed before dispatching.
//! The increment (write) is non-blocking — runs via waitUntil after response.

use worker::*;

use crate::project::{get_plan_limits, ProjectConfig};

/// Result of a usage check.
pub struct UsageCheckResult {
    /// If set, the request should be rejected with 429.
    pub error: Option<String>,
    /// If set, add as X-Solobase-Warning header (request still succeeds).
    pub warning: Option<String>,
}

impl UsageCheckResult {
    fn ok() -> Self {
        Self { error: None, warning: None }
    }
}

/// Check if the project is within plan limits (blocking read).
/// Does NOT increment the counter — call `increment_usage` separately.
pub async fn check_usage(
    db: &D1Database,
    project: &ProjectConfig,
) -> UsageCheckResult {
    let limits = get_plan_limits(&project.plan);
    let month = current_month();
    let mut result = UsageCheckResult::ok();

    // Check subscription status if project has an owner
    if let Some(ref owner_id) = project.owner_user_id {
        match check_subscription_status(db, owner_id).await {
            SubStatus::Active => {}
            SubStatus::PastDue(warning) => {
                result.warning = Some(warning);
            }
            SubStatus::Inactive(err) => {
                return UsageCheckResult { error: Some(err), warning: None };
            }
        }
    }

    // Read current usage
    let (current_requests, addon_requests) = match db
        .prepare("SELECT requests, addon_requests FROM project_usage WHERE project_id = ?1 AND month = ?2")
        .bind(&[project.id.clone().into(), month.clone().into()])
    {
        Ok(stmt) => match stmt.first::<serde_json::Value>(None).await {
            Ok(Some(row)) => (
                row.get("requests").and_then(|v| v.as_u64()).unwrap_or(0),
                row.get("addon_requests").and_then(|v| v.as_u64()).unwrap_or(0),
            ),
            _ => (0, 0),
        },
        Err(_) => (0, 0),
    };

    let max_requests = limits.max_requests_per_month + addon_requests;

    // Check limit
    if current_requests >= max_requests {
        return UsageCheckResult {
            error: Some(format!(
                "Plan limit exceeded: {} / {} requests this month. Upgrade your plan or add more requests.",
                current_requests, max_requests
            )),
            warning: None,
        };
    }

    // Warn at 80% usage
    let new_count = current_requests + 1;
    if max_requests > 0 {
        let pct = (new_count as f64 / max_requests as f64 * 100.0) as u64;
        if pct >= 80 && result.warning.is_none() {
            result.warning = Some(format!(
                "{}% of monthly API requests used ({} / {})",
                pct, new_count, max_requests
            ));
        }
    }

    result
}

/// Increment the usage counter (non-blocking, called via waitUntil).
pub async fn increment_usage(db: &D1Database, project_id: &str) {
    let month = current_month();
    let usage_id = format!("{}:{}", project_id, month);
    if let Ok(stmt) = db.prepare(
        "INSERT INTO project_usage (id, project_id, month, requests, r2_bytes, addon_requests, addon_r2_bytes, addon_d1_bytes) \
         VALUES (?1, ?2, ?3, 1, 0, 0, 0, 0) \
         ON CONFLICT (project_id, month) DO UPDATE SET requests = requests + 1"
    ).bind(&[usage_id.into(), project_id.into(), month.into()]) {
        let _ = stmt.run().await;
    }
}

// ---------------------------------------------------------------------------
// Subscription status check
// ---------------------------------------------------------------------------

enum SubStatus {
    Active,
    PastDue(String),
    Inactive(String),
}

async fn check_subscription_status(db: &D1Database, user_id: &str) -> SubStatus {
    let row = match db
        .prepare("SELECT status, grace_period_end FROM subscriptions WHERE user_id = ?1 LIMIT 1")
        .bind(&[user_id.into()])
    {
        Ok(stmt) => match stmt.first::<serde_json::Value>(None).await {
            Ok(Some(row)) => row,
            _ => return SubStatus::Active, // no subscription = free tier
        },
        Err(_) => return SubStatus::Active,
    };

    let status = row.get("status").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
    let grace_end = row.get("grace_period_end").and_then(|v| v.as_str().map(String::from));

    match status.as_str() {
        "cancelled" | "suspended" => {
            SubStatus::Inactive("Subscription inactive. Please resubscribe.".into())
        }
        "past_due" => {
            if let Some(ref end) = grace_end {
                if let Ok(end_time) = chrono::DateTime::parse_from_rfc3339(end) {
                    let now = chrono::Utc::now();
                    let end_utc = end_time.with_timezone(&chrono::Utc);
                    if now > end_utc {
                        return SubStatus::Inactive(
                            "Payment overdue. Service suspended. Please update payment.".into(),
                        );
                    }
                    let days_left = (end_utc - now).num_days().max(0);
                    return SubStatus::PastDue(format!(
                        "Payment failed. {} days remaining before suspension.",
                        days_left
                    ));
                }
            }
            SubStatus::PastDue("Payment failed. Please update your payment method.".into())
        }
        _ => SubStatus::Active,
    }
}

fn current_month() -> String {
    let now = chrono::Utc::now();
    now.format("%Y-%m").to_string()
}
