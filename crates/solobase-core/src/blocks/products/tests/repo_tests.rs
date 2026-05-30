use std::collections::HashMap;

use wafer_core::clients::database as db;

use super::mock_context::*;
use crate::blocks::products::repo;

/// `cancel_and_reset_addons` flips status to cancelled and zeroes every addon
/// column for the matched subscription.
#[tokio::test]
async fn cancel_and_reset_addons_zeroes_addons_and_cancels() {
    let ctx = MockContext::new();
    let mut sd = HashMap::new();
    sd.insert("user_id".to_string(), serde_json::json!("user_1"));
    sd.insert(
        "stripe_subscription_id".to_string(),
        serde_json::json!("sub_stripe_1"),
    );
    sd.insert("status".to_string(), serde_json::json!("active"));
    sd.insert("addon_projects".to_string(), serde_json::json!(5));
    sd.insert("addon_requests".to_string(), serde_json::json!(1000));
    sd.insert("addon_r2_bytes".to_string(), serde_json::json!(42));
    sd.insert("addon_d1_bytes".to_string(), serde_json::json!(7));
    ctx.seed("suppers_ai__products__subscriptions", "sub_db_1", sd);

    let rows = repo::subscriptions::cancel_and_reset_addons(&ctx, "sub_stripe_1")
        .await
        .expect("cancel ok");
    assert_eq!(rows, 1, "exactly one subscription row updated");

    let rec = db::get(&ctx, "suppers_ai__products__subscriptions", "sub_db_1")
        .await
        .expect("row exists");
    assert_eq!(
        rec.data.get("status").and_then(|v| v.as_str()),
        Some("cancelled")
    );
    assert_eq!(
        rec.data.get("addon_projects").and_then(|v| v.as_i64()),
        Some(0)
    );
    assert_eq!(
        rec.data.get("addon_requests").and_then(|v| v.as_i64()),
        Some(0)
    );
    assert_eq!(
        rec.data.get("addon_r2_bytes").and_then(|v| v.as_i64()),
        Some(0)
    );
    assert_eq!(
        rec.data.get("addon_d1_bytes").and_then(|v| v.as_i64()),
        Some(0)
    );
}

/// `complete_atomic` transitions a pending purchase to completed and records
/// the payment intent; a second call is a 0-row no-op (idempotent).
#[tokio::test]
async fn complete_atomic_only_from_pending_or_checkout_started() {
    let ctx = MockContext::new();
    let mut pd = HashMap::new();
    pd.insert("status".to_string(), serde_json::json!("pending"));
    ctx.seed("suppers_ai__products__purchases", "pur_1", pd);

    let rows = repo::purchases::complete_atomic(&ctx, "pur_1", "pi_abc")
        .await
        .expect("complete ok");
    assert_eq!(rows, 1);
    let rec = db::get(&ctx, "suppers_ai__products__purchases", "pur_1")
        .await
        .unwrap();
    assert_eq!(
        rec.data.get("status").and_then(|v| v.as_str()),
        Some("completed")
    );
    assert_eq!(
        rec.data
            .get("provider_payment_intent_id")
            .and_then(|v| v.as_str()),
        Some("pi_abc")
    );

    // Second call: already completed -> 0 rows, no change.
    let rows2 = repo::purchases::complete_atomic(&ctx, "pur_1", "pi_zzz")
        .await
        .expect("idempotent ok");
    assert_eq!(rows2, 0, "completed purchase is not re-completed");
    let rec2 = db::get(&ctx, "suppers_ai__products__purchases", "pur_1")
        .await
        .unwrap();
    assert_eq!(
        rec2.data
            .get("provider_payment_intent_id")
            .and_then(|v| v.as_str()),
        Some("pi_abc"),
        "payment intent not overwritten by the no-op call"
    );
}

/// `refund_atomic` only transitions a completed purchase; a pending one is a
/// 0-row no-op (prevents double-refund / refunding incomplete orders).
#[tokio::test]
async fn refund_atomic_only_from_completed() {
    let ctx = MockContext::new();
    let mut completed = HashMap::new();
    completed.insert("status".to_string(), serde_json::json!("completed"));
    ctx.seed("suppers_ai__products__purchases", "pur_done", completed);
    let mut pending = HashMap::new();
    pending.insert("status".to_string(), serde_json::json!("pending"));
    ctx.seed("suppers_ai__products__purchases", "pur_pending", pending);

    let ok = repo::purchases::refund_atomic(&ctx, "pur_done", "admin_1", "duplicate")
        .await
        .expect("refund ok");
    assert_eq!(ok, 1);
    let rec = db::get(&ctx, "suppers_ai__products__purchases", "pur_done")
        .await
        .unwrap();
    assert_eq!(
        rec.data.get("status").and_then(|v| v.as_str()),
        Some("refunded")
    );
    assert_eq!(
        rec.data.get("refunded_by").and_then(|v| v.as_str()),
        Some("admin_1")
    );

    let noop = repo::purchases::refund_atomic(&ctx, "pur_pending", "admin_1", "x")
        .await
        .expect("noop ok");
    assert_eq!(noop, 0, "pending purchase cannot be refunded");
}
