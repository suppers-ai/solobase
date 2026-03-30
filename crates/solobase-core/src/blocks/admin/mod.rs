mod users;
mod database;
mod iam;
mod logs;
mod pages;
mod settings;
mod wafer_info;
mod custom_tables;

use wafer_run::block::{Block, BlockInfo, AdminUIInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;

/// Sanitize an identifier to prevent SQL injection. Only allows
/// alphanumeric characters and underscores.
pub(crate) fn sanitize_ident(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

pub struct AdminBlock;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for AdminBlock {
    fn info(&self) -> BlockInfo {
        use wafer_run::types::CollectionSchema;

        BlockInfo {
            name: "suppers-ai/admin".to_string(),
            version: "1.0.0".to_string(),
            interface: "http-handler@v1".to_string(),
            summary: "Admin panel: users, database, IAM, logs, settings, wafer introspection, custom tables".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: Some(AdminUIInfo {
                label: "Admin".to_string(),
                description: "Users, database, storage, settings, and blocks".to_string(),
                url: "/b/admin/".to_string(),
            }),
            runtime: wafer_run::types::BlockRuntime::Native,
            requires: Vec::new(),
            collections: vec![
                CollectionSchema::new("iam_roles")
                    .field("name", "string")
                    .field_default("description", "string", "")
                    .field_default("permissions", "json", "[]")
                    .field_default("is_system", "bool", "false"),
                CollectionSchema::new("iam_permissions")
                    .field("name", "string")
                    .field_default("resource", "string", "")
                    .field_default("actions", "json", "[]"),
                CollectionSchema::new("iam_user_roles")
                    .field_ref("user_id", "string", "auth_users.id")
                    .field("role", "string")
                    .field_optional("assigned_at", "datetime")
                    .field_default("assigned_by", "string", "")
                    .index(&["user_id"]),
                CollectionSchema::new("variables")
                    .field_unique("key", "string")
                    .field_default("name", "string", "")
                    .field_default("description", "string", "")
                    .field_default("value", "string", "")
                    .field_default("warning", "string", "")
                    .field_default("sensitive", "int", "0")
                    .field_default("updated_by", "string", ""),
                CollectionSchema::new("audit_logs")
                    .field_default("user_id", "string", "")
                    .field("action", "string")
                    .field_default("resource", "string", "")
                    .field_default("ip_address", "string", "")
                    .index(&["created_at"]),
            ],
            config_schema: None,
        }
    }

    fn ui_routes(&self) -> Vec<wafer_run::UiRoute> {
        vec![
            wafer_run::UiRoute::admin("/"),
            wafer_run::UiRoute::admin("/users"),
            wafer_run::UiRoute::admin("/iam"),
            wafer_run::UiRoute::admin("/settings"),
            wafer_run::UiRoute::admin("/logs"),
        ]
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path().to_string();

        // SSR pages + htmx mutations at /b/admin/...
        if path.starts_with("/b/admin") {
            let action = msg.action().to_string();
            let sub = path.strip_prefix("/b/admin").unwrap_or("/");

            // Extract IDs upfront as owned strings (avoids borrow conflicts with msg)
            let sub = sub.to_string();

            // htmx mutation handlers
            if action == "create" && sub.ends_with("/disable") {
                let user_id = sub.strip_prefix("/users/").and_then(|s| s.strip_suffix("/disable")).unwrap_or("").to_string();
                if !user_id.is_empty() {
                    return pages::handle_user_disable(ctx, msg, &user_id).await;
                }
            }
            if action == "create" && sub.ends_with("/enable") {
                let user_id = sub.strip_prefix("/users/").and_then(|s| s.strip_suffix("/enable")).unwrap_or("").to_string();
                if !user_id.is_empty() {
                    return pages::handle_user_enable(ctx, msg, &user_id).await;
                }
            }
            if action == "delete" && sub.starts_with("/users/") {
                let user_id = sub.strip_prefix("/users/").unwrap_or("").to_string();
                if !user_id.is_empty() {
                    return pages::handle_user_delete(ctx, msg, &user_id).await;
                }
            }
            if action == "create" && sub == "/iam/roles" {
                return pages::handle_create_role(ctx, msg).await;
            }
            if action == "delete" && sub.starts_with("/iam/roles/") {
                let role_id = sub.strip_prefix("/iam/roles/").unwrap_or("").to_string();
                if !role_id.is_empty() {
                    return pages::handle_delete_role(ctx, msg, &role_id).await;
                }
            }

            // SSR page handlers (GET)
            return match sub.as_str() {
                "" | "/" => pages::dashboard(ctx, msg).await,
                "/users" => pages::users_page(ctx, msg).await,
                "/iam" => pages::iam_page(ctx, msg).await,
                "/settings" => pages::settings_page(ctx, msg).await,
                "/logs" => pages::logs_page(ctx, msg).await,
                _ => err_not_found(msg, "not found"),
            };
        }

        // Existing API routes
        if path.starts_with("/admin/users") {
            return users::handle(ctx, msg).await;
        }
        if path.starts_with("/admin/database") {
            return database::handle(ctx, msg).await;
        }
        if path.starts_with("/admin/iam") {
            return iam::handle(ctx, msg).await;
        }
        if path.starts_with("/admin/logs") {
            return logs::handle(ctx, msg).await;
        }
        if path.starts_with("/admin/settings") || path.starts_with("/settings") {
            return settings::handle(ctx, msg).await;
        }
        if path.starts_with("/admin/extensions") {
            let blocks: Vec<_> = ctx.registered_blocks().into_iter().map(|b| {
                let mut entry = serde_json::json!({
                    "name": b.name,
                    "version": b.version,
                    "interface": b.interface,
                    "summary": b.summary,
                    "runtime": format!("{:?}", b.runtime),
                    "enabled": true,
                });
                if let Some(ui) = &b.admin_ui {
                    entry["admin_ui"] = serde_json::json!({
                        "label": ui.label,
                        "url": ui.url,
                    });
                }
                entry
            }).collect();
            return wafer_run::helpers::json_respond(msg, &blocks);
        }
        if path.starts_with("/admin/wafer") {
            return wafer_info::handle(ctx, msg);
        }
        if path.starts_with("/admin/custom-tables") {
            return custom_tables::handle(ctx, msg).await;
        }

        err_not_found(msg, "not found")
    }

    async fn lifecycle(&self, ctx: &dyn Context, event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        if matches!(event.event_type, LifecycleType::Init) {
            iam::seed_defaults(ctx).await;
            settings::seed_defaults(ctx).await;
        }
        Ok(())
    }
}
