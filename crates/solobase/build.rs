//! Locates the precompiled solobase-web wasm in the workspace target dir
//! and copies it into OUT_DIR for include_bytes! consumption from main.

use std::{env, fs, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR set by cargo"));

    // CARGO_MANIFEST_DIR is crates/solobase. Workspace root is two up.
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root is two levels above crates/solobase");

    // 1. WASM binary candidates
    let wasm_candidates = [
        workspace_root.join("crates/solobase-web/pkg/solobase_web_bg.wasm"),
        workspace_root.join("target/wasm32-unknown-unknown/release/solobase_web.wasm"),
    ];

    let wasm_src = wasm_candidates.iter().find(|p| p.exists()).unwrap_or_else(|| {
        eprintln!(
            "\nerror: solobase-web wasm not found. Tried:\n{}\n\nRun \"just build\" or:\n  \
                 cargo build -p solobase-web --release --target wasm32-unknown-unknown\nfirst.\n",
            wasm_candidates
                .iter()
                .map(|p| format!("  - {}", p.display()))
                .collect::<Vec<_>>()
                .join("\n")
        );
        std::process::exit(1);
    });

    let wasm_dst = out_dir.join("solobase-web.wasm");
    fs::copy(wasm_src, &wasm_dst)
        .unwrap_or_else(|e| panic!("failed to copy {} -> {}: {e}", wasm_src.display(), wasm_dst.display()));

    // 2. JS glue file
    let js_src = workspace_root.join("crates/solobase-web/pkg/solobase_web.js");
    if !js_src.exists() {
        eprintln!(
            "\nerror: solobase-web JS glue not found at {}.\nRun \"just build\" first.\n",
            js_src.display()
        );
        std::process::exit(1);
    }

    let js_dst = out_dir.join("solobase-web.js");
    fs::copy(&js_src, &js_dst)
        .unwrap_or_else(|e| panic!("failed to copy {} -> {}: {e}", js_src.display(), js_dst.display()));

    // Re-run if the source wasm or JS changes.
    println!("cargo:rerun-if-changed={}", wasm_src.display());
    println!("cargo:rerun-if-changed={}", js_src.display());
    // Allow override during developer iteration via env.
    println!("cargo:rerun-if-env-changed=SOLOBASE_WEB_WASM_OVERRIDE_FOR_BUILD");
}
