use wafer_run::{context::Context, OutputStream, Message};

use crate::blocks::helpers::{err_not_found, ok_json};

pub fn handle(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let action = msg.action();
    let path = msg.path();

    match (action, path) {
        ("retrieve", "/admin/wafer/blocks") => handle_blocks(ctx),
        ("retrieve", "/admin/wafer/flows") => handle_flows(),
        ("retrieve", "/admin/wafer/info") => handle_info(),
        _ => err_not_found("not found"),
    }
}

fn handle_blocks(ctx: &dyn Context) -> OutputStream {
    // Surface the live registered-blocks list from the runtime. Was a
    // hand-maintained static list that drifted from reality every time a
    // block was added or renamed.
    let blocks: Vec<serde_json::Value> = ctx
        .registered_blocks()
        .into_iter()
        .map(|b| {
            let runtime_label = match b.runtime {
                wafer_run::BlockRuntime::Wasm => "wasm",
                _ => "native",
            };
            serde_json::json!({
                "name": b.name,
                "version": b.version,
                "interface": b.interface,
                "type": runtime_label,
            })
        })
        .collect();
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
