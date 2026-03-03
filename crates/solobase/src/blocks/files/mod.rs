mod storage;
mod cloud;
mod quota;
mod share;
pub(crate) mod models;

use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;

pub(crate) use super::helpers::{get_db, get_storage};

pub struct FilesBlock;

impl Block for FilesBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "files-feature".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "File storage, sharing, quotas, and access logging".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
        }
    }

    fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path();

        // Direct share access (public, no auth)
        if path.starts_with("/storage/direct/") {
            return share::handle_direct_access(ctx, msg);
        }

        // Cloud storage routes
        if path.starts_with("/ext/cloudstorage") || path.starts_with("/admin/ext/cloudstorage") {
            return cloud::handle(ctx, msg);
        }

        // Admin storage routes
        if path.starts_with("/admin/storage") {
            return storage::handle_admin(ctx, msg);
        }

        // User storage routes
        if path.starts_with("/storage") {
            return storage::handle(ctx, msg);
        }

        err_not_found(msg.clone(), "not found")
    }

    fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
