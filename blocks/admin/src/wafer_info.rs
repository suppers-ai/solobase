use crate::wafer::block_world::types::*;
use crate::helpers::*;

pub fn handle(msg: &Message) -> BlockResult {
    let action = msg_get_meta(msg, "req.action");
    let path = msg_get_meta(msg, "req.resource");

    match (action, path) {
        ("retrieve", "/admin/wafer/blocks") => handle_blocks(msg),
        ("retrieve", "/admin/wafer/flows") => handle_flows(msg),
        ("retrieve", "/admin/wafer/info") => handle_info(msg),
        _ => err_not_found(msg, "not found"),
    }
}

fn handle_blocks(msg: &Message) -> BlockResult {
    // ctx.registered_blocks() is not available in WASM — return empty array.
    json_respond(msg, &serde_json::json!([]))
}

fn handle_flows(msg: &Message) -> BlockResult {
    // ctx.flow_infos() is not available in WASM — return empty array.
    json_respond(msg, &serde_json::json!([]))
}

fn handle_info(msg: &Message) -> BlockResult {
    json_respond(msg, &serde_json::json!({
        "runtime": "wafer",
        "version": "1.0.0",
        "platform": "solobase",
        "block_mode": "wasm-component",
        "features": ["database", "storage", "crypto", "network", "config"]
    }))
}
