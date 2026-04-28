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

    // wasm-pack and cargo cdylib outputs both land under target/wasm32-...
    // Try the canonical post-wasm-pack location first, then the raw cargo
    // cdylib output. Filename is solobase_web.wasm (Cargo replaces - with _).
    let candidates = [
        workspace_root.join("crates/solobase-web/pkg/solobase_web_bg.wasm"),
        workspace_root.join("target/wasm32-unknown-unknown/release/solobase_web.wasm"),
    ];

    let src = candidates.iter().find(|p| p.exists()).unwrap_or_else(|| {
        eprintln!(
            "\nerror: solobase-web wasm not found. Tried:\n{}\n\nRun \"just build\" or:\n  \
                 cargo build -p solobase-web --release --target wasm32-unknown-unknown\nfirst.\n",
            candidates
                .iter()
                .map(|p| format!("  - {}", p.display()))
                .collect::<Vec<_>>()
                .join("\n")
        );
        std::process::exit(1);
    });

    let dst = out_dir.join("solobase-web.wasm");
    fs::copy(src, &dst)
        .unwrap_or_else(|e| panic!("failed to copy {} -> {}: {e}", src.display(), dst.display()));

    // Re-run if the source wasm changes.
    println!("cargo:rerun-if-changed={}", src.display());
    // Allow override during developer iteration via env.
    println!("cargo:rerun-if-env-changed=SOLOBASE_WEB_WASM_OVERRIDE_FOR_BUILD");
}
