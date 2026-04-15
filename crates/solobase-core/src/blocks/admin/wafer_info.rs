use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::OutputStream;

use crate::blocks::helpers::{err_not_found, ok_json};

pub fn handle(_ctx: &dyn Context, msg: &Message) -> OutputStream {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/wafer/blocks") => handle_blocks(),
        ("retrieve", "/admin/wafer/flows") => handle_flows(),
        ("retrieve", "/admin/wafer/info") => handle_info(),
        _ => err_not_found("not found"),
    }
}

fn handle_blocks() -> OutputStream {
    // Return list of registered blocks
    // In a real implementation, this would query the Wafer runtime
    // For now, return the known block list
    let blocks = serde_json::json!([
        {"name": "auth-feature", "version": "1.0.0", "interface": "http-handler@v1", "type": "native"},
        {"name": "admin-feature", "version": "1.0.0", "interface": "http-handler@v1", "type": "native"},
        {"name": "system-feature", "version": "1.0.0", "interface": "http-handler@v1", "type": "native"},
        {"name": "files-feature", "version": "1.0.0", "interface": "http-handler@v1", "type": "native"},
        {"name": "legalpages-feature", "version": "1.0.0", "interface": "http-handler@v1", "type": "native"},
        {"name": "products-feature", "version": "1.0.0", "interface": "http-handler@v1", "type": "native"},
        {"name": "userportal-feature", "version": "1.0.0", "interface": "http-handler@v1", "type": "native"},
        {"name": "profile-feature", "version": "1.0.0", "interface": "http-handler@v1", "type": "native"},
        {"name": "monitoring-feature", "version": "1.0.0", "interface": "http-handler@v1", "type": "native"},
        {"name": "web-feature", "version": "1.0.0", "interface": "http-handler@v1", "type": "native"},
        {"name": "wafer-run/auth-validator", "version": "0.1.0", "interface": "middleware@v1", "type": "native"},
        {"name": "wafer-run/web", "version": "0.1.0", "interface": "handler@v1", "type": "native"},
        {"name": "wafer-run/cors", "version": "0.1.0", "interface": "middleware@v1", "type": "native"},
        {"name": "wafer-run/security-headers", "version": "0.1.0", "interface": "middleware@v1", "type": "native"},
        {"name": "wafer-run/ip-rate-limit", "version": "0.1.0", "interface": "middleware@v1", "type": "native"},
        {"name": "wafer-run/iam-guard", "version": "0.1.0", "interface": "middleware@v1", "type": "native"}
    ]);
    ok_json(&blocks)
}

fn handle_flows() -> OutputStream {
    // Return flow list. In production, query Wafer runtime.
    let flows = serde_json::json!([
        {"id": "auth", "summary": "Authentication routes"},
        {"id": "system", "summary": "System infrastructure routes"},
        {"id": "admin", "summary": "Admin management"},
        {"id": "monitoring", "summary": "Monitoring dashboard"},
        {"id": "settings", "summary": "Settings routes"},
        {"id": "files", "summary": "File storage, sharing, quotas"},
        {"id": "legalpages", "summary": "Legal pages routes"},
        {"id": "products", "summary": "Products extension routes"},
        {"id": "userportal", "summary": "User portal config"},
        {"id": "web-site", "summary": "Static website serving"},
        {"id": "http-infra", "summary": "HTTP infrastructure pipeline"},
        {"id": "auth-pipe", "summary": "Authentication pipeline"},
        {"id": "admin-pipe", "summary": "Admin authorization pipeline"}
    ]);
    ok_json(&flows)
}

fn handle_info() -> OutputStream {
    ok_json(&serde_json::json!({
        "runtime": "wafer",
        "version": "1.0.0",
        "platform": "solobase",
        "block_mode": "native-rust",
        "features": ["database", "storage", "crypto", "network", "config"]
    }))
}
