# Custom WASM Blocks on Cloudflare Workers

## Goal

Allow users to run custom blocks (written in Rust or TypeScript) on the Cloudflare Worker deployment of Solobase.

## Approach: V8 WebAssembly API

The CF Worker already runs inside V8. Instead of embedding a WASM interpreter (wasmi) inside the Worker WASM binary, we call `WebAssembly.instantiate()` directly via wasm-bindgen. V8 JIT-compiles the guest block natively — no double-interpretation overhead.

### Flow

1. User writes a block in Rust or TypeScript (using `wafer-sdk-ts`)
2. Compiles to `wasm32-unknown-unknown` → `.wasm` file
3. Uploads the `.wasm` to R2 (e.g. `blocks/my-block.wasm`)
4. Registers the block in tenant config (D1 or KV)
5. On request, the Worker loads the `.wasm` from R2, instantiates it via V8, and calls it

### ABI

Reuse the existing WAFER thin ABI defined in `wafer-run/crates/wafer-run/src/wasm/loader.rs`:

**Guest exports:**
- `__wafer_alloc(len: i32) -> i32` — allocate guest memory
- `__wafer_info() -> i64` — return block info (packed ptr|len)
- `__wafer_handle(ptr: i32, len: i32) -> i64` — handle a message
- `__wafer_lifecycle(ptr: i32, len: i32) -> i64` — lifecycle events

**Host imports (`wafer` namespace):**
- `wafer.call_block(name_ptr, name_len, msg_ptr, msg_len) -> i32` — call another block
- `wafer.read_result(dest_ptr, dest_len) -> i32` — read result back into guest memory
- `wafer.is_cancelled() -> i32` — check cancellation
- `wafer.log(level_ptr, level_len, msg_ptr, msg_len)` — logging

The WIT (Component Model) definitions in `wafer-run/wit/wit/` also apply.

### Implementation Tasks

- [ ] Create wasm-bindgen bindings to `WebAssembly.instantiate()` and `WebAssembly.Memory`
- [ ] Implement host imports as JS functions that bridge back into the Rust Worker
- [ ] Handle async `call_block` — guest traps, host resolves, guest resumes (match the resumable call pattern from wasmi loader)
- [ ] Load `.wasm` bytes from R2 on demand (cache instantiated modules in-memory for the Worker lifetime)
- [ ] Add capabilities/sandbox enforcement (reuse `BlockCapabilities` from wafer-run)
- [ ] Register custom blocks in `CloudflareContext.call_block()` dispatch
- [ ] Add tenant config for custom block mappings (block name → R2 path)

### Why not wasmi?

wasmi is a WASM interpreter written in Rust. It compiles to WASM itself, so it *could* run inside the Worker. But:
- Double interpretation: V8 interprets Worker WASM which interprets guest WASM
- Adds significant binary size to the Worker
- CPU time limits on Workers (10-50ms) would be consumed by interpreter overhead
- V8's native `WebAssembly.instantiate()` JIT-compiles guest blocks — much faster

### References

- Thin ABI: `wafer-run/crates/wafer-run/src/wasm/loader.rs`
- Capabilities: `wafer-run/crates/wafer-run/src/wasm/capabilities.rs`
- WIT definitions: `wafer-run/wit/wit/`
- TypeScript SDK: `wafer-run/packages/wafer-sdk-ts/`
