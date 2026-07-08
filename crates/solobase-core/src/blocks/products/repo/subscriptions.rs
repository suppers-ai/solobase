//! Data access for the platform-billing subscriptions table.

use wafer_block::db::{Filter, FilterOp, ListOptions};
use wafer_core::clients::database as db;
use wafer_run::{context::Context, ErrorCode, WaferError};

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
    let backend = crate::db_backend(ctx).await;
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
        backend,
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
    let backend = crate::db_backend(ctx).await;
    let stmt = wafer_sql_utils::query::build_update_where(
        SUBSCRIPTIONS_TABLE,
        &data,
        &[Filter {
            field: "stripe_subscription_id".into(),
            operator: FilterOp::Equal,
            value: serde_json::json!(stripe_subscription_id),
        }],
        backend,
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
    let backend = crate::db_backend(ctx).await;
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
        backend,
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
    let backend = crate::db_backend(ctx).await;
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
        backend,
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
    let backend = crate::db_backend(ctx).await;
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
        backend,
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
    let backend = crate::db_backend(ctx).await;
    let stmt = wafer_sql_utils::query::build_select_columns(
        SUBSCRIPTIONS_TABLE,
        &["user_id"],
        &opts,
        None,
        backend,
    );
    let rows = db::query(ctx, &stmt).await.ok()?;
    rows.first()?
        .data
        .get("user_id")
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Whether the user has an `active` subscription whose `plan` equals `plan`.
pub(crate) async fn active_plan_exists(ctx: &dyn Context, user_id: &str, plan: &str) -> bool {
    let opts = ListOptions {
        filters: vec![
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
            Filter {
                field: "plan".into(),
                operator: FilterOp::Equal,
                value: serde_json::json!(plan),
            },
        ],
        limit: 1,
        ..Default::default()
    };
    let backend = crate::db_backend(ctx).await;
    let stmt = wafer_sql_utils::query::build_select_columns(
        SUBSCRIPTIONS_TABLE,
        &["id"],
        &opts,
        None,
        backend,
    );
    matches!(db::query(ctx, &stmt).await, Ok(rows) if !rows.is_empty())
}

/// Fetch a user's subscription row with addon columns coalesced to 0 for the
/// admin subscription-status endpoint. Errors / no-row collapse to `None`
/// (preserves the original `handle_subscription` behaviour).
///
/// This is not a grouped aggregate — it's a single-row lookup by `user_id`
/// with 4 addon columns defaulted from NULL/absent to 0, so it's built on
/// `db::get_by_field` + a Rust-side coalesce rather than `db::aggregate`
/// (which can't express an empty-group `COALESCE`). The response is returned
/// directly to the authenticated user via `handle_subscription`, so the
/// output is projected down to the same curated column set the old
/// `select_columns` used — `user_id`/`stripe_customer_id` must not leak.
pub(crate) async fn subscription_for_user(
    ctx: &dyn Context,
    user_id: &str,
) -> Option<serde_json::Value> {
    let record = match db::get_by_field(
        ctx,
        SUBSCRIPTIONS_TABLE,
        "user_id",
        serde_json::json!(user_id),
    )
    .await
    {
        Ok(record) => record,
        Err(e) if e.code == ErrorCode::NotFound => return None,
        Err(_) => return None,
    };

    let mut out = serde_json::Map::new();
    for col in [
        "id",
        "plan",
        "status",
        "stripe_subscription_id",
        "grace_period_end",
        "created_at",
        "updated_at",
    ] {
        if let Some(v) = record.data.get(col) {
            out.insert(col.to_string(), v.clone());
        }
    }
    for col in [
        "addon_projects",
        "addon_requests",
        "addon_r2_bytes",
        "addon_d1_bytes",
    ] {
        let v = record.data.get(col).and_then(|v| v.as_i64()).unwrap_or(0);
        out.insert(col.to_string(), serde_json::json!(v));
    }
    Some(serde_json::Value::Object(out))
}
