mod custom_tables;
mod database;
mod iam;
mod logs;
mod pages;
mod settings;
mod users;
mod wafer_info;

pub(crate) const ROLES_COLLECTION: &str = "suppers_ai__admin__roles";
pub(crate) const PERMISSIONS_COLLECTION: &str = "suppers_ai__admin__permissions";
pub(crate) const USER_ROLES_COLLECTION: &str = "suppers_ai__admin__user_roles";
pub(crate) const AUDIT_LOGS_COLLECTION: &str = "suppers_ai__admin__audit_logs";
pub(crate) const REQUEST_LOGS_COLLECTION: &str = "suppers_ai__admin__request_logs";
pub(crate) const NETWORK_REQUEST_LOGS_COLLECTION: &str = "suppers_ai__admin__network_request_logs";
pub(crate) const STORAGE_ACCESS_LOGS_COLLECTION: &str = "suppers_ai__admin__storage_access_logs";
pub(crate) const STORAGE_RULES_COLLECTION: &str = "suppers_ai__admin__storage_rules";
pub(crate) const BLOCK_SETTINGS_COLLECTION: &str = "suppers_ai__admin__block_settings";
pub(crate) const VARIABLES_COLLECTION: &str = "suppers_ai__admin__variables";
pub(crate) const NETWORK_RULES_COLLECTION: &str = "suppers_ai__admin__network_rules";
pub(crate) const WRAP_GRANTS_COLLECTION: &str = "suppers_ai__admin__wrap_grants";

use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::{InputStream, OutputStream};

use crate::blocks::helpers::{err_not_found, ok_json};

pub(crate) use wafer_sql_utils::ident::sanitize_ident;

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
                CollectionSchema::new(ROLES_COLLECTION)
                    .field("name", "string")
                    .field_default("description", "string", "")
                    .field_default("permissions", "json", "[]")
                    .field_default("is_system", "bool", "false"),
                CollectionSchema::new(PERMISSIONS_COLLECTION)
                    .field("name", "string")
                    .field_default("resource", "string", "")
                    .field_default("actions", "json", "[]"),
                CollectionSchema::new(USER_ROLES_COLLECTION)
                    .field_ref("user_id", "string", &format!("{}.id", crate::blocks::auth::USERS_COLLECTION))
                    .field("role", "string")
                    .field_optional("assigned_at", "datetime")
                    .field_default("assigned_by", "string", "")
                    .index(&["user_id"]),
                CollectionSchema::new(VARIABLES_COLLECTION)
                    .field_unique("key", "string")
                    .field_default("name", "string", "")
                    .field_default("description", "string", "")
                    .field_default("value", "string", "")
                    .field_default("warning", "string", "")
                    .field_default("sensitive", "int", "0")
                    .field_default("updated_by", "string", ""),
                CollectionSchema::new(AUDIT_LOGS_COLLECTION)
                    .field_default("user_id", "string", "")
                    .field("action", "string")
                    .field_default("resource", "string", "")
                    .field_default("ip_address", "string", "")
                    .index(&["created_at"]),
                CollectionSchema::new(REQUEST_LOGS_COLLECTION)
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
                CollectionSchema::new(NETWORK_REQUEST_LOGS_COLLECTION)
                    .field_default("source_block", "string", "")
                    .field_default("method", "string", "")
                    .field_default("url", "string", "")
                    .field_default("status_code", "int", "0")
                    .field_default("duration_ms", "int", "0")
                    .field_default("error_message", "string", "")
                    .index(&["created_at"]),
                CollectionSchema::new(STORAGE_ACCESS_LOGS_COLLECTION)
                    .field_default("source_block", "string", "")
                    .field_default("operation", "string", "")
                    .field_default("path", "string", "")
                    .field_default("status", "string", "")
                    .index(&["created_at"]),
                CollectionSchema::new(BLOCK_SETTINGS_COLLECTION)
                    .field_unique("block_name", "string")
                    .field_default("enabled", "int", "1"),
                CollectionSchema::new(WRAP_GRANTS_COLLECTION)
                    .field("grantee", "string")
                    .field("resource", "string")
                    .field_default("write", "int", "0")
                    .field_default("resource_type", "string", "")
                    .field_default("description", "string", ""),
            ])
            .grants(vec![
                wafer_run::ResourceGrant::read_write(super::auth::AUTH_BLOCK_ID, USER_ROLES_COLLECTION),
                wafer_run::ResourceGrant::read(super::auth::AUTH_BLOCK_ID, VARIABLES_COLLECTION),
                wafer_run::ResourceGrant::read("suppers-ai/userportal", BLOCK_SETTINGS_COLLECTION),
                // Infrastructure logging: network/storage wrappers + pipeline write logs
                wafer_run::ResourceGrant::read_write("*", NETWORK_REQUEST_LOGS_COLLECTION),
                wafer_run::ResourceGrant::read_write("*", STORAGE_ACCESS_LOGS_COLLECTION),
                wafer_run::ResourceGrant::read_write("*", REQUEST_LOGS_COLLECTION),
                // Default: allow all blocks to make outbound network requests.
                // Remove this grant via the admin UI to restrict network access.
                wafer_run::ResourceGrant::read("*", "*")
                    .typed(wafer_run::types::ResourceType::Network),
            ])
            .category(wafer_run::BlockCategory::Feature)
            .description("Administration panel for managing users, roles, variables, blocks, and logs. Provides SSR dashboard with stats, user management with role assignment, IAM (roles and API keys), environment variables editor, block management with feature toggles, and system/audit log viewer.")
            .endpoints(vec![
                BlockEndpoint::get("/b/admin/").summary("Dashboard").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/users").summary("User management").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/variables").summary("Config management").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/blocks").summary("Block management").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/network").summary("Network monitoring and rules").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/storage").summary("Storage isolation and rules").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/logs").summary("System and audit logs").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/email").summary("Email settings").auth(AuthLevel::Admin),
                BlockEndpoint::post("/b/admin/email").summary("Save email settings").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/permissions").summary("Permissions management").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/grants").summary("WRAP grants management").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/api/users").summary("List users API").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/api/iam/roles").summary("List roles API").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/api/settings").summary("List variables API").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/api/logs").summary("Audit logs API").auth(AuthLevel::Admin),
                BlockEndpoint::post("/b/admin/custom-blocks/install").summary("Install custom block from registry").auth(AuthLevel::Admin),
                BlockEndpoint::post("/b/admin/custom-blocks/upload").summary("Upload custom .wasm block").auth(AuthLevel::Admin),
                BlockEndpoint::delete("/b/admin/custom-blocks/{name}").summary("Delete custom block").auth(AuthLevel::Admin),
            ])
    }

    fn ui_routes(&self) -> Vec<wafer_run::UiRoute> {
        vec![
            wafer_run::UiRoute::admin("/"),
            wafer_run::UiRoute::admin("/users"),
            wafer_run::UiRoute::admin("/variables"),
            wafer_run::UiRoute::admin("/network"),
            wafer_run::UiRoute::admin("/blocks"),
            wafer_run::UiRoute::admin("/logs"),
            wafer_run::UiRoute::admin("/email"),
            wafer_run::UiRoute::admin("/permissions"),
            wafer_run::UiRoute::admin("/grants"),
        ]
    }

    async fn handle(
        &self,
        ctx: &dyn Context,
        mut msg: Message,
        input: InputStream,
    ) -> OutputStream {
        let path = msg.path().to_string();

        // JSON API at /b/admin/api/... — normalize to /admin/... for sub-module compatibility
        if let Some(api_rest) = path.strip_prefix("/b/admin/api") {
            let normalized = format!("/admin{}", api_rest);
            msg.set_meta("req.resource", &normalized);

            if api_rest.starts_with("/users") {
                return users::handle(ctx, &msg, input).await;
            }
            if api_rest.starts_with("/database") {
                return database::handle(ctx, &msg, input).await;
            }
            if api_rest.starts_with("/iam") {
                return iam::handle(ctx, &msg, input).await;
            }
            if api_rest.starts_with("/logs") {
                return logs::handle(ctx, &msg).await;
            }
            if api_rest.starts_with("/settings") {
                return settings::handle(ctx, &msg, input).await;
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
                return ok_json(&blocks);
            }
            if api_rest.starts_with("/wafer") {
                return wafer_info::handle(ctx, &msg);
            }
            if api_rest.starts_with("/custom-tables") {
                return custom_tables::handle(ctx, &msg, input).await;
            }
            // Delegate admin storage to Files block
            if api_rest.starts_with("/storage") {
                msg.set_meta("req.resource", format!("/admin{}", api_rest));
                return crate::blocks::files::handle_admin_storage(ctx, msg, input).await;
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
                return crate::blocks::files::handle_admin_cloud(ctx, msg, input).await;
            }
            return err_not_found("not found");
        }

        // SSR pages + htmx mutations at /b/admin/...
        if path.starts_with("/b/admin") {
            let action = msg.action().to_string();
            let sub = path.strip_prefix("/b/admin").unwrap_or("/").to_string();

            // htmx mutation handlers
            if action == "create" && sub.ends_with("/disable") {
                let user_id = sub
                    .strip_prefix("/users/")
                    .and_then(|s| s.strip_suffix("/disable"))
                    .unwrap_or("")
                    .to_string();
                if !user_id.is_empty() {
                    return pages::handle_user_disable(ctx, &msg, &user_id).await;
                }
            }
            if action == "create" && sub.ends_with("/enable") {
                let user_id = sub
                    .strip_prefix("/users/")
                    .and_then(|s| s.strip_suffix("/enable"))
                    .unwrap_or("")
                    .to_string();
                if !user_id.is_empty() {
                    return pages::handle_user_enable(ctx, &msg, &user_id).await;
                }
            }
            if action == "delete" && sub.starts_with("/users/") {
                let user_id = sub.strip_prefix("/users/").unwrap_or("").to_string();
                if !user_id.is_empty() {
                    return pages::handle_user_delete(ctx, &msg, &user_id).await;
                }
            }
            if action == "create" && sub == "/iam/roles" {
                return pages::handle_create_role(ctx, &msg, input).await;
            }
            if action == "delete" && sub.starts_with("/iam/roles/") {
                let role_id = sub.strip_prefix("/iam/roles/").unwrap_or("").to_string();
                if !role_id.is_empty() {
                    return pages::handle_delete_role(ctx, &msg, &role_id).await;
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
                    return pages::handle_block_detail(ctx, &msg, &block_name).await;
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
                    return pages::handle_toggle_feature(ctx, &msg, &block_name).await;
                }
            }
            // Variable mutations
            if action == "create" && sub == "/variables" {
                return pages::handle_create_variable(ctx, &msg, input).await;
            }
            if action == "retrieve" && sub.ends_with("/edit") && sub.starts_with("/variables/") {
                let var_key = sub
                    .strip_prefix("/variables/")
                    .and_then(|s| s.strip_suffix("/edit"))
                    .unwrap_or("")
                    .to_string();
                if !var_key.is_empty() {
                    return pages::handle_edit_variable_form(ctx, &msg, &var_key).await;
                }
            }
            if action == "update" && sub.starts_with("/variables/") {
                let var_key = sub.strip_prefix("/variables/").unwrap_or("").to_string();
                if !var_key.is_empty() {
                    return pages::handle_update_variable(ctx, &msg, input, &var_key).await;
                }
            }

            // Network detail fragments (htmx)
            if action == "retrieve" && sub == "/network/detail/inbound" {
                return pages::network_inbound_detail(ctx, &msg).await;
            }
            if action == "retrieve" && sub == "/network/detail/outbound" {
                return pages::network_outbound_detail(ctx, &msg).await;
            }

            // WRAP grants CRUD (htmx)
            if action == "create" && sub == "/grants/rules" {
                return handle_create_wrap_grant(ctx, msg, input).await;
            }
            if action == "delete" && sub.starts_with("/grants/rules/") {
                let rule_id = sub
                    .strip_prefix("/grants/rules/")
                    .unwrap_or("")
                    .to_string();
                if !rule_id.is_empty() {
                    return handle_delete_wrap_grant(ctx, msg, &rule_id).await;
                }
            }

            // Email settings save (POST)
            if action == "create" && sub == "/email" {
                return pages::handle_save_email_settings(ctx, &msg, input).await;
            }

            // Custom block management
            if action == "create" && sub == "/custom-blocks/install" {
                return pages::handle_custom_block_install(ctx, &msg, input).await;
            }
            if action == "create" && sub == "/custom-blocks/upload" {
                return pages::handle_custom_block_upload(ctx, &msg, input).await;
            }
            if action == "delete" && sub.starts_with("/custom-blocks/") {
                let encoded = sub.strip_prefix("/custom-blocks/").unwrap_or("").to_string();
                if !encoded.is_empty() {
                    let block_name = encoded.replace("--", "/");
                    return pages::handle_custom_block_delete(ctx, &msg, &block_name).await;
                }
            }

            // SSR page handlers (GET)
            return match sub.as_str() {
                "" | "/" => pages::dashboard(ctx, &msg).await,
                "/users" => pages::users_page(ctx, &msg).await,
                "/variables" => pages::variables_page(ctx, &msg).await,
                "/network" => pages::network_page(ctx, &msg).await,
                "/storage" => pages::storage_page(ctx, &msg).await,
                "/blocks" => pages::blocks_page(ctx, &msg).await,
                "/logs" => pages::logs_page(ctx, &msg).await,
                "/email" => pages::email_settings_page(ctx, &msg).await,
                "/permissions" => pages::permissions_page(ctx, &msg).await,
                "/grants" => pages::grants_page(ctx, &msg).await,
                _ => err_not_found("not found"),
            };
        }

        err_not_found("not found")
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

// ---------------------------------------------------------------------------
// WRAP grant handlers
// ---------------------------------------------------------------------------

use crate::blocks::helpers::parse_form_body;
use wafer_core::clients::database as db;

async fn handle_create_wrap_grant(
    ctx: &dyn Context,
    mut msg: Message,
    input: InputStream,
) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let form = parse_form_body(&raw);
    let grantee = form.get("grantee").cloned().unwrap_or_default();
    let resource = form.get("resource").cloned().unwrap_or_default();
    let write = form
        .get("write")
        .map(|v| v == "on" || v == "true" || v == "1")
        .unwrap_or(false);
    let resource_type = form.get("resource_type").cloned().unwrap_or_default();
    let description = form.get("description").cloned().unwrap_or_default();

    if grantee.is_empty() || resource.is_empty() {
        return pages::permissions_page(ctx, &msg).await;
    }

    let mut data = std::collections::HashMap::new();
    data.insert("grantee".into(), serde_json::json!(grantee));
    data.insert("resource".into(), serde_json::json!(resource));
    data.insert(
        "write".into(),
        serde_json::json!(if write { 1 } else { 0 }),
    );
    data.insert("resource_type".into(), serde_json::json!(resource_type));
    data.insert("description".into(), serde_json::json!(description));
    let _ = db::create(ctx, WRAP_GRANTS_COLLECTION, data).await;

    msg.set_meta("req.query.tab", "database");
    pages::permissions_page(ctx, &msg).await
}

async fn handle_delete_wrap_grant(
    ctx: &dyn Context,
    mut msg: Message,
    grant_id: &str,
) -> OutputStream {
    let _ = db::delete(ctx, WRAP_GRANTS_COLLECTION, grant_id).await;
    msg.set_meta("req.query.tab", "database");
    pages::permissions_page(ctx, &msg).await
}
