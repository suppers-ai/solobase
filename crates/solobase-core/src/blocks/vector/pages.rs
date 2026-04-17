//! HTTP route dispatcher for suppers-ai/vector.
//!
//! Actual route handlers are implemented in Tasks 15-17 and 19. This module
//! currently provides a single placeholder that returns `Unimplemented` for
//! any (action, path) pair declared in `BlockInfo::endpoints`.

use wafer_run::{context::Context, types::*, OutputStream};

/// Placeholder route dispatcher. Returns `Unimplemented` for every action/path
/// so that the block compiles and can be registered before the real handlers
/// are written.
pub async fn route(_ctx: &dyn Context, msg: &Message) -> OutputStream {
    let action = msg.action();
    let path = msg.path();
    OutputStream::error(WaferError {
        code: ErrorCode::Unimplemented,
        message: format!("vector route not yet implemented: {action} {path}"),
        meta: vec![],
    })
}
