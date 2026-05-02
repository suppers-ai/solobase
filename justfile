# Default: full build (wasm + native).
default: build

# Build solobase-web wasm via wasm-pack (gets wasm-opt automatically),
# then the solobase CLI binary which include_bytes!s the wasm.
build:
    cd crates/solobase-web && RUSTFLAGS="-C target-feature=+simd128" wasm-pack build --target web --release --out-dir pkg
    cargo build -p solobase --release

# Build the CLI in debug profile. Wasm stays release-built (no point
# shipping a debug wasm — it's data baked into the binary).
build-debug:
    cd crates/solobase-web && RUSTFLAGS="-C target-feature=+simd128" wasm-pack build --target web --release --out-dir pkg
    cargo build -p solobase

# Run the workspace test suite.
test:
    cargo test --workspace

# Run only the unit tests (no integration tests that need a wasm rebuild).
test-unit:
    cargo test --workspace --lib

# Clean all build artifacts.
clean:
    cargo clean
    rm -rf crates/solobase-web/pkg
