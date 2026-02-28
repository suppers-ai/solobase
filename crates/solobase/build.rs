//! Build script for Solobase.
//!
//! All feature blocks are now native Rust — no WASM pre-compilation needed.
//! This build script only triggers re-runs when the frontend assets change.

fn main() {
    println!("cargo:rerun-if-changed=frontend/build/");
}
