mod users;
mod database;
mod iam;
mod logs;
mod settings;
mod wafer_info;
mod custom_tables;

use std::sync::Arc;
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_run::services::database::DatabaseService;

pub struct AdminBlock;

pub(crate) fn get_db(ctx: &dyn Context) -> Result<&Arc<dyn DatabaseService>, Result_> {
    ctx.services()
        .and_then(|s| s.database.as_ref())
        .ok_or_else(|| Result_::error(WaferError::new("unavailable", "Database service unavailable")))
}

impl Block for AdminBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "admin-feature".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "Admin panel: users, database, IAM, logs, settings, wafer introspection, custom tables".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
        }
    }

    fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path();

        if path.starts_with("/admin/users") {
            return users::handle(ctx, msg);
        }
        if path.starts_with("/admin/database") {
            return database::handle(ctx, msg);
        }
        if path.starts_with("/admin/iam") {
            return iam::handle(ctx, msg);
        }
        if path.starts_with("/admin/logs") {
            return logs::handle(ctx, msg);
        }
        if path.starts_with("/admin/settings") || path.starts_with("/settings") {
            return settings::handle(ctx, msg);
        }
        if path.starts_with("/admin/wafer") || path.starts_with("/admin/waffle") {
            return wafer_info::handle(ctx, msg);
        }
        if path.starts_with("/admin/custom-tables") {
            return custom_tables::handle(ctx, msg);
        }

        err_not_found(msg.clone(), "not found")
    }

    fn lifecycle(&self, ctx: &dyn Context, event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        if matches!(event.event_type, LifecycleType::Init) {
            if let Some(db) = ctx.services().and_then(|s| s.database.as_ref()) {
                iam::seed_defaults(db.as_ref());
                settings::seed_defaults(db.as_ref());
            }
        }
        Ok(())
    }
}
