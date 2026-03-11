use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;

pub fn handle(_ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/wafer/blocks") => handle_blocks(msg),
        ("retrieve", "/admin/wafer/flows") => handle_flows(msg),
        ("retrieve", "/admin/wafer/info") => handle_info(msg),
        _ => err_not_found(msg, "not found"),
    }
}

fn handle_blocks(msg: &mut Message) -> Result_ {
    // Return list of registered blocks
    // In a real implementation, this would query the Wafer runtime
    // For now, return the known block list
    let blocks = serde_json::json!([
        {"name": "auth-feature", "version": "1.0.0", "interface": "http.handler", "type": "native"},
        {"name": "admin-feature", "version": "1.0.0", "interface": "http.handler", "type": "native"},
        {"name": "system-feature", "version": "1.0.0", "interface": "http.handler", "type": "native"},
        {"name": "files-feature", "version": "1.0.0", "interface": "http.handler", "type": "native"},
        {"name": "legalpages-feature", "version": "1.0.0", "interface": "http.handler", "type": "native"},
        {"name": "products-feature", "version": "1.0.0", "interface": "http.handler", "type": "native"},
        {"name": "userportal-feature", "version": "1.0.0", "interface": "http.handler", "type": "native"},
        {"name": "profile-feature", "version": "1.0.0", "interface": "http.handler", "type": "native"},
        {"name": "monitoring-feature", "version": "1.0.0", "interface": "http.handler", "type": "native"},
        {"name": "web-feature", "version": "1.0.0", "interface": "http.handler", "type": "native"},
        {"name": "@wafer/auth-validator", "version": "0.1.0", "interface": "middleware@v1", "type": "native"},
        {"name": "@wafer/web", "version": "0.1.0", "interface": "handler@v1", "type": "native"},
        {"name": "@wafer/cors", "version": "0.1.0", "interface": "middleware@v1", "type": "native"},
        {"name": "@wafer/security-headers", "version": "0.1.0", "interface": "middleware@v1", "type": "native"},
        {"name": "@wafer/ip-rate-limit", "version": "0.1.0", "interface": "middleware@v1", "type": "native"},
        {"name": "@wafer/iam-guard", "version": "0.1.0", "interface": "middleware@v1", "type": "native"}
    ]);
    json_respond(msg, &blocks)
}

fn handle_flows(msg: &mut Message) -> Result_ {
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
        {"id": "profile", "summary": "Profile sections"},
        {"id": "web-site", "summary": "Static website serving"},
        {"id": "http-infra", "summary": "HTTP infrastructure pipeline"},
        {"id": "auth-pipe", "summary": "Authentication pipeline"},
        {"id": "admin-pipe", "summary": "Admin authorization pipeline"}
    ]);
    json_respond(msg, &flows)
}

fn handle_info(msg: &mut Message) -> Result_ {
    json_respond(msg, &serde_json::json!({
        "runtime": "wafer",
        "version": "1.0.0",
        "platform": "solobase",
        "block_mode": "native-rust",
        "features": ["database", "storage", "crypto", "network", "config"]
    }))
}
