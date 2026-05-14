//! /b/auth/api/api-keys — relocated from auth/api_keys.rs in Task 5.
//!
//! Admin user-management still calls these routes via htmx (see
//! `solobase-core/src/blocks/admin/pages/users.rs`). PAT migration is a
//! follow-up; for PR 5 we relocate rather than delete.

use std::collections::HashMap;

use wafer_core::clients::{
    crypto, database as db,
    database::{Filter, FilterOp, ListOptions, SortField},
};
use wafer_run::{context::Context, types::Message, InputStream, OutputStream};

use crate::blocks::{
    auth::API_KEYS_TABLE,
    helpers::{
        self, err_bad_request, err_forbidden, err_internal, err_not_found, hex_encode, ok_json,
        sha256_hex, RecordExt,
    },
};

pub async fn handle_list(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let user_id = msg.user_id();
    if user_id.is_empty() {
        return crate::blocks::errors::error_response(
            crate::blocks::errors::ErrorCode::NotAuthenticated,
            "Authentication required",
        );
    }
    let opts = ListOptions {
        filters: vec![Filter {
            field: "user_id".to_string(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.to_string()),
        }],
        sort: vec![SortField {
            field: "created_at".to_string(),
            desc: true,
        }],
        limit: 100,
        ..Default::default()
    };
    match db::list(ctx, API_KEYS_TABLE, &opts).await {
        Ok(mut result) => {
            // Strip key_hash from response
            for record in &mut result.records {
                record.data.remove("key_hash");
            }
            ok_json(&result)
        }
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

pub async fn handle_create(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    let user_id = msg.user_id();
    if user_id.is_empty() {
        return crate::blocks::errors::error_response(
            crate::blocks::errors::ErrorCode::NotAuthenticated,
            "Authentication required",
        );
    }

    #[derive(serde::Deserialize)]
    struct CreateKeyReq {
        name: String,
        expires_at: Option<String>,
    }
    let raw = input.collect_to_bytes().await;
    let parsed = crate::blocks::helpers::parse_body_value(&raw);
    let body: CreateKeyReq = match serde_json::from_value(parsed) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };
    if body.name.is_empty() {
        return err_bad_request("API key name is required");
    }

    // Generate random key
    let random_bytes = match crypto::random_bytes(ctx, 24).await {
        Ok(b) => b,
        Err(e) => return err_internal(&format!("Failed to generate key: {e}")),
    };
    let key_string = format!("sb_{}", hex_encode(&random_bytes));

    // Use deterministic SHA-256 hash for key lookup (not argon2, which is non-deterministic)
    let key_hash = sha256_hex(key_string.as_bytes());

    let now = crate::blocks::helpers::now_rfc3339();
    let mut data = HashMap::new();
    data.insert(
        "user_id".to_string(),
        serde_json::Value::String(user_id.to_string()),
    );
    data.insert("name".to_string(), serde_json::Value::String(body.name));
    data.insert("key_hash".to_string(), serde_json::Value::String(key_hash));
    data.insert(
        "key_prefix".to_string(),
        serde_json::Value::String(key_string[..10].to_string()),
    );
    data.insert("created_at".to_string(), serde_json::Value::String(now));
    if let Some(exp) = body.expires_at {
        data.insert("expires_at".to_string(), serde_json::Value::String(exp));
    }

    match db::create(ctx, API_KEYS_TABLE, data).await {
        Ok(record) => {
            // htmx form callers want HTML back so the swap renders cleanly.
            // Programmatic JSON callers (no HX-Request header) get the JSON
            // payload as before so existing API consumers don't break.
            if !msg.get_meta("http.header.hx-request").is_empty() {
                let key_for_display = key_string.clone();
                let name = record.str_field("name").to_string();
                // Inline JS handler for the copy button. The key text lives
                // in #new-api-key — read `innerText` (not the JS string) so
                // we never have to escape the key into a JS literal, and so
                // the button works even if the swap re-renders without the
                // original closure scope.
                let copy_js = "\
                    var el=document.getElementById('new-api-key');\
                    var t=el?el.innerText:'';\
                    if(t&&navigator.clipboard){\
                        navigator.clipboard.writeText(t).then(function(){\
                            var b=event.currentTarget;b.textContent='Copied';\
                            setTimeout(function(){b.textContent='Copy'},1500);\
                        });\
                    }";
                let markup = maud::html! {
                    div .card style="margin-bottom: var(--spacing-md)" {
                        div .card__head { h3 .card__title { "Key created — save it now" } }
                        div .card__body {
                            p style="margin:0 0 var(--spacing-sm); font-size: 13px; color: var(--text-secondary)" {
                                "This is the only time the full key will be shown. Copy it now."
                            }
                            div style="display:flex; gap: var(--spacing-sm); align-items: stretch" {
                                code #new-api-key style="flex:1 1 auto; padding: var(--spacing-sm); background: var(--bg-secondary); border-radius: var(--radius-md); font-family: ui-monospace, Menlo, monospace; font-size: 13px; word-break: break-all; user-select: all" {
                                    (key_for_display)
                                }
                                button type="button" .btn .btn-secondary .btn-sm
                                    style="flex: 0 0 auto"
                                    onclick=(copy_js)
                                { "Copy" }
                            }
                            p style="margin: var(--spacing-sm) 0 0; font-size: var(--text-xs); color: var(--text-muted)" {
                                "Name: " (name)
                            }
                        }
                    }
                };
                let trigger = r#"{"showToast":{"message":"API key created","type":"success"},"closeModal":{"id":"create-api-key"}}"#;
                crate::blocks::helpers::ResponseBuilder::new()
                    .set_header("HX-Trigger", trigger)
                    .body(
                        markup.into_string().into_bytes(),
                        "text/html; charset=utf-8",
                    )
            } else {
                ok_json(&serde_json::json!({
                    "id": record.id,
                    "key": key_string,
                    "name": record.str_field("name"),
                    "key_prefix": record.str_field("key_prefix"),
                    "message": "Save this key — it won't be shown again"
                }))
            }
        }
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

pub async fn handle_revoke(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    let id = path.rsplit_once('/').map(|(_, id)| id).unwrap_or("");
    if id.is_empty() {
        return err_bad_request("Missing key ID");
    }
    let user_id = msg.user_id();

    // Verify ownership
    let key = match db::get(ctx, API_KEYS_TABLE, id).await {
        Ok(k) => k,
        Err(_) => return err_not_found("API key not found"),
    };
    let key_owner = key.str_field("user_id");
    if key_owner != user_id && !helpers::is_admin(msg) {
        return err_forbidden("Cannot revoke another user's API key");
    }

    let data = crate::blocks::helpers::json_map(
        serde_json::json!({"revoked_at": crate::blocks::helpers::now_rfc3339()}),
    );
    match db::update(ctx, API_KEYS_TABLE, id, data).await {
        Ok(_) => ok_json(&serde_json::json!({"message": "API key revoked"})),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}

pub async fn handle_delete(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let path = msg.path();
    let id = path.rsplit_once('/').map(|(_, id)| id).unwrap_or("");
    if id.is_empty() {
        return err_bad_request("Missing key ID");
    }
    let user_id = msg.user_id();

    // Verify ownership
    let key = match db::get(ctx, API_KEYS_TABLE, id).await {
        Ok(k) => k,
        Err(_) => return err_not_found("API key not found"),
    };
    let key_owner = key.str_field("user_id");
    if key_owner != user_id && !helpers::is_admin(msg) {
        return err_forbidden("Cannot delete another user's API key");
    }

    match db::delete(ctx, API_KEYS_TABLE, id).await {
        Ok(_) => ok_json(&serde_json::json!({"deleted": true})),
        Err(e) => err_internal(&format!("Database error: {e}")),
    }
}
