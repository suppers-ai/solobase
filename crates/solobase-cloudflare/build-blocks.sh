#!/bin/bash
# Build all solobase WASM blocks and jco-transpile them for Cloudflare Workers.
#
# Each block is compiled to a WASM Component, then transpiled via jco into an
# ES module that can be instantiated at runtime inside the worker.
#
# Prerequisites:
#   cargo, wasm-tools, npx @bytecodealliance/jco
#   WASM target: rustup target add wasm32-unknown-unknown

set -euo pipefail

BLOCKS=(system auth profile userportal legalpages admin files products deployments)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SOLOBASE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BLOCKS_DIR="$SOLOBASE_ROOT/blocks"
BLOCKS_OUT="$SCRIPT_DIR/worker/blocks"
mkdir -p "$BLOCKS_OUT"

echo "=== Building WASM blocks ==="
echo "  Blocks dir: $BLOCKS_DIR"
echo "  Output dir: $BLOCKS_OUT"
echo ""

for block in "${BLOCKS[@]}"; do
  CRATE_DIR="$BLOCKS_DIR/$block"
  CRATE_NAME="solobase-block-${block}"

  if [ ! -d "$CRATE_DIR" ]; then
    echo "SKIP: $block (directory not found at $CRATE_DIR)"
    continue
  fi

  echo "--- Building $block ---"

  # 1. Build for wasm32
  cargo build --release --target wasm32-unknown-unknown \
    --manifest-path "$CRATE_DIR/Cargo.toml"

  # 2. Find the .wasm output
  WASM_FILE="$CRATE_DIR/target/wasm32-unknown-unknown/release/${CRATE_NAME//-/_}.wasm"
  if [ ! -f "$WASM_FILE" ]; then
    echo "ERROR: Expected wasm at $WASM_FILE"
    exit 1
  fi

  # 3. Convert to WASM Component
  COMPONENT_FILE="/tmp/${block}.component.wasm"
  wasm-tools component new "$WASM_FILE" -o "$COMPONENT_FILE"
  wasm-tools validate "$COMPONENT_FILE"

  # 4. jco transpile → ES module with async instantiation
  BLOCK_OUT="$BLOCKS_OUT/$block"
  rm -rf "$BLOCK_OUT"
  npx @bytecodealliance/jco transpile "$COMPONENT_FILE" --instantiation async -o "$BLOCK_OUT"

  rm -f "$COMPONENT_FILE"

  SIZE=$(ls -lh "$WASM_FILE" | awk '{print $5}')
  echo "  → $block ($SIZE)"
done

echo ""
echo "=== Done ==="
