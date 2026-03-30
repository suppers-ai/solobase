//! Account-level usage tracking and plan limit enforcement.
//!
//! Usage is tracked per-project per-month in `project_usage` (for breakdown visibility),
//! but limits are enforced at the account level by summing across all user's projects.
//! Add-on allowances live on the `subscriptions` table (account-level).
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

/// Account-level addons read from the subscriptions table.
struct AccountAddons {
    requests: u64,
    r2_bytes: u64,
    d1_bytes: u64,
}

/// Check if the project's owner is within account-level plan limits.
///
/// Sums usage across ALL of the owner's projects for the current month,
/// then checks against plan limits + account-level addons.
pub async fn check_usage(
    db: &D1Database,
    project: &ProjectConfig,
) -> UsageCheckResult {
    let limits = get_plan_limits(&project.plan);
    let month = current_month();
    let mut result = UsageCheckResult::ok();

    let owner_id = match &project.owner_user_id {
        Some(id) if !id.is_empty() => id.clone(),
        _ => return result, // No owner = no limits (platform project)
    };

    // Check subscription status
    match check_subscription_status(db, &owner_id).await {
        SubStatus::Active => {}
        SubStatus::PastDue(warning) => {
            result.warning = Some(warning);
        }
        SubStatus::Inactive(err) => {
            return UsageCheckResult { error: Some(err), warning: None };
        }
    }

    // Free tier: block all requests (no resources)
    if limits.max_requests_per_month == 0 {
        let addons = get_account_addons(db, &owner_id).await;
        if addons.requests == 0 {
            return UsageCheckResult {
                error: Some("No active subscription. Subscribe to a plan to access your project.".into()),
                warning: None,
            };
        }
    }

    // Sum usage across ALL owner's projects for this month
    let (total_requests, total_r2, total_d1) = get_account_usage(db, &owner_id, &month).await;

    // Get account-level addons from subscriptions table
    let addons = get_account_addons(db, &owner_id).await;

    // --- Check request limit ---
    let max_requests = limits.max_requests_per_month.saturating_add(addons.requests);

    if max_requests > 0 && total_requests >= max_requests {
        return UsageCheckResult {
            error: Some(format!(
                "Plan limit exceeded: {} / {} requests this month. Upgrade your plan or purchase add-ons.",
                total_requests, max_requests
            )),
            warning: None,
        };
    }

    // --- Check R2 storage limit ---
    let max_r2 = limits.max_r2_storage_bytes.saturating_add(addons.r2_bytes);

    if max_r2 > 0 && total_r2 >= max_r2 {
        return UsageCheckResult {
            error: Some(format!(
                "Object storage limit exceeded: {} / {} bytes. Upgrade your plan or purchase add-ons.",
                total_r2, max_r2
            )),
            warning: None,
        };
    }

    // --- Check D1 storage limit ---
    let max_d1 = limits.max_d1_storage_bytes.saturating_add(addons.d1_bytes);

    if max_d1 > 0 && total_d1 >= max_d1 {
        return UsageCheckResult {
            error: Some(format!(
                "Database storage limit exceeded: {} / {} bytes. Upgrade your plan or purchase add-ons.",
                total_d1, max_d1
            )),
            warning: None,
        };
    }

    // --- Warnings at 80% usage ---
    if max_requests > 0 {
        let pct = ((total_requests + 1) as f64 / max_requests as f64 * 100.0) as u64;
        if pct >= 80 && result.warning.is_none() {
            result.warning = Some(format!(
                "{}% of monthly API requests used ({} / {})",
                pct, total_requests + 1, max_requests
            ));
        }
    }

    if max_r2 > 0 && total_r2 > 0 {
        let pct = (total_r2 as f64 / max_r2 as f64 * 100.0) as u64;
        if pct >= 80 && result.warning.is_none() {
            result.warning = Some(format!(
                "{}% of object storage used ({} / {} bytes)",
                pct, total_r2, max_r2
            ));
        }
    }

    if max_d1 > 0 && total_d1 > 0 {
        let pct = (total_d1 as f64 / max_d1 as f64 * 100.0) as u64;
        if pct >= 80 && result.warning.is_none() {
            result.warning = Some(format!(
                "{}% of database storage used ({} / {} bytes)",
                pct, total_d1, max_d1
            ));
        }
    }

    result
}

/// Increment the per-project usage counter (non-blocking, called via waitUntil).
/// Tracking stays per-project for breakdown visibility.
pub async fn increment_usage(db: &D1Database, project_id: &str) {
    let month = current_month();
    let usage_id = format!("{}:{}", project_id, month);
    if let Ok(stmt) = db.prepare(
        "INSERT INTO project_usage (id, project_id, month, requests, r2_bytes, d1_bytes) \
         VALUES (?1, ?2, ?3, 1, 0, 0) \
         ON CONFLICT (project_id, month) DO UPDATE SET requests = requests + 1"
    ).bind(&[usage_id.into(), project_id.into(), month.into()]) {
        let _ = stmt.run().await;
    }
}

// ---------------------------------------------------------------------------
// Account-level usage aggregation
// ---------------------------------------------------------------------------

/// Sum requests, r2_bytes, and d1_bytes across all of a user's projects for a month.
async fn get_account_usage(db: &D1Database, owner_id: &str, month: &str) -> (u64, u64, u64) {
    let query = "\
        SELECT COALESCE(SUM(pu.requests), 0) as total_requests, \
               COALESCE(SUM(pu.r2_bytes), 0) as total_r2, \
               COALESCE(SUM(COALESCE(pu.d1_bytes, 0)), 0) as total_d1 \
        FROM project_usage pu \
        JOIN projects p ON p.id = pu.project_id \
        WHERE p.owner_user_id = ?1 AND pu.month = ?2";

    match db.prepare(query).bind(&[owner_id.into(), month.into()]) {
        Ok(stmt) => match stmt.first::<serde_json::Value>(None).await {
            Ok(Some(row)) => (
                row.get("total_requests").and_then(|v| v.as_u64()).unwrap_or(0),
                row.get("total_r2").and_then(|v| v.as_u64()).unwrap_or(0),
                row.get("total_d1").and_then(|v| v.as_u64()).unwrap_or(0),
            ),
            _ => (0, 0, 0),
        },
        Err(_) => (0, 0, 0),
    }
}

/// Read account-level addon allowances from the subscriptions table.
async fn get_account_addons(db: &D1Database, owner_id: &str) -> AccountAddons {
    let query = "SELECT \
        COALESCE(addon_requests, 0) as addon_requests, \
        COALESCE(addon_r2_bytes, 0) as addon_r2_bytes, \
        COALESCE(addon_d1_bytes, 0) as addon_d1_bytes \
        FROM subscriptions WHERE user_id = ?1 AND status = 'active' LIMIT 1";

    match db.prepare(query).bind(&[owner_id.into()]) {
        Ok(stmt) => match stmt.first::<serde_json::Value>(None).await {
            Ok(Some(row)) => AccountAddons {
                requests: row.get("addon_requests").and_then(|v| v.as_u64()).unwrap_or(0),
                r2_bytes: row.get("addon_r2_bytes").and_then(|v| v.as_u64()).unwrap_or(0),
                d1_bytes: row.get("addon_d1_bytes").and_then(|v| v.as_u64()).unwrap_or(0),
            },
            _ => AccountAddons { requests: 0, r2_bytes: 0, d1_bytes: 0 },
        },
        Err(_) => AccountAddons { requests: 0, r2_bytes: 0, d1_bytes: 0 },
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
    chrono::Utc::now().format("%Y-%m").to_string()
}
