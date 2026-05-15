use std::{collections::HashMap, time::Duration};

use wafer_core::clients::{crypto, database as db, storage as store};
use wafer_run::{context::Context, types::*, OutputStream};

use crate::blocks::{
    helpers::{
        err_bad_request, err_forbidden, err_internal, err_internal_no_cause, err_not_found,
        ResponseBuilder,
    },
    rate_limit::{check_rate_limit, RateLimit, RateLimitOutcome, UserRateLimiter},
};

/// Public share-link table — one row per generated token.
pub(crate) const SHARES_TABLE: &str = "suppers_ai__files__cloud_shares";

/// Access log table — one row per recorded share access (audit trail).
pub(crate) const ACCESS_LOGS_TABLE: &str = "suppers_ai__files__cloud_access_logs";

pub async fn generate_share_token(
    ctx: &dyn Context,
    bucket: &str,
    key: &str,
) -> Result<String, OutputStream> {
    let mut claims = HashMap::new();
    claims.insert(
        "bucket".to_string(),
        serde_json::Value::String(bucket.to_string()),
    );
    claims.insert(
        "key".to_string(),
        serde_json::Value::String(key.to_string()),
    );
    claims.insert(
        "type".to_string(),
        serde_json::Value::String("share".to_string()),
    );

    // SEC-055: share JWT lifetime — 30 days. The previous 1-year default
    // gave any leaked share URL effectively unbounded validity. Users who
    // need longer-lived shares can re-share; the typical use case (send
    // a link, recipient downloads within hours/days) fits well under 30d.
    const SHARE_TOKEN_TTL: Duration = Duration::from_secs(30 * 24 * 3600);
    crypto::sign(ctx, &claims, SHARE_TOKEN_TTL)
        .await
        .map_err(|e| err_internal("Token generation failed", e))
}

pub async fn handle_direct_access(
    ctx: &dyn Context,
    msg: &Message,
    limiter: &UserRateLimiter,
) -> OutputStream {
    let path = msg.path();
    let token = path.strip_prefix("/storage/direct/").unwrap_or("");
    if token.is_empty() {
        return err_bad_request("Missing share token");
    }

    // Rate-limit per remote IP before doing any work — `/storage/direct/*` is
    // public (no auth required) so without this an attacker can enumerate
    // valid tokens / amplify DOS by issuing many lookups. Identity key falls
    // back to "unknown" if the platform layer can't expose a remote IP.
    let identity = {
        let addr = msg.remote_addr();
        if addr.is_empty() {
            "unknown".to_string()
        } else {
            addr.to_string()
        }
    };
    match check_rate_limit(limiter, ctx, &identity, "share_direct", RateLimit::API_READ).await {
        RateLimitOutcome::Limited(r) => return r,
        // Allowed headers can't be attached to a binary file response here —
        // accept this as a known limitation; the platform layer would need
        // streaming-meta middleware to inject them.
        RateLimitOutcome::Allowed(_) | RateLimitOutcome::Disabled => {}
    }

    // Verify the token's HMAC before touching the DB. Tokens are JWT-signed
    // at issue time (`generate_share_token`), so an invalid signature means
    // the token wasn't minted by us — short-circuit before the DB lookup so
    // attackers can't enumerate the SHARES_TABLE via random tokens.
    if crypto::verify(ctx, token).await.is_err() {
        return err_not_found("Share not found or expired");
    }

    // Look up share by token
    let share = match db::get_by_field(
        ctx,
        SHARES_TABLE,
        "token",
        serde_json::Value::String(token.to_string()),
    )
    .await
    {
        Ok(s) => s,
        Err(_) => return err_not_found("Share not found or expired"),
    };

    // Check expiry
    if let Some(expires) = share.data.get("expires_at").and_then(|v| v.as_str()) {
        if !expires.is_empty() {
            if let Ok(exp_time) = chrono::DateTime::parse_from_rfc3339(expires) {
                if exp_time < chrono::Utc::now() {
                    return err_forbidden("Share link has expired");
                }
            }
        }
    }

    // Atomic access-count increment + cap enforcement via a CAS UPDATE:
    //   UPDATE shares SET access_count = access_count + 1
    //   WHERE id = ? AND access_count < max_access_count
    // The read-then-write pattern previously here let two concurrent
    // accesses with `max_access_count = 1` both pass the check and double-
    // serve the file. With the cap inside the WHERE clause, at most one
    // updater wins per row and rowcount 0 ⇒ cap reached.
    let max = share
        .data
        .get("max_access_count")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    match increment_access_count_atomic(ctx, &share.id, max).await {
        Ok(true) => {}
        Ok(false) => return err_forbidden("Share link access limit reached"),
        Err(e) => {
            // Don't block a legitimate access on a transient DB blip — log and
            // continue. Counters drifting low is preferable to denying paid-
            // for downloads. (The cap check above ran on a stale read but
            // covers the common case.)
            tracing::warn!(error = %e, share_id = %share.id, "Failed to increment share access count");
        }
    }

    let bucket = share
        .data
        .get("bucket")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let key = share.data.get("key").and_then(|v| v.as_str()).unwrap_or("");

    if bucket.is_empty() || key.is_empty() {
        return err_internal_no_cause("Invalid share data");
    }

    // Log access
    let mut log_data = HashMap::new();
    log_data.insert(
        "share_id".to_string(),
        serde_json::Value::String(share.id.clone()),
    );
    log_data.insert(
        "accessed_at".to_string(),
        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
    );
    log_data.insert(
        "ip_address".to_string(),
        serde_json::Value::String(msg.remote_addr().to_string()),
    );
    log_data.insert(
        "user_agent".to_string(),
        serde_json::Value::String(msg.header("User-Agent").to_string()),
    );
    if let Err(e) = db::create(ctx, ACCESS_LOGS_TABLE, log_data).await {
        tracing::warn!("Failed to log share access: {e}");
    }

    // Serve the file
    match store::get(ctx, bucket, key).await {
        Ok((data, info)) => ResponseBuilder::new()
            .set_header(
                "Content-Disposition",
                &format!(
                    "inline; filename=\"{}\"",
                    key.replace(['"', '\n', '\r'], "")
                ),
            )
            .set_header("Cache-Control", "private, max-age=3600")
            .body(data, &info.content_type),
        Err(e) if e.code == ErrorCode::NotFound => err_not_found("File not found"),
        Err(e) => err_internal("Storage error", e),
    }
}

/// CAS-style increment of `access_count` for a share row. Returns `Ok(true)`
/// if a row was updated (and the cap, if any, still allowed the access),
/// `Ok(false)` if the row was already at its cap, or `Err` on DB failure.
///
/// `max <= 0` means unlimited — we only filter on id. Otherwise we add
/// `access_count < max` to the WHERE so two concurrent accesses can't both
/// pass a 1-access cap.
async fn increment_access_count_atomic(
    ctx: &dyn Context,
    share_id: &str,
    max: i64,
) -> Result<bool, wafer_run::WaferError> {
    use wafer_core::interfaces::database::service::{Filter, FilterOp};
    use wafer_sql_utils::{value::sea_values_to_json, Backend};

    let mut filters: Vec<Filter> = vec![Filter {
        field: "id".into(),
        operator: FilterOp::Equal,
        value: serde_json::Value::String(share_id.to_string()),
    }];
    if max > 0 {
        filters.push(Filter {
            field: "access_count".into(),
            operator: FilterOp::LessThan,
            value: serde_json::json!(max),
        });
    }
    let (sql, vals) = wafer_sql_utils::query::build_increment_field_where(
        SHARES_TABLE,
        "access_count",
        1,
        &filters,
        Backend::Sqlite,
    );
    let args = sea_values_to_json(vals);
    let rows = db::exec_raw(ctx, &sql, &args).await?;
    Ok(rows > 0)
}
