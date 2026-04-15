use super::AuthBlock;
use super::API_KEYS_COLLECTION;
use crate::blocks::helpers::{
    err_bad_request, err_forbidden, err_internal, err_not_found, hex_encode, ok_json, sha256_hex,
    RecordExt,
};
use std::collections::HashMap;
use wafer_core::clients::crypto;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::{InputStream, OutputStream};

impl AuthBlock {
    pub(super) async fn handle_api_keys_list(
        &self,
        ctx: &dyn Context,
        msg: &Message,
    ) -> OutputStream {
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
        match db::list(ctx, API_KEYS_COLLECTION, &opts).await {
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

    pub(super) async fn handle_api_keys_create(
        &self,
        ctx: &dyn Context,
        msg: &Message,
        input: InputStream,
    ) -> OutputStream {
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
        let body: CreateKeyReq = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
        };

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

        match db::create(ctx, API_KEYS_COLLECTION, data).await {
            Ok(record) => ok_json(&serde_json::json!({
                "id": record.id,
                "key": key_string,
                "name": record.str_field("name"),
                "key_prefix": record.str_field("key_prefix"),
                "message": "Save this key — it won't be shown again"
            })),
            Err(e) => err_internal(&format!("Database error: {e}")),
        }
    }

    pub(super) async fn handle_api_keys_revoke(
        &self,
        ctx: &dyn Context,
        msg: &Message,
    ) -> OutputStream {
        let path = msg.path();
        let id = path.rsplit_once('/').map(|(_, id)| id).unwrap_or("");
        if id.is_empty() {
            return err_bad_request("Missing key ID");
        }
        let user_id = msg.user_id();

        // Verify ownership
        let key = match db::get(ctx, API_KEYS_COLLECTION, id).await {
            Ok(k) => k,
            Err(_) => return err_not_found("API key not found"),
        };
        let key_owner = key.str_field("user_id");
        if key_owner != user_id
            && !msg
                .get_meta("auth.user_roles")
                .split(',')
                .any(|r| r.trim() == "admin")
        {
            return err_forbidden("Cannot revoke another user's API key");
        }

        let data = crate::blocks::helpers::json_map(
            serde_json::json!({"revoked_at": crate::blocks::helpers::now_rfc3339()}),
        );
        match db::update(ctx, API_KEYS_COLLECTION, id, data).await {
            Ok(_) => ok_json(&serde_json::json!({"message": "API key revoked"})),
            Err(e) => err_internal(&format!("Database error: {e}")),
        }
    }

    pub(super) async fn handle_api_keys_delete(
        &self,
        ctx: &dyn Context,
        msg: &Message,
    ) -> OutputStream {
        let path = msg.path();
        let id = path.rsplit_once('/').map(|(_, id)| id).unwrap_or("");
        if id.is_empty() {
            return err_bad_request("Missing key ID");
        }
        let user_id = msg.user_id();

        // Verify ownership
        let key = match db::get(ctx, API_KEYS_COLLECTION, id).await {
            Ok(k) => k,
            Err(_) => return err_not_found("API key not found"),
        };
        let key_owner = key.str_field("user_id");
        if key_owner != user_id
            && !msg
                .get_meta("auth.user_roles")
                .split(',')
                .any(|r| r.trim() == "admin")
        {
            return err_forbidden("Cannot delete another user's API key");
        }

        match db::delete(ctx, API_KEYS_COLLECTION, id).await {
            Ok(_) => ok_json(&serde_json::json!({"deleted": true})),
            Err(e) => err_internal(&format!("Database error: {e}")),
        }
    }
}
