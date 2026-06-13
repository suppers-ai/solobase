mod database;
mod iam;
mod logs;
pub mod migrations;
mod ops;
mod pages;
mod route;
mod settings;
mod users;

pub(crate) use iam::{PERMISSIONS_TABLE, ROLES_TABLE, USER_ROLES_TABLE};
pub(crate) use logs::{AUDIT_LOGS_TABLE, REQUEST_LOGS_TABLE, STORAGE_ACCESS_LOGS_TABLE};
pub use settings::{BLOCK_SETTINGS_TABLE, VARIABLES_TABLE};

/// Registered name of the admin block.
///
/// Mirror of [`crate::blocks::auth::AUTH_BLOCK_ID`] for callers that need to
/// reference the admin block by name without hardcoding the string (e.g.
/// `solobase-cloudflare` initialises the admin block first so its migrations
/// have run before the runner seeds `auto_generate` secrets).
pub const ADMIN_BLOCK_ID: &str = "suppers-ai/admin";

/// WRAP grant rows (block-to-resource access tokens).
pub(crate) const WRAP_GRANTS_TABLE: &str = "suppers_ai__admin__wrap_grants";

use wafer_run::{
    context::Context, Block, BlockEndpoint, BlockInfo, ErrorCode, InputStream, InstanceMode,
    LifecycleEvent, LifecycleType, Message, OutputStream, WaferError,
};

use crate::blocks::helpers::{err_not_found, ok_json};

pub struct AdminBlock;

impl AdminBlock {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AdminBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for AdminBlock {
    fn info(&self) -> BlockInfo {
        use wafer_run::{AuthLevel, CollectionSchema};

        BlockInfo::new("suppers-ai/admin", "0.0.1", "http-handler@v1", "Admin panel: users, database, IAM, logs, settings")
            .instance_mode(InstanceMode::Singleton)
            .requires(vec![
                "wafer-run/database".into(),
                "wafer-run/config".into(),
                "wafer-run/crypto".into(),
            ])
            .collections(vec![
                CollectionSchema::new(ROLES_TABLE)
                    .field("name", "string")
                    .field_default("description", "string", "")
                    .field_default("permissions", "json", "[]")
                    .field_default("is_system", "bool", "false"),
                CollectionSchema::new(PERMISSIONS_TABLE)
                    .field("name", "string")
                    .field_default("resource", "string", "")
                    .field_default("actions", "json", "[]"),
                CollectionSchema::new(USER_ROLES_TABLE)
                    .field_ref("user_id", "string", &format!("{}.id", crate::blocks::auth::USERS_TABLE))
                    .field("role", "string")
                    .field_optional("assigned_at", "datetime")
                    .field_default("assigned_by", "string", "")
                    .index(&["user_id"]),
                CollectionSchema::new(VARIABLES_TABLE)
                    .field_unique("key", "string")
                    .field_default("name", "string", "")
                    .field_default("description", "string", "")
                    .field_default("value", "string", "")
                    .field_default("warning", "string", "")
                    .field_default("sensitive", "int", "0")
                    .field_default("updated_by", "string", ""),
                CollectionSchema::new(AUDIT_LOGS_TABLE)
                    .field_default("user_id", "string", "")
                    .field("action", "string")
                    .field_default("resource", "string", "")
                    .field_default("ip_address", "string", "")
                    .index(&["created_at"]),
                CollectionSchema::new(REQUEST_LOGS_TABLE)
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
                CollectionSchema::new(STORAGE_ACCESS_LOGS_TABLE)
                    .field_default("source_block", "string", "")
                    .field_default("operation", "string", "")
                    .field_default("path", "string", "")
                    .field_default("status", "string", "")
                    .index(&["created_at"]),
                CollectionSchema::new(BLOCK_SETTINGS_TABLE)
                    .field_unique("block_name", "string")
                    .field_default("enabled", "int", "1"),
                CollectionSchema::new(WRAP_GRANTS_TABLE)
                    .field("grantee", "string")
                    .field("resource", "string")
                    .field_default("write", "int", "0")
                    .field_default("resource_type", "string", "")
                    .field_default("description", "string", ""),
            ])
            .grants(vec![
                wafer_run::ResourceGrant::read_write(super::auth::AUTH_BLOCK_ID, USER_ROLES_TABLE),
                wafer_run::ResourceGrant::read(super::auth::AUTH_BLOCK_ID, VARIABLES_TABLE),
                wafer_run::ResourceGrant::read("suppers-ai/userportal", BLOCK_SETTINGS_TABLE),
                // Every block may upsert its own migration state into block_settings.
                wafer_run::ResourceGrant::read_write("*", BLOCK_SETTINGS_TABLE),
                // Infrastructure logging: storage wrapper + pipeline write logs
                wafer_run::ResourceGrant::read_write("*", STORAGE_ACCESS_LOGS_TABLE),
                wafer_run::ResourceGrant::read_write("*", REQUEST_LOGS_TABLE),
                // Default: allow all blocks to make outbound network requests.
                // Remove this grant via the admin UI to restrict network access.
                wafer_run::ResourceGrant::read("*", "*")
                    .typed(wafer_run::ResourceType::Network),
                // Default: allow all blocks to perform any crypto operation
                // (hash/compare_hash/sign/verify/random_bytes). The runtime
                // already isolates JWT signing keys per caller via HKDF
                // (SEC-016), so this wildcard does not let a block forge
                // another block's tokens. Tighten via the admin UI (e.g.
                // restrict sign/verify to specific blocks) if a deployment
                // wants per-op control.
                wafer_run::ResourceGrant::read_write("*", "*")
                    .typed(wafer_run::ResourceType::Crypto),
                // Wave 26 (c18) made Storage WRAP namespace-aware: every
                // block self-admits its own `{org}/{block}/*` namespace
                // via Rule 3 without any grant. The previous
                // `read_write("suppers-ai/files", "*")` grant the admin
                // block used to declare on behalf of the files block was
                // removed because the files block now reaches its own
                // storage namespace under the new self-admit rule.
                // Cross-block Storage grants are declared by the owning
                // block, the same way Db grants are.
            ])
            .category(wafer_run::BlockCategory::Feature)
            .description("Administration panel for managing users, roles, variables, blocks, and logs. Provides SSR dashboard with stats, user management with role assignment, IAM (roles and API keys), environment variables editor, block management with feature toggles, and system/audit log viewer.")
            .endpoints(vec![
                BlockEndpoint::get("/b/admin/").summary("Dashboard").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/users").summary("User management").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/variables").summary("Config management").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/blocks").summary("Block management").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/network").summary("Network monitoring").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/storage").summary("Storage isolation and access logs").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/logs").summary("System and audit logs").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/email").summary("Email settings").auth(AuthLevel::Admin),
                BlockEndpoint::post("/b/admin/email").summary("Save email settings").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/permissions").summary("Permissions management").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/grants").summary("WRAP grants management").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/database").summary("Database admin page").auth(AuthLevel::Admin),
                BlockEndpoint::post("/b/admin/database/query").summary("Run read-only SQL (SSR)").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/api/users").summary("List users API").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/api/iam/roles").summary("List roles API").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/api/settings").summary("List variables API").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/admin/api/logs").summary("Audit logs API").auth(AuthLevel::Admin),
            ])
    }

    async fn handle(
        &self,
        ctx: &dyn Context,
        mut msg: Message,
        input: InputStream,
    ) -> OutputStream {
        use route::AdminRoute;

        // Capture path + action BEFORE any meta mutation. msg.path() reads from
        // get_meta("req.resource") which the API normalization below mutates;
        // without these captures the routing classifier would see the normalized
        // path and miss the /b/admin/api prefix.
        let path_owned = msg.path().to_string();
        let action_owned = msg.action().to_string();

        // API path normalization: downstream sub-handlers (users::handle,
        // database::handle, etc.) expect req.resource as /admin/... instead
        // of /b/admin/api/... — preserve that contract exactly as the
        // pre-refactor handler did.
        if let Some(api_rest) = path_owned.strip_prefix("/b/admin/api") {
            msg.set_meta("req.resource", &format!("/admin{}", api_rest));
        }
        match route::route(&path_owned, &action_owned) {
            // --- /b/admin/api/... ---
            AdminRoute::UsersApi => users::handle(ctx, &msg, input).await,
            AdminRoute::DatabaseApi => database::handle(ctx, &msg, input).await,
            AdminRoute::IamApi => iam::handle(ctx, &msg, input).await,
            AdminRoute::LogsApi => logs::handle(ctx, &msg).await,
            AdminRoute::SettingsApi => settings::handle(ctx, &msg, input).await,
            AdminRoute::ExtensionsApi => {
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
                ok_json(&blocks)
            }
            AdminRoute::StorageDelegate => {
                // The original handler re-set req.resource INSIDE the if branch
                // (to /admin/<api_rest>). The top-of-function normalization already
                // did this, but the original re-applied; we mirror by deriving
                // from path_owned (NOT msg.path() which is now normalized).
                let api_rest = path_owned.strip_prefix("/b/admin/api").unwrap_or("");
                msg.set_meta("req.resource", &format!("/admin{}", api_rest));
                ctx.call_block("suppers-ai/files", msg, input).await
            }
            AdminRoute::CloudStorageDelegate { rest } => {
                msg.set_meta("req.resource", &format!("/admin/b/cloudstorage{}", rest));
                ctx.call_block("suppers-ai/files", msg, input).await
            }
            AdminRoute::ApiNotFound => err_not_found("not found"),

            // --- /b/admin/settings/... ---
            AdminRoute::SettingsRedirect => redirect_308("/b/admin/settings/email"),
            AdminRoute::SettingsPage { tab } => pages::settings_page(ctx, &msg, tab).await,

            // --- /b/admin/... htmx mutations ---
            AdminRoute::UserDisable { user_id } => {
                pages::handle_user_disable(ctx, &msg, user_id).await
            }
            AdminRoute::UserEnable { user_id } => {
                pages::handle_user_enable(ctx, &msg, user_id).await
            }
            AdminRoute::UserDelete { user_id } => {
                pages::handle_user_delete(ctx, &msg, user_id).await
            }
            AdminRoute::CreateRole => pages::handle_create_role(ctx, &msg, input).await,
            AdminRoute::DeleteRole { role_id } => {
                pages::handle_delete_role(ctx, &msg, role_id).await
            }
            AdminRoute::BlockDetail { block_name } => {
                pages::handle_block_detail(ctx, &msg, &block_name).await
            }
            AdminRoute::BlockToggle { block_name } => {
                pages::handle_toggle_feature(ctx, &msg, &block_name).await
            }
            AdminRoute::CreateVariable => pages::handle_create_variable(ctx, &msg, input).await,
            AdminRoute::EditVariableForm { var_key } => {
                pages::handle_edit_variable_form(ctx, &msg, var_key).await
            }
            AdminRoute::UpdateVariable { var_key } => {
                pages::handle_update_variable(ctx, &msg, input, var_key).await
            }
            AdminRoute::NetworkInboundDetail => pages::network_inbound_detail(ctx, &msg).await,
            AdminRoute::CreateWrapGrant => handle_create_wrap_grant(ctx, msg, input).await,
            AdminRoute::DeleteWrapGrant { rule_id } => {
                handle_delete_wrap_grant(ctx, msg, rule_id).await
            }
            AdminRoute::SaveEmailSettings => {
                pages::handle_save_email_settings(ctx, &msg, input).await
            }
            AdminRoute::DatabaseQuery => pages::handle_database_query(ctx, &msg, input).await,

            // --- /b/admin/... SSR pages ---
            AdminRoute::Dashboard => pages::dashboard(ctx, &msg).await,
            AdminRoute::UsersPage => pages::users_page(ctx, &msg).await,
            AdminRoute::StoragePage => pages::storage_page(ctx, &msg).await,
            AdminRoute::BlocksPage => pages::blocks_page(ctx, &msg).await,
            AdminRoute::DatabasePage => pages::database_page(ctx, &msg).await,
            AdminRoute::LogsPage => pages::logs_page(ctx, &msg).await,
            AdminRoute::EmailRedirect => redirect_308("/b/admin/settings/email"),
            AdminRoute::NetworkRedirect => redirect_308("/b/admin/settings/network"),
            AdminRoute::VariablesRedirect => redirect_308("/b/admin/settings/variables"),
            AdminRoute::PermissionsRedirect => {
                // Carry ?tab= as ?subtab= to preserve deep-links.
                let old_tab = msg.query("tab");
                if old_tab.is_empty() {
                    redirect_308("/b/admin/settings/permissions")
                } else {
                    redirect_308(&format!("/b/admin/settings/permissions?subtab={}", old_tab))
                }
            }
            AdminRoute::GrantsPage => pages::grants_page(ctx, &msg).await,

            AdminRoute::NotFound => err_not_found("not found"),
        }
    }

    async fn lifecycle(
        &self,
        ctx: &dyn Context,
        event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        if matches!(event.event_type, LifecycleType::Init) {
            if let Err(e) = migrations::apply(ctx).await {
                return Err(WaferError::new(
                    ErrorCode::Internal,
                    format!("admin migrations: {e}"),
                ));
            }
            iam::seed_defaults(ctx).await;
            settings::seed_defaults(ctx).await;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Redirect helper
// ---------------------------------------------------------------------------

/// Build a 308 Permanent Redirect to `target`. Preserves method + body
/// per RFC 7538, so POST/PUT htmx requests redirect correctly.
fn redirect_308(target: &str) -> OutputStream {
    crate::blocks::helpers::ResponseBuilder::new()
        .status(308)
        .set_header("Location", target)
        .body(Vec::new(), "text/plain")
}

// ---------------------------------------------------------------------------
// WRAP grant handlers
// ---------------------------------------------------------------------------

use wafer_core::clients::database as db;

use crate::blocks::helpers::parse_form_body;

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
    data.insert("write".into(), serde_json::json!(if write { 1 } else { 0 }));
    data.insert("resource_type".into(), serde_json::json!(resource_type));
    data.insert("description".into(), serde_json::json!(description));
    let _ = db::create(ctx, WRAP_GRANTS_TABLE, data).await;

    msg.set_meta("req.query.subtab", "database");
    pages::permissions_page(ctx, &msg).await
}

async fn handle_delete_wrap_grant(
    ctx: &dyn Context,
    mut msg: Message,
    grant_id: &str,
) -> OutputStream {
    let _ = db::delete(ctx, WRAP_GRANTS_TABLE, grant_id).await;
    msg.set_meta("req.query.subtab", "database");
    pages::permissions_page(ctx, &msg).await
}

#[cfg(not(target_arch = "wasm32"))]
::wafer_block::register_static_block!("suppers-ai/admin", AdminBlock);

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn redirect_308_sets_location_and_status() {
        let out = redirect_308("/b/admin/settings/email");
        let buf = out.collect_buffered().await.unwrap();
        let status = buf
            .meta
            .iter()
            .find(|e| e.key == "resp.status")
            .map(|e| e.value.as_str())
            .unwrap_or("");
        let location = buf
            .meta
            .iter()
            .find(|e| e.key == "resp.header.Location")
            .map(|e| e.value.as_str())
            .unwrap_or("");
        assert_eq!(status, "308");
        assert_eq!(location, "/b/admin/settings/email");
    }
}

#[cfg(test)]
mod grant_tests {
    use wafer_run::{Block, ResourceType};

    use super::AdminBlock;

    #[test]
    fn admin_block_no_longer_declares_storage_grant_for_files() {
        // Wave 26 (c18): Storage WRAP became namespace-aware. The files
        // block self-admits its own `suppers-ai/files/*` namespace via
        // Rule 3, so the admin block no longer needs to declare a typed
        // Storage grant on its behalf. This test pins the absence — if a
        // future change re-introduces the grant it's almost certainly a
        // regression from the c18 model.
        let admin = AdminBlock::new();
        let grants = admin.info().grants;

        let storage_grant_for_files = grants.iter().find(|g| {
            g.resource_type == Some(ResourceType::Storage) && g.grantee == "suppers-ai/files"
        });

        assert!(
            storage_grant_for_files.is_none(),
            "admin block must not declare a typed Storage grant for suppers-ai/files \
             — the files block self-admits its own namespace via WRAP Rule 3 (Wave 26 \
             / c18). Found: {storage_grant_for_files:?}"
        );
    }
}
