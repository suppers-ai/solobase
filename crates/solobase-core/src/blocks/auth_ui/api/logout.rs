//! POST /b/auth/api/logout — relocated from auth/login.rs in Task 5.

use wafer_core::clients::database as db;
use wafer_run::{context::Context, types::Message, OutputStream};

use crate::{
    blocks::{
        auth::{
            helpers::build_auth_cookie,
            repo::jwt_blocklist::{self, NewBlocklistEntry},
            TOKENS_TABLE,
        },
        helpers::ResponseBuilder,
    },
    crypto::{META_AUTH_EXP, META_AUTH_JTI},
};

pub async fn handle(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let user_id = msg.user_id();
    if !user_id.is_empty() {
        // Refresh tokens are wiped wholesale for the user — anything stored
        // pre-PR-J as raw tokens (or post-PR-J as hashes) is dropped here.
        db::delete_by_field(
            ctx,
            TOKENS_TABLE,
            "user_id",
            serde_json::Value::String(user_id.to_string()),
        )
        .await
        .ok();

        // SEC-042: the currently-presented access JWT stays structurally
        // valid until its natural exp. Blocklist its `jti` so subsequent
        // requests with the same token are rejected by `extract_auth_meta`.
        //
        // Only the in-flight JWT is blocklisted (per-jti, not per-user) so
        // other live sessions for the same user are unaffected.
        let jti = msg.get_meta(META_AUTH_JTI);
        let exp_str = msg.get_meta(META_AUTH_EXP);
        if !jti.is_empty() {
            // Convert exp (UNIX seconds) to ISO-8601 so we can prune by
            // string comparison consistent with other auth tables. Fall
            // back to "now + 1 day" if exp is missing/unparseable — the
            // blocklist row's only job is to outlive the JWT itself, so
            // a generous fallback is fine.
            let expires_at = exp_str
                .parse::<i64>()
                .ok()
                .and_then(|secs| chrono::DateTime::from_timestamp(secs, 0))
                .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::days(1));
            let expires_at_iso = expires_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            let _ = jwt_blocklist::insert(
                ctx,
                NewBlocklistEntry {
                    jti,
                    user_id,
                    expires_at: &expires_at_iso,
                },
            )
            .await;
        }
    }

    let cookie = build_auth_cookie("", 0, ctx).await;
    ResponseBuilder::new()
        .set_cookie(&cookie)
        .status(303)
        .set_header("Location", "/b/auth/login")
        .json(&serde_json::json!({"message": "Logged out successfully"}))
}
