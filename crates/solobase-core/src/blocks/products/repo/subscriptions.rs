//! Data access for the platform-billing subscriptions table.

use wafer_block::db::{Filter, FilterOp, ListOptions};
use wafer_core::clients::database as db;
use wafer_run::{context::Context, WaferError};
use wafer_sql_utils::{
    aggregate::{build_grouped_query, AggFunc, AggregateColumn, GroupedQueryConfig},
    Backend,
};

/// Platform-billing subscription table — one row per user.
pub(crate) const SUBSCRIPTIONS_TABLE: &str = "suppers_ai__products__subscriptions";

/// Insert-or-update the platform subscription for a user. The row id is the
/// deterministic `sub_{user_id}` so two webhooks racing for the same user hit
/// the same primary key and the `user_id`-conflict clause merges them rather
/// than inserting duplicates. Returns rows affected.
pub(crate) async fn upsert_platform(
    ctx: &dyn Context,
    user_id: &str,
    stripe_customer_id: &str,
    stripe_subscription_id: &str,
    plan: &str,
) -> Result<i64, WaferError> {
    let now = chrono::Utc::now().to_rfc3339();
    let sub_id = format!("sub_{user_id}");
    let stmt = wafer_sql_utils::upsert::build_upsert(
        SUBSCRIPTIONS_TABLE,
        &[
            ("id".to_string(), serde_json::json!(sub_id)),
            ("user_id".to_string(), serde_json::json!(user_id)),
            (
                "stripe_customer_id".to_string(),
                serde_json::json!(stripe_customer_id),
            ),
            (
                "stripe_subscription_id".to_string(),
                serde_json::json!(stripe_subscription_id),
            ),
            ("plan".to_string(), serde_json::json!(plan)),
            ("status".to_string(), serde_json::json!("active")),
            ("created_at".to_string(), serde_json::json!(&now)),
            ("updated_at".to_string(), serde_json::json!(&now)),
        ],
        &["user_id"],
        &[
            "stripe_customer_id",
            "stripe_subscription_id",
            "plan",
            "status",
            "updated_at",
        ],
        Backend::Sqlite,
    );
    db::execute(ctx, &stmt).await
}

/// Sync status (and optionally plan) from a `customer.subscription.updated`
/// event, matched by Stripe subscription id. Returns rows affected.
pub(crate) async fn update_status_plan(
    ctx: &dyn Context,
    stripe_subscription_id: &str,
    status: &str,
    plan: Option<&str>,
) -> Result<i64, WaferError> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut data: Vec<(String, serde_json::Value)> = vec![
        ("status".to_string(), serde_json::json!(status)),
        ("updated_at".to_string(), serde_json::json!(&now)),
    ];
    if let Some(plan) = plan {
        data.push(("plan".to_string(), serde_json::json!(plan)));
    }
    let stmt = wafer_sql_utils::query::build_update_where(
        SUBSCRIPTIONS_TABLE,
        &data,
        &[Filter {
            field: "stripe_subscription_id".into(),
            operator: FilterOp::Equal,
            value: serde_json::json!(stripe_subscription_id),
        }],
        Backend::Sqlite,
    );
    db::execute(ctx, &stmt).await
}

/// Mark a subscription past-due with a 7-day grace window
/// (`invoice.payment_failed`). Returns rows affected.
pub(crate) async fn mark_past_due(
    ctx: &dyn Context,
    stripe_subscription_id: &str,
) -> Result<i64, WaferError> {
    let now = chrono::Utc::now();
    let grace_end = (now + chrono::Duration::days(7)).to_rfc3339();
    let now = now.to_rfc3339();
    let stmt = wafer_sql_utils::query::build_update_where(
        SUBSCRIPTIONS_TABLE,
        &[
            ("status".to_string(), serde_json::json!("past_due")),
            (
                "grace_period_end".to_string(),
                serde_json::json!(&grace_end),
            ),
            ("updated_at".to_string(), serde_json::json!(&now)),
        ],
        &[Filter {
            field: "stripe_subscription_id".into(),
            operator: FilterOp::Equal,
            value: serde_json::json!(stripe_subscription_id),
        }],
        Backend::Sqlite,
    );
    db::execute(ctx, &stmt).await
}

/// Cancel a subscription and reset every addon column to 0
/// (`customer.subscription.deleted`). Returns rows affected.
pub(crate) async fn cancel_and_reset_addons(
    ctx: &dyn Context,
    stripe_subscription_id: &str,
) -> Result<i64, WaferError> {
    let now = chrono::Utc::now().to_rfc3339();
    let stmt = wafer_sql_utils::query::build_update_where(
        SUBSCRIPTIONS_TABLE,
        &[
            ("status".to_string(), serde_json::json!("cancelled")),
            ("addon_projects".to_string(), serde_json::json!(0)),
            ("addon_requests".to_string(), serde_json::json!(0)),
            ("addon_r2_bytes".to_string(), serde_json::json!(0)),
            ("addon_d1_bytes".to_string(), serde_json::json!(0)),
            ("updated_at".to_string(), serde_json::json!(&now)),
        ],
        &[Filter {
            field: "stripe_subscription_id".into(),
            operator: FilterOp::Equal,
            value: serde_json::json!(stripe_subscription_id),
        }],
        Backend::Sqlite,
    );
    db::execute(ctx, &stmt).await
}

/// Set the addon column totals for a user's active subscription. The caller
/// (stripe.rs) parses Stripe subscription-item metadata into the four totals;
/// this writes them. Returns rows affected.
pub(crate) async fn set_addon_totals(
    ctx: &dyn Context,
    user_id: &str,
    projects: i64,
    requests: i64,
    r2_bytes: i64,
    d1_bytes: i64,
) -> Result<i64, WaferError> {
    let now = chrono::Utc::now().to_rfc3339();
    let stmt = wafer_sql_utils::query::build_update_where(
        SUBSCRIPTIONS_TABLE,
        &[
            ("addon_projects".to_string(), serde_json::json!(projects)),
            ("addon_requests".to_string(), serde_json::json!(requests)),
            ("addon_r2_bytes".to_string(), serde_json::json!(r2_bytes)),
            ("addon_d1_bytes".to_string(), serde_json::json!(d1_bytes)),
            ("updated_at".to_string(), serde_json::json!(now)),
        ],
        &[
            Filter {
                field: "user_id".into(),
                operator: FilterOp::Equal,
                value: serde_json::json!(user_id),
            },
            Filter {
                field: "status".into(),
                operator: FilterOp::Equal,
                value: serde_json::json!("active"),
            },
        ],
        Backend::Sqlite,
    );
    db::execute(ctx, &stmt).await
}

/// Look up the user_id owning a Stripe subscription. Errors collapse to `None`
/// (preserves the original `get_user_for_stripe_sub` behaviour).
pub(crate) async fn find_user_by_stripe_sub(
    ctx: &dyn Context,
    stripe_subscription_id: &str,
) -> Option<String> {
    let opts = ListOptions {
        filters: vec![Filter {
            field: "stripe_subscription_id".into(),
            operator: FilterOp::Equal,
            value: serde_json::json!(stripe_subscription_id),
        }],
        limit: 1,
        ..Default::default()
    };
    let stmt = wafer_sql_utils::query::build_select_columns(
        SUBSCRIPTIONS_TABLE,
        &["user_id"],
        &opts,
        None,
        Backend::Sqlite,
    );
    let rows = db::query(ctx, &stmt).await.ok()?;
    rows.first()?
        .data
        .get("user_id")
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Fetch a user's subscription row with addon columns coalesced to 0 for the
/// admin subscription-status endpoint. Errors / no-row collapse to `None`
/// (preserves the original `handle_subscription` behaviour).
pub(crate) async fn subscription_for_user(
    ctx: &dyn Context,
    user_id: &str,
) -> Option<serde_json::Value> {
    let coalesced = |alias: &str, field: &str| AggregateColumn {
        func: AggFunc::Coalesce(serde_json::json!(0)),
        field: Some(field.into()),
        alias: alias.into(),
        cast_as: None,
        inner_expr: None,
    };
    let cfg = GroupedQueryConfig {
        table: SUBSCRIPTIONS_TABLE.into(),
        select_columns: vec![
            "id".into(),
            "plan".into(),
            "status".into(),
            "stripe_subscription_id".into(),
            "grace_period_end".into(),
            "created_at".into(),
            "updated_at".into(),
        ],
        aggregates: vec![
            coalesced("addon_projects", "addon_projects"),
            coalesced("addon_requests", "addon_requests"),
            coalesced("addon_r2_bytes", "addon_r2_bytes"),
            coalesced("addon_d1_bytes", "addon_d1_bytes"),
        ],
        filters: vec![Filter {
            field: "user_id".into(),
            operator: FilterOp::Equal,
            value: serde_json::json!(user_id),
        }],
        group_by: vec![],
        order_by: vec![],
        limit: Some(1),
    };
    let stmt = build_grouped_query(cfg, Backend::Sqlite);
    match db::query(ctx, &stmt).await {
        Ok(records) if !records.is_empty() => serde_json::to_value(&records[0].data).ok(),
        _ => None,
    }
}
