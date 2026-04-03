mod custom_tables;
mod database;
mod iam;
mod logs;
mod pages;
mod settings;
mod users;
mod wafer_info;

use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

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
        use wafer_run::AuthLevel;

        BlockInfo::new("suppers-ai/admin", "0.0.1", "http-handler@v1", "Admin panel: users, database, IAM, logs, settings, wafer introspection, custom tables")
            .instance_mode(InstanceMode::Singleton)
            .requires(vec!["wafer-run/database".into(), "wafer-run/config".into()])
            .collections(vec![
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
                CollectionSchema::new("request_logs")
                    .field_default("flow_id", "string", "")
                    .field_default("method", "string", "")
                    .field_default("path", "string", "")
                    .field_default("status", "string", "")
                    .field_default("status_code", "int", "0")
                    .field_default("duration_ms", "int", "0")
                    .field_default("error_message", "string", "")
                    .field_default("client_ip", "string", "")
                    .field_default("user_id", "string", "")
                    .index(&["created_at"]),
                CollectionSchema::new("block_settings")
                    .field_unique("block_name", "string")
                    .field_default("enabled", "int", "1"),
            ])
            .category(wafer_run::BlockCategory::Feature)
            .description("Administration panel for managing users, roles, variables, blocks, and logs. Provides SSR dashboard with stats, user management with role assignment, IAM (roles and API keys), environment variables editor, block management with feature toggles, and system/audit log viewer.")
            .endpoints(vec![
                BlockEndpoint::get("/b/admin/", "Dashboard", AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/users", "User management", AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/variables", "Variables editor", AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/blocks", "Block management", AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/logs", "System and audit logs", AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/api/users", "List users API", AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/api/iam/roles", "List roles API", AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/api/settings", "List variables API", AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/api/logs", "Audit logs API", AuthLevel::Admin),
            ])
    }

    fn ui_routes(&self) -> Vec<wafer_run::UiRoute> {
        vec![
            wafer_run::UiRoute::admin("/"),
            wafer_run::UiRoute::admin("/users"),
            wafer_run::UiRoute::admin("/variables"),
            wafer_run::UiRoute::admin("/blocks"),
            wafer_run::UiRoute::admin("/logs"),
        ]
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path().to_string();

        // JSON API at /b/admin/api/... — normalize to /admin/... for sub-module compatibility
        if let Some(api_rest) = path.strip_prefix("/b/admin/api") {
            let normalized = format!("/admin{}", api_rest);
            msg.set_meta("req.resource", &normalized);

            if api_rest.starts_with("/users") {
                return users::handle(ctx, msg).await;
            }
            if api_rest.starts_with("/database") {
                return database::handle(ctx, msg).await;
            }
            if api_rest.starts_with("/iam") {
                return iam::handle(ctx, msg).await;
            }
            if api_rest.starts_with("/logs") {
                return logs::handle(ctx, msg).await;
            }
            if api_rest.starts_with("/settings") {
                return settings::handle(ctx, msg).await;
            }
            if api_rest.starts_with("/extensions") {
                let blocks: Vec<_> = ctx
                    .registered_blocks()
                    .into_iter()
                    .map(|b| {
                        serde_json::json!({
                            "name": b.name,
                            "version": b.version,
                            "interface": b.interface,
                            "summary": b.summary,
                            "enabled": true,
                        })
                    })
                    .collect();
                return wafer_run::helpers::json_respond(msg, &blocks);
            }
            if api_rest.starts_with("/wafer") {
                return wafer_info::handle(ctx, msg);
            }
            if api_rest.starts_with("/custom-tables") {
                return custom_tables::handle(ctx, msg).await;
            }
            // Delegate admin storage to Files block
            if api_rest.starts_with("/storage") {
                msg.set_meta("req.resource", format!("/admin{}", api_rest));
                return crate::blocks::files::handle_admin_storage(ctx, msg).await;
            }
            // Delegate admin cloud storage to Files block
            if api_rest.starts_with("/cloudstorage") {
                msg.set_meta(
                    "req.resource",
                    format!(
                        "/admin/b/cloudstorage{}",
                        api_rest.strip_prefix("/cloudstorage").unwrap_or("")
                    ),
                );
                return crate::blocks::files::handle_admin_cloud(ctx, msg).await;
            }
            return err_not_found(msg, "not found");
        }

        // SSR pages + htmx mutations at /b/admin/...
        if path.starts_with("/b/admin") {
            let action = msg.action().to_string();
            let sub = path.strip_prefix("/b/admin").unwrap_or("/");

            // Extract IDs upfront as owned strings (avoids borrow conflicts with msg)
            let sub = sub.to_string();

            // htmx mutation handlers
            if action == "create" && sub.ends_with("/disable") {
                let user_id = sub
                    .strip_prefix("/users/")
                    .and_then(|s| s.strip_suffix("/disable"))
                    .unwrap_or("")
                    .to_string();
                if !user_id.is_empty() {
                    return pages::handle_user_disable(ctx, msg, &user_id).await;
                }
            }
            if action == "create" && sub.ends_with("/enable") {
                let user_id = sub
                    .strip_prefix("/users/")
                    .and_then(|s| s.strip_suffix("/enable"))
                    .unwrap_or("")
                    .to_string();
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
            // Block detail modal
            if action == "retrieve" && sub.starts_with("/blocks/") && sub.ends_with("/detail") {
                let encoded = sub
                    .strip_prefix("/blocks/")
                    .and_then(|s| s.strip_suffix("/detail"))
                    .unwrap_or("")
                    .to_string();
                let block_name = encoded.replace("--", "/");
                if !block_name.is_empty() {
                    return pages::handle_block_detail(ctx, msg, &block_name).await;
                }
            }
            // Block feature toggle
            if action == "create" && sub.starts_with("/blocks/") && sub.ends_with("/toggle") {
                let encoded = sub
                    .strip_prefix("/blocks/")
                    .and_then(|s| s.strip_suffix("/toggle"))
                    .unwrap_or("")
                    .to_string();
                let block_name = encoded.replace("--", "/");
                if !block_name.is_empty() {
                    return pages::handle_toggle_feature(ctx, msg, &block_name).await;
                }
            }
            // Variable mutations
            if action == "create" && sub == "/variables" {
                return pages::handle_create_variable(ctx, msg).await;
            }
            if action == "retrieve" && sub.ends_with("/edit") && sub.starts_with("/variables/") {
                let var_key = sub
                    .strip_prefix("/variables/")
                    .and_then(|s| s.strip_suffix("/edit"))
                    .unwrap_or("")
                    .to_string();
                if !var_key.is_empty() {
                    return pages::handle_edit_variable_form(ctx, msg, &var_key).await;
                }
            }
            if action == "update" && sub.starts_with("/variables/") {
                let var_key = sub.strip_prefix("/variables/").unwrap_or("").to_string();
                if !var_key.is_empty() {
                    return pages::handle_update_variable(ctx, msg, &var_key).await;
                }
            }

            // SSR page handlers (GET)
            return match sub.as_str() {
                "" | "/" => pages::dashboard(ctx, msg).await,
                "/users" => pages::users_page(ctx, msg).await,
                "/variables" => pages::variables_page(ctx, msg).await,
                "/blocks" => pages::blocks_page(ctx, msg).await,
                "/logs" => pages::logs_page(ctx, msg).await,
                _ => err_not_found(msg, "not found"),
            };
        }

        err_not_found(msg, "not found")
    }

    async fn lifecycle(
        &self,
        ctx: &dyn Context,
        event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        if matches!(event.event_type, LifecycleType::Init) {
            iam::seed_defaults(ctx).await;
            settings::seed_defaults(ctx).await;
        }
        Ok(())
    }
}
