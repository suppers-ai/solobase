mod users;
mod database;
mod iam;
mod logs;
mod settings;
mod wafer_info;
mod custom_tables;

use wafer_run::block::{Block, BlockInfo};
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
        BlockInfo {
            name: "@solobase/admin".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "Admin panel: users, database, IAM, logs, settings, wafer introspection, custom tables".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
            runtime: wafer_run::types::BlockRuntime::Native,
            requires: Vec::new(),
        }
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path();

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
